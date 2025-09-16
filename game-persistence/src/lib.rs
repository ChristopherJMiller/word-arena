pub mod connection;
pub mod entities;
pub mod repositories;

use sea_orm::{Database, DatabaseConnection, DbErr};

pub struct DatabaseManager {
    connection: DatabaseConnection,
}

impl DatabaseManager {
    pub async fn connect(database_url: &str) -> Result<Self, DbErr> {
        let connection = Database::connect(database_url).await?;
        Ok(Self { connection })
    }

    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }
}
