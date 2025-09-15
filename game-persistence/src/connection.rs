use sea_orm::{Database, DatabaseConnection, DbErr};

pub async fn connect_to_database() -> Result<DatabaseConnection, DbErr> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://word_arena.db".to_string());
    
    Database::connect(&database_url).await
}

pub async fn connect_to_memory_database() -> Result<DatabaseConnection, DbErr> {
    Database::connect("sqlite::memory:").await
}