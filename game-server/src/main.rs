use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::info;

use game_persistence::{connection::connect_and_migrate, repositories::UserRepository};
use game_server::{
    auth::AuthService, config::Config, create_routes, game_manager::GameManager,
    matchmaking::MatchmakingQueue, websocket::ConnectionManager,
};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Word Arena server...");

    // Initialize application state
    let config = Config::new();
    let connection_manager = Arc::new(ConnectionManager::new());

    // Initialize game manager with directory-based word loading
    let words_dir =
        std::env::var("WORDS_DIRECTORY").unwrap_or_else(|_| "./shared/words".to_string());
    info!("Loading words from directory: {}", words_dir);

    let game_manager = match GameManager::new(connection_manager.clone(), &words_dir) {
        Ok(gm) => {
            info!("Successfully loaded words from directory");
            Arc::new(gm)
        }
        Err(e) => {
            tracing::error!("Failed to load words from directory '{}': {}", words_dir, e);
            tracing::error!("The server requires word files to function.");
            tracing::error!("Run './scripts/download_and_split_words.sh' to generate word lists.");
            tracing::error!(
                "Or set WORDS_DIRECTORY to point to a directory containing .txt word files."
            );
            std::process::exit(1);
        }
    };

    let matchmaking_queue = Arc::new(MatchmakingQueue::new());

    // Initialize database connection and run migrations
    let db = match connect_and_migrate().await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!("Failed to connect to database and run migrations: {}", e);
            std::process::exit(1);
        }
    };
    let user_repository = Arc::new(UserRepository::new(db));

    // Check for dev mode
    let auth_service =
        if std::env::var("AUTH_DEV_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
            info!("Starting in development authentication mode - JWT validation disabled");
            Arc::new(AuthService::new_dev_mode())
        } else {
            Arc::new(AuthService::new(
                std::env::var("AZURE_TENANT_ID").unwrap_or_else(|_| "common".to_string()),
                std::env::var("AZURE_CLIENT_ID").unwrap_or_else(|_| "your-client-id".to_string()),
            ))
        };

    let routes = create_routes(
        connection_manager.clone(),
        game_manager.clone(),
        matchmaking_queue.clone(),
        auth_service,
        user_repository,
    );

    // Start cleanup task
    let cleanup_connection_manager = connection_manager.clone();
    let cleanup_game_manager = game_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let connection_timeout = Duration::from_secs(config.connection_timeout_seconds);
            let game_timeout = Duration::from_secs(config.game_timeout_minutes * 60);

            cleanup_connection_manager
                .cleanup_inactive_connections(connection_timeout)
                .await;
            cleanup_game_manager
                .cleanup_abandoned_games(game_timeout)
                .await;
        }
    });

    info!("Server starting on {}:{}", config.host, config.port);

    let addr = (
        config.host.parse::<std::net::IpAddr>().unwrap(),
        config.port,
    );

    let (addr, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async {
        // Wait for SIGINT (Ctrl+C) or SIGTERM
        #[cfg(unix)]
        {
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

            tokio::select! {
                _ = sigint.recv() => {
                    info!("Received SIGINT, shutting down gracefully...");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down gracefully...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
            info!("Received Ctrl+C, shutting down gracefully...");
        }
    });

    info!(
        "Server started successfully on {}. Press Ctrl+C to stop.",
        addr
    );
    server.await;
    info!("Server shutdown complete.");
}
