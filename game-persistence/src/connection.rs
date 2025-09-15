use sea_orm::{Database, DatabaseConnection, DbErr};
use migration::{Migrator, MigratorTrait};

pub async fn connect_to_database() -> Result<DatabaseConnection, DbErr> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://word_arena.db".to_string());
    
    Database::connect(&database_url).await
}

pub async fn connect_to_memory_database() -> Result<DatabaseConnection, DbErr> {
    Database::connect("sqlite::memory:").await
}

pub async fn connect_and_migrate() -> Result<DatabaseConnection, DbErr> {
    // Check if we're in dev mode - use in-memory database
    let is_dev_mode = std::env::var("AUTH_DEV_MODE").unwrap_or_else(|_| "false".to_string()) == "true";
    
    let db = if is_dev_mode {
        tracing::info!("Development mode: using in-memory SQLite database");
        connect_to_memory_database().await?
    } else {
        connect_to_database().await?
    };
    
    // Run migrations
    tracing::info!("Running database migrations...");
    match Migrator::up(&db, None).await {
        Ok(_) => {
            tracing::info!("Database migrations completed successfully");
        }
        Err(e) => {
            tracing::error!("Failed to run migrations: {}", e);
            return Err(e);
        }
    }
    
    Ok(db)
}