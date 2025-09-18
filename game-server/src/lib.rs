use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use warp::Filter;

use crate::auth::AuthService;
use crate::game_manager::GameManager;
use crate::matchmaking::MatchmakingQueue;
use crate::websocket::ConnectionManager;
use game_persistence::repositories::UserRepository;

#[derive(Deserialize)]
struct LeaderboardQuery {
    limit: Option<u64>,
}

#[derive(serde::Serialize)]
struct UserStatsResponse {
    user: game_types::User,
    rank: Option<u32>,
}

pub mod auth;
pub mod config;
pub mod game_manager;
pub mod matchmaking;
pub mod websocket;

pub fn create_routes(
    connection_manager: Arc<ConnectionManager>,
    game_manager: Arc<GameManager>,
    matchmaking_queue: Arc<MatchmakingQueue>,
    auth_service: Arc<AuthService>,
    user_repository: Arc<UserRepository>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Clone for filters
    let connection_manager_filter = warp::any().map({
        let connection_manager = connection_manager.clone();
        move || connection_manager.clone()
    });

    let game_manager_filter = warp::any().map({
        let game_manager = game_manager.clone();
        move || game_manager.clone()
    });

    let matchmaking_filter = warp::any().map({
        let matchmaking_queue = matchmaking_queue.clone();
        move || matchmaking_queue.clone()
    });

    let auth_filter = warp::any().map({
        let auth_service = auth_service.clone();
        move || auth_service.clone()
    });

    let user_repository_filter = warp::any().map({
        let user_repository = user_repository.clone();
        move || user_repository.clone()
    });

    // WebSocket endpoint
    let websocket = warp::path("ws")
        .and(warp::ws())
        .and(connection_manager_filter.clone())
        .and(game_manager_filter.clone())
        .and(matchmaking_filter.clone())
        .and(auth_filter.clone())
        .map(|ws: warp::ws::Ws, conn_mgr, game_mgr, queue, auth| {
            ws.on_upgrade(move |socket| {
                websocket::handle_connection(socket, conn_mgr, game_mgr, queue, auth)
            })
        });

    // Health check endpoint
    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

    // Game state endpoint - safe for reconnection
    let game_state = warp::path!("game" / String / "state")
        .and(warp::get())
        .and(warp::header::optional::<String>("authorization"))
        .and(game_manager_filter.clone())
        .and(auth_filter.clone())
        .and_then(handle_game_state_request);

    // Leaderboard endpoint
    let leaderboard = warp::path("leaderboard")
        .and(warp::get())
        .and(warp::query::<LeaderboardQuery>())
        .and(user_repository_filter.clone())
        .and_then(handle_leaderboard_request);

    // User stats endpoint
    let user_stats = warp::path!("user" / String / "stats")
        .and(warp::get())
        .and(warp::header::optional::<String>("authorization"))
        .and(user_repository_filter.clone())
        .and(auth_filter.clone())
        .and_then(handle_user_stats_request);

    // CORS configuration
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization"])
        .allow_methods(vec!["GET", "POST", "DELETE"]);

    websocket
        .or(health)
        .or(game_state)
        .or(leaderboard)
        .or(user_stats)
        .with(cors)
        .with(warp::log("word_arena"))
}

async fn handle_game_state_request(
    game_id: String,
    auth_header: Option<String>,
    game_manager: Arc<GameManager>,
    auth_service: Arc<AuthService>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Parse game ID as UUID
    let _game_uuid = match Uuid::parse_str(&game_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Invalid game ID format"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    // Check authentication if provided
    if let Some(auth_header) = auth_header {
        let token = auth_header.strip_prefix("Bearer ").unwrap_or(&auth_header);

        match auth_service.validate_token(token).await {
            Ok(user) => {
                // Check if user is in the game
                if !game_manager.is_user_in_game(&game_id, &user.id).await {
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({
                            "error": "Not authorized to view this game"
                        })),
                        warp::http::StatusCode::FORBIDDEN,
                    ));
                }
            }
            Err(_) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "error": "Invalid authentication token"
                    })),
                    warp::http::StatusCode::UNAUTHORIZED,
                ));
            }
        }
    }

    // Get safe game state
    match game_manager.get_safe_game_state(&game_id).await {
        Some(safe_state) => Ok(warp::reply::with_status(
            warp::reply::json(&safe_state),
            warp::http::StatusCode::OK,
        )),
        None => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Game not found"
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
    }
}

async fn handle_leaderboard_request(
    query: LeaderboardQuery,
    user_repository: Arc<UserRepository>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let limit = query.limit.unwrap_or(10).min(100); // Default 10, max 100

    match user_repository.get_leaderboard(limit).await {
        Ok(leaderboard) => Ok(warp::reply::with_status(
            warp::reply::json(&leaderboard),
            warp::http::StatusCode::OK,
        )),
        Err(err) => {
            tracing::error!("Failed to fetch leaderboard: {}", err);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Failed to fetch leaderboard"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn handle_user_stats_request(
    user_id: String,
    auth_header: Option<String>,
    user_repository: Arc<UserRepository>,
    auth_service: Arc<AuthService>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Parse user ID as UUID
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Invalid user ID format"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    // Check authentication if provided - user can only view their own stats unless admin
    if let Some(auth_header) = auth_header {
        let token = auth_header.strip_prefix("Bearer ").unwrap_or(&auth_header);

        match auth_service.validate_token(token).await {
            Ok(authenticated_user) => {
                // Only allow users to view their own stats
                if authenticated_user.id != user_uuid {
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({
                            "error": "Not authorized to view this user's stats"
                        })),
                        warp::http::StatusCode::FORBIDDEN,
                    ));
                }
            }
            Err(_) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "error": "Invalid authentication token"
                    })),
                    warp::http::StatusCode::UNAUTHORIZED,
                ));
            }
        }
    } else {
        // No auth header - public stats viewing not allowed
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Authentication required"
            })),
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    }

    // Get user and their rank
    match user_repository.find_by_id(user_uuid).await {
        Ok(Some(user)) => {
            let rank = match user_repository.get_user_rank(user_uuid).await {
                Ok(rank) => rank,
                Err(err) => {
                    tracing::error!("Failed to get user rank: {}", err);
                    None
                }
            };

            let response = UserStatsResponse { user, rank };
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "User not found"
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(err) => {
            tracing::error!("Failed to fetch user stats: {}", err);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Failed to fetch user stats"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use game_persistence::repositories::user_repository::LeaderboardEntry;
    use game_types::{ClientMessage, ServerMessage, User};
    use migration::{Migrator, MigratorTrait};
    use std::time::Duration;

    async fn create_test_app()
    -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let connection_manager = Arc::new(ConnectionManager::new());
        let game_manager =
            Arc::new(GameManager::new_with_default_words(connection_manager.clone()).unwrap());
        let matchmaking_queue = Arc::new(MatchmakingQueue::new());
        let auth_service = Arc::new(AuthService::new(
            "test-tenant".to_string(),
            "test-client".to_string(),
        ));

        // Create in-memory database for tests
        let db = game_persistence::connection::connect_to_memory_database()
            .await
            .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        let user_repository = Arc::new(UserRepository::new(db));

        create_routes(
            connection_manager,
            game_manager,
            matchmaking_queue,
            auth_service,
            user_repository,
        )
    }

    async fn create_dev_test_app()
    -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let connection_manager = Arc::new(ConnectionManager::new());

        // Use test words for predictable testing
        let word_validator = game_core::word_validation::WordValidator::new_with_test_words();
        let game_manager = Arc::new(GameManager::new_with_validator(
            connection_manager.clone(),
            word_validator,
        ));

        let matchmaking_queue = Arc::new(MatchmakingQueue::new());
        let auth_service = Arc::new(AuthService::new_dev_mode());

        // Create in-memory database for tests
        let db = game_persistence::connection::connect_to_memory_database()
            .await
            .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        let user_repository = Arc::new(UserRepository::new(db));

        create_routes(
            connection_manager,
            game_manager,
            matchmaking_queue,
            auth_service,
            user_repository,
        )
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_test_app().await;

        let response = warp::test::request()
            .method("GET")
            .path("/health")
            .reply(&app)
            .await;

        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), "OK");
    }

    #[tokio::test]
    async fn test_websocket_connection_upgrade() {
        let app = create_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Try to send a heartbeat to verify connection works
        let heartbeat_msg = ClientMessage::Heartbeat;
        let heartbeat_json = serde_json::to_string(&heartbeat_msg).expect("Should serialize");

        ws.send_text(heartbeat_json).await;

        // Heartbeat doesn't send a response, so if no error occurs, connection is working
    }

    #[tokio::test]
    async fn test_websocket_invalid_message_handling() {
        let app = create_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Send invalid JSON
        ws.send_text("invalid json").await;

        // Should either receive an error message or connection should close
        match ws.recv().await {
            Ok(msg) if msg.is_text() => {
                let text = msg.to_str().unwrap();
                let server_msg: ServerMessage =
                    serde_json::from_str(&text).expect("Should be valid ServerMessage");
                if let ServerMessage::Error { message } = server_msg {
                    assert!(message.contains("Invalid JSON message"));
                } else {
                    panic!("Expected error message, got: {:?}", server_msg);
                }
            }
            Err(_) => {
                // Connection closed due to invalid message - this is acceptable behavior
            }
            _ => panic!("Expected text message or connection closure"),
        }
    }

    #[tokio::test]
    async fn test_websocket_join_queue_message() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // First authenticate
        let auth_msg = ClientMessage::Authenticate {
            token: "user1:test@example.com:Test User".to_string(),
        };
        let auth_json = serde_json::to_string(&auth_msg).expect("Should serialize");
        ws.send_text(&auth_json).await;

        // Consume authentication success message
        let _auth_response = ws.recv().await.expect("Should receive auth response");

        // Send join queue message
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");
        ws.send_text(&join_json).await;

        // Should receive queue joined confirmation
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 1); // First player in queue
            } else {
                panic!("Expected QueueJoined message, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message or connection error");
        }
    }

    #[tokio::test]
    async fn test_websocket_multiple_clients_queue() {
        let app = create_dev_test_app().await;

        // Create two WebSocket connections
        let mut ws1 = warp::test::ws()
            .path("/ws")
            .handshake(app.clone())
            .await
            .expect("WebSocket handshake should succeed");

        let mut ws2 = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Authenticate both clients
        let auth_msg1 = ClientMessage::Authenticate {
            token: "user3:test3@example.com:Test User 3".to_string(),
        };
        let auth_json1 = serde_json::to_string(&auth_msg1).expect("Should serialize");
        ws1.send_text(&auth_json1).await;
        let _auth_response1 = ws1.recv().await.expect("Should receive auth response");

        let auth_msg2 = ClientMessage::Authenticate {
            token: "user4:test4@example.com:Test User 4".to_string(),
        };
        let auth_json2 = serde_json::to_string(&auth_msg2).expect("Should serialize");
        ws2.send_text(&auth_json2).await;
        let _auth_response2 = ws2.recv().await.expect("Should receive auth response");

        // Both clients join queue
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");

        ws1.send_text(&join_json).await;
        ws2.send_text(&join_json).await;

        // First client should get position 1
        let msg1 = ws1.recv().await.expect("Should receive response");
        if msg1.is_text() {
            let text = msg1.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 1);
            } else {
                panic!("Expected QueueJoined message, got: {:?}", server_msg);
            }
        }

        // Second client should get position 2
        let msg2 = ws2.recv().await.expect("Should receive response");
        if msg2.is_text() {
            let text = msg2.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 2);
            } else {
                panic!("Expected QueueJoined message, got: {:?}", server_msg);
            }
        }
    }

    #[tokio::test]
    async fn test_websocket_match_creation() {
        let app = create_test_app().await;

        // Create two WebSocket connections (minimum for a match)
        let mut ws1 = warp::test::ws()
            .path("/ws")
            .handshake(app.clone())
            .await
            .expect("WebSocket handshake should succeed");

        let mut ws2 = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Both clients join queue
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");

        ws1.send_text(&join_json).await;
        ws2.send_text(&join_json).await;

        // Consume queue joined messages
        let _msg1 = ws1.recv().await.expect("Should receive queue response");
        let _msg2 = ws2.recv().await.expect("Should receive queue response");

        // Both should eventually receive match found messages
        // Note: This test might be timing-dependent based on matchmaking logic
        tokio::time::sleep(Duration::from_millis(10)).await;

        // For this test, we'll just verify the setup completed without error
        // Testing actual match creation would require more complex timing coordination
    }

    #[tokio::test]
    async fn test_websocket_leave_queue() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // First authenticate
        let auth_msg = ClientMessage::Authenticate {
            token: "user2:test2@example.com:Test User 2".to_string(),
        };
        let auth_json = serde_json::to_string(&auth_msg).expect("Should serialize");
        ws.send_text(&auth_json).await;

        // Consume authentication success message
        let _auth_response = ws.recv().await.expect("Should receive auth response");

        // Join queue first
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");
        ws.send_text(&join_json).await;

        // Consume queue joined message
        let _msg = ws.recv().await.expect("Should receive queue response");

        // Leave queue
        let leave_msg = ClientMessage::LeaveQueue;
        let leave_json = serde_json::to_string(&leave_msg).expect("Should serialize");
        ws.send_text(&leave_json).await;

        // Should receive queue left confirmation
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueLeft = server_msg {
                // Success
            } else {
                panic!("Expected QueueLeft message, got: {:?}", server_msg);
            }
        }
    }

    #[tokio::test]
    async fn test_websocket_invalid_game_operations() {
        let app = create_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Try to submit guess without being in a game
        let guess_msg = ClientMessage::SubmitGuess {
            word: "HELLO".to_string(),
        };
        let guess_json = serde_json::to_string(&guess_msg).expect("Should serialize");
        ws.send_text(&guess_json).await;

        // Should either receive an error message or connection should close
        match ws.recv().await {
            Ok(msg) if msg.is_text() => {
                let text = msg.to_str().unwrap();
                let server_msg: ServerMessage =
                    serde_json::from_str(&text).expect("Should be valid ServerMessage");
                if let ServerMessage::Error { message } = server_msg {
                    assert!(
                        message.contains("Not in a game")
                            || message.contains("Connection not found")
                    );
                } else {
                    panic!("Expected error message, got: {:?}", server_msg);
                }
            }
            Err(_) => {
                // Connection closed due to invalid operation - this is acceptable behavior
            }
            _ => panic!("Expected text message or connection closure"),
        }
    }

    #[tokio::test]
    async fn test_websocket_heartbeat() {
        let app = create_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Send heartbeat
        let heartbeat_msg = ClientMessage::Heartbeat;
        let heartbeat_json = serde_json::to_string(&heartbeat_msg).expect("Should serialize");
        ws.send(warp::ws::Message::text(&heartbeat_json)).await;

        // Heartbeat should not generate a response, just update activity
        // We can't easily test activity update without exposing internal state
        // So we just verify no error occurs during send

        // Small delay to let the message be processed
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    #[tokio::test]
    async fn test_websocket_connection_close_cleanup() {
        let app = create_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Join queue
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");
        ws.send_text(&join_json).await;

        // Consume response
        let _msg = ws.recv().await.expect("Should receive queue response");

        // Drop the WebSocket to close the connection
        drop(ws);

        // The cleanup should happen automatically when the connection drops
        // We can't easily test this without exposing internal state
    }

    #[tokio::test]
    async fn test_http_endpoints_cors() {
        let app = create_test_app().await;

        // Test CORS preflight request
        let response = warp::test::request()
            .method("OPTIONS")
            .path("/health")
            .header("origin", "http://localhost:3000")
            .header("access-control-request-method", "GET")
            .reply(&app)
            .await;

        // Should allow CORS
        assert_eq!(response.status(), 200);

        // Check CORS headers are present
        let headers = response.headers();
        assert!(headers.contains_key("access-control-allow-origin"));
    }

    #[tokio::test]
    async fn test_invalid_routes() {
        let app = create_test_app().await;

        // Test invalid path
        let response = warp::test::request()
            .method("GET")
            .path("/invalid")
            .reply(&app)
            .await;

        assert_eq!(response.status(), 404);
    }

    // Authentication Integration Tests
    #[tokio::test]
    async fn test_auth_unauthenticated_queue_join_fails() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Try to join queue without authentication
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");
        ws.send_text(&join_json).await;

        // Should receive error message
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::Error { message } = server_msg {
                assert!(message.contains("Authentication required"));
            } else {
                panic!("Expected error message, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message");
        }
    }

    #[tokio::test]
    async fn test_auth_dev_mode_string_token() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Authenticate with simple string token
        let auth_msg = ClientMessage::Authenticate {
            token: "user1:alice@example.com:Alice".to_string(),
        };
        let auth_json = serde_json::to_string(&auth_msg).expect("Should serialize");
        ws.send_text(&auth_json).await;

        // Should receive authentication success
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::AuthenticationSuccess { user } = server_msg {
                assert_eq!(user.email, "alice@example.com");
                assert_eq!(user.display_name, "Alice");
            } else {
                panic!("Expected AuthenticationSuccess, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message");
        }
    }

    #[tokio::test]
    async fn test_auth_dev_mode_json_token() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Authenticate with JSON token
        let json_token = r#"{"user_id":"550e8400-e29b-41d4-a716-446655440000","email":"bob@example.com","name":"Bob"}"#;
        let auth_msg = ClientMessage::Authenticate {
            token: json_token.to_string(),
        };
        let auth_json = serde_json::to_string(&auth_msg).expect("Should serialize");
        ws.send_text(&auth_json).await;

        // Should receive authentication success
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::AuthenticationSuccess { user } = server_msg {
                assert_eq!(user.email, "bob@example.com");
                assert_eq!(user.display_name, "Bob");
                assert_eq!(user.id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
            } else {
                panic!("Expected AuthenticationSuccess, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message");
        }
    }

    #[tokio::test]
    async fn test_auth_invalid_token_format() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Try to authenticate with invalid token
        let auth_msg = ClientMessage::Authenticate {
            token: "invalid:token".to_string(),
        };
        let auth_json = serde_json::to_string(&auth_msg).expect("Should serialize");
        ws.send_text(&auth_json).await;

        // Should receive authentication failure
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::AuthenticationFailed { reason } = server_msg {
                assert!(reason.contains("Invalid token"));
            } else {
                panic!("Expected AuthenticationFailed, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message");
        }
    }

    #[tokio::test]
    async fn test_auth_successful_queue_join() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // First authenticate
        let auth_msg = ClientMessage::Authenticate {
            token: "user1:charlie@example.com:Charlie".to_string(),
        };
        let auth_json = serde_json::to_string(&auth_msg).expect("Should serialize");
        ws.send_text(&auth_json).await;

        // Consume authentication success message
        let _auth_response = ws.recv().await.expect("Should receive auth response");

        // Now try to join queue
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");
        ws.send_text(&join_json).await;

        // Should receive queue joined message
        let msg = ws.recv().await.expect("Should receive response");
        if msg.is_text() {
            let text = msg.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 1); // First player in queue
            } else {
                panic!("Expected QueueJoined, got: {:?}", server_msg);
            }
        } else {
            panic!("Expected text message");
        }
    }

    #[tokio::test]
    async fn test_auth_two_users_matchmaking() {
        let app = create_dev_test_app().await;

        // Create two WebSocket connections
        let mut ws1 = warp::test::ws()
            .path("/ws")
            .handshake(app.clone())
            .await
            .expect("WebSocket handshake should succeed");

        let mut ws2 = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Authenticate first user
        let auth_msg1 = ClientMessage::Authenticate {
            token: "user1:alice@example.com:Alice".to_string(),
        };
        ws1.send_text(&serde_json::to_string(&auth_msg1).unwrap())
            .await;
        let _auth1 = ws1.recv().await.expect("Should receive auth response");

        // Authenticate second user
        let auth_msg2 = ClientMessage::Authenticate {
            token: "user2:bob@example.com:Bob".to_string(),
        };
        ws2.send_text(&serde_json::to_string(&auth_msg2).unwrap())
            .await;
        let _auth2 = ws2.recv().await.expect("Should receive auth response");

        // Both users join queue
        let join_msg = ClientMessage::JoinQueue;
        let join_json = serde_json::to_string(&join_msg).expect("Should serialize");

        ws1.send_text(&join_json).await;
        ws2.send_text(&join_json).await;

        // First user should get position 1
        let msg1 = ws1.recv().await.expect("Should receive response");
        if msg1.is_text() {
            let text = msg1.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 1);
            } else {
                panic!("Expected QueueJoined, got: {:?}", server_msg);
            }
        }

        // Second user should get position 2
        let msg2 = ws2.recv().await.expect("Should receive response");
        if msg2.is_text() {
            let text = msg2.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 2);
            } else {
                panic!("Expected QueueJoined, got: {:?}", server_msg);
            }
        }

        // Both should eventually receive match found messages
        // Let's wait a bit and see if matchmaking creates a game
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to receive any additional messages (like match found)
        // This is optional since matchmaking might not trigger immediately
        if let Ok(msg) = tokio::time::timeout(Duration::from_millis(100), ws1.recv()).await {
            if let Ok(msg) = msg {
                if msg.is_text() {
                    let text = msg.to_str().unwrap();
                    let server_msg: ServerMessage =
                        serde_json::from_str(&text).expect("Should be valid ServerMessage");
                    println!("User 1 received additional message: {:?}", server_msg);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_queue_status_tracking() {
        let app = create_dev_test_app().await;

        let mut ws = warp::test::ws()
            .path("/ws")
            .handshake(app)
            .await
            .expect("WebSocket handshake should succeed");

        // Authenticate
        let auth_msg = ClientMessage::Authenticate {
            token: "user1:test@example.com:TestUser".to_string(),
        };
        ws.send_text(&serde_json::to_string(&auth_msg).unwrap())
            .await;

        // Consume auth success
        let auth_response = ws.recv().await.expect("Should receive auth response");
        if auth_response.is_text() {
            let text = auth_response.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            assert!(matches!(
                server_msg,
                ServerMessage::AuthenticationSuccess { .. }
            ));
        }

        // Join queue
        let join_msg = ClientMessage::JoinQueue;
        ws.send_text(&serde_json::to_string(&join_msg).unwrap())
            .await;

        // Verify queue joined response
        let queue_response = ws.recv().await.expect("Should receive queue response");
        if queue_response.is_text() {
            let text = queue_response.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            if let ServerMessage::QueueJoined { position } = server_msg {
                assert_eq!(position, 1);
                println!("✅ User successfully joined queue at position {}", position);
            } else {
                panic!("Expected QueueJoined, got: {:?}", server_msg);
            }
        }

        // Leave queue
        let leave_msg = ClientMessage::LeaveQueue;
        ws.send_text(&serde_json::to_string(&leave_msg).unwrap())
            .await;

        // Verify queue left response
        let leave_response = ws.recv().await.expect("Should receive leave response");
        if leave_response.is_text() {
            let text = leave_response.to_str().unwrap();
            let server_msg: ServerMessage =
                serde_json::from_str(&text).expect("Should be valid ServerMessage");
            assert!(matches!(server_msg, ServerMessage::QueueLeft));
            println!("✅ User successfully left queue");
        }
    }

    // Helper function to create test users in the database
    async fn create_test_users(app: &warp::filters::BoxedFilter<(impl warp::Reply,)>) -> Vec<User> {
        // Access the user repository to create test data
        // Since we can't easily extract it from the app, we'll use HTTP requests to create users
        let users = vec![
            User {
                id: uuid::Uuid::new_v4(),
                email: "alice@test.com".to_string(),
                display_name: "Alice".to_string(),
                total_points: 100,
                total_wins: 5,
                total_games: 10,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            User {
                id: uuid::Uuid::new_v4(),
                email: "bob@test.com".to_string(),
                display_name: "Bob".to_string(),
                total_points: 200,
                total_wins: 8,
                total_games: 12,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            User {
                id: uuid::Uuid::new_v4(),
                email: "charlie@test.com".to_string(),
                display_name: "Charlie".to_string(),
                total_points: 50,
                total_wins: 2,
                total_games: 8,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        ];

        // We would need to populate the database through the repository directly
        // Since we can't access it from the test app, we'll create a fresh DB connection
        let db = game_persistence::connection::connect_to_memory_database()
            .await
            .unwrap();
        Migrator::up(&db, None).await.unwrap();
        let repo = UserRepository::new(db);

        for user in &users {
            repo.create_user(user.clone()).await.unwrap();
        }

        users
    }

    #[tokio::test]
    async fn test_leaderboard_endpoint_empty() {
        let app = create_dev_test_app().await;

        let response = warp::test::request()
            .method("GET")
            .path("/leaderboard")
            .reply(&app)
            .await;

        assert_eq!(response.status(), 200);

        let leaderboard: Vec<LeaderboardEntry> =
            serde_json::from_slice(response.body()).expect("Should parse JSON");

        assert_eq!(leaderboard.len(), 0);
    }

    #[tokio::test]
    async fn test_leaderboard_endpoint_with_limit() {
        let app = create_dev_test_app().await;

        let response = warp::test::request()
            .method("GET")
            .path("/leaderboard?limit=2")
            .reply(&app)
            .await;

        assert_eq!(response.status(), 200);

        let leaderboard: Vec<LeaderboardEntry> =
            serde_json::from_slice(response.body()).expect("Should parse JSON");

        // Should respect the limit (even if empty)
        assert!(leaderboard.len() <= 2);
    }

    #[tokio::test]
    async fn test_leaderboard_endpoint_with_invalid_limit() {
        let app = create_dev_test_app().await;

        // Test with very high limit - should be capped at 100
        let response = warp::test::request()
            .method("GET")
            .path("/leaderboard?limit=1000")
            .reply(&app)
            .await;

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_user_stats_endpoint_unauthorized() {
        let app = create_dev_test_app().await;
        let user_id = uuid::Uuid::new_v4();

        let response = warp::test::request()
            .method("GET")
            .path(&format!("/user/{}/stats", user_id))
            .reply(&app)
            .await;

        assert_eq!(response.status(), 401);

        let error: serde_json::Value =
            serde_json::from_slice(response.body()).expect("Should parse JSON");

        assert_eq!(error["error"], "Authentication required");
    }

    #[tokio::test]
    async fn test_user_stats_endpoint_invalid_user_id() {
        let app = create_dev_test_app().await;

        let response = warp::test::request()
            .method("GET")
            .path("/user/invalid-uuid/stats")
            .header("authorization", "user1:test@example.com:Test")
            .reply(&app)
            .await;

        assert_eq!(response.status(), 400);

        let error: serde_json::Value =
            serde_json::from_slice(response.body()).expect("Should parse JSON");

        assert_eq!(error["error"], "Invalid user ID format");
    }

    #[tokio::test]
    async fn test_user_stats_endpoint_forbidden() {
        let app = create_dev_test_app().await;
        let user_id = uuid::Uuid::new_v4();
        let different_user_id = uuid::Uuid::new_v4();

        // Try to access another user's stats
        let response = warp::test::request()
            .method("GET")
            .path(&format!("/user/{}/stats", different_user_id))
            .header(
                "authorization",
                &format!("{}:test@example.com:Test", user_id),
            )
            .reply(&app)
            .await;

        assert_eq!(response.status(), 403);

        let error: serde_json::Value =
            serde_json::from_slice(response.body()).expect("Should parse JSON");

        assert_eq!(error["error"], "Not authorized to view this user's stats");
    }

    #[tokio::test]
    async fn test_user_stats_endpoint_user_not_found() {
        let app = create_dev_test_app().await;
        let user_id = uuid::Uuid::new_v4();

        // Request own stats for a user that doesn't exist in DB
        let response = warp::test::request()
            .method("GET")
            .path(&format!("/user/{}/stats", user_id))
            .header(
                "authorization",
                &format!("{}:test@example.com:Test", user_id),
            )
            .reply(&app)
            .await;

        assert_eq!(response.status(), 404);

        let error: serde_json::Value =
            serde_json::from_slice(response.body()).expect("Should parse JSON");

        assert_eq!(error["error"], "User not found");
    }
}
