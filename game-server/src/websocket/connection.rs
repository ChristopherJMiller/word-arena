use game_types::{ServerMessage, User};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub user_id: Option<String>,
    pub user: Option<User>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub is_authenticated: bool,
    pub game_id: Option<String>,
    pub sender: mpsc::UnboundedSender<ServerMessage>,
}

impl Connection {
    pub fn new(id: ConnectionId) -> (Self, mpsc::UnboundedReceiver<ServerMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let now = Instant::now();

        let connection = Self {
            id,
            user_id: None,
            user: None,
            connected_at: now,
            last_activity: now,
            is_authenticated: false,
            game_id: None,
            sender,
        };

        (connection, receiver)
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn set_authenticated(&mut self, user_id: String) {
        self.user_id = Some(user_id);
        self.is_authenticated = true;
    }

    pub fn set_user(&mut self, user: User) {
        self.user_id = Some(user.id.to_string());
        self.user = Some(user);
        self.is_authenticated = true;
    }

    pub fn clear_user(&mut self) {
        self.user_id = None;
        self.user = None;
        self.is_authenticated = false;
    }

    pub fn set_game(&mut self, game_id: Option<String>) {
        self.game_id = game_id;
    }

    pub fn send_message(&self, message: ServerMessage) -> Result<(), String> {
        self.sender
            .send(message)
            .map_err(|_| "Connection closed".to_string())
    }

    pub fn is_inactive(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }
}

pub struct ConnectionManager {
    connections: RwLock<HashMap<ConnectionId, Connection>>,
    user_to_connection: RwLock<HashMap<String, ConnectionId>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            user_to_connection: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_connection(
        &self,
        id: ConnectionId,
    ) -> mpsc::UnboundedReceiver<ServerMessage> {
        let (conn, receiver) = Connection::new(id);

        {
            let mut connections = self.connections.write().await;
            connections.insert(id, conn);
        }

        receiver
    }

    pub async fn remove_connection(&self, id: ConnectionId) {
        let user_id = {
            let mut connections = self.connections.write().await;
            connections.remove(&id).and_then(|conn| conn.user_id)
        };

        if let Some(user_id) = user_id {
            let mut user_to_connection = self.user_to_connection.write().await;
            user_to_connection.remove(&user_id);
        }
    }

    pub async fn get_connection(&self, id: ConnectionId) -> Option<Connection> {
        let connections = self.connections.read().await;
        connections.get(&id).cloned()
    }

    pub async fn get_connection_by_user(&self, user_id: &str) -> Option<Connection> {
        let user_to_connection = self.user_to_connection.read().await;
        if let Some(connection_id) = user_to_connection.get(user_id) {
            let connections = self.connections.read().await;
            connections.get(connection_id).cloned()
        } else {
            None
        }
    }

    pub async fn authenticate_connection(
        &self,
        id: ConnectionId,
        user_id: String,
    ) -> Result<(), String> {
        // Check if user is already connected from another connection
        {
            let user_to_connection = self.user_to_connection.read().await;
            if user_to_connection.contains_key(&user_id) {
                return Err("User already connected".to_string());
            }
        }

        {
            let mut connections = self.connections.write().await;
            if let Some(connection) = connections.get_mut(&id) {
                connection.set_authenticated(user_id.clone());
            } else {
                return Err("Connection not found".to_string());
            }
        }

        {
            let mut user_to_connection = self.user_to_connection.write().await;
            user_to_connection.insert(user_id, id);
        }

        Ok(())
    }

    pub async fn update_activity(&self, id: ConnectionId) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(&id) {
            connection.update_activity();
        }
    }

    pub async fn send_to_connection(
        &self,
        id: ConnectionId,
        message: ServerMessage,
    ) -> Result<(), String> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&id) {
            connection.send_message(message)
        } else {
            Err("Connection not found".to_string())
        }
    }

    pub async fn send_to_user(&self, user_id: &str, message: ServerMessage) -> Result<(), String> {
        let connection_id = {
            let user_to_connection = self.user_to_connection.read().await;
            user_to_connection.get(user_id).copied()
        };

        if let Some(connection_id) = connection_id {
            self.send_to_connection(connection_id, message).await
        } else {
            Err("User not connected".to_string())
        }
    }

    pub async fn send_to_game(&self, game_id: &str, message: ServerMessage) {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            if let Some(ref conn_game_id) = connection.game_id {
                if conn_game_id == game_id {
                    let _ = connection.send_message(message.clone());
                }
            }
        }
    }

    pub async fn send_to_game_except(
        &self,
        game_id: &str,
        except_connection: ConnectionId,
        message: ServerMessage,
    ) {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            if connection.id != except_connection {
                if let Some(ref conn_game_id) = connection.game_id {
                    if conn_game_id == game_id {
                        let _ = connection.send_message(message.clone());
                    }
                }
            }
        }
    }

    pub async fn cleanup_inactive_connections(&self, timeout: Duration) {
        let inactive_connections: Vec<ConnectionId> = {
            let connections = self.connections.read().await;
            connections
                .values()
                .filter(|conn| conn.is_inactive(timeout))
                .map(|conn| conn.id)
                .collect()
        };

        for connection_id in inactive_connections {
            tracing::info!("Removing inactive connection: {}", connection_id);
            self.remove_connection(connection_id).await;
        }
    }

    pub async fn set_connection_game(&self, id: ConnectionId, game_id: Option<String>) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(&id) {
            connection.set_game(game_id);
        }
    }

    pub async fn get_connections_in_game(&self, game_id: &str) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        connections
            .values()
            .filter(|conn| {
                if let Some(ref conn_game_id) = conn.game_id {
                    conn_game_id == game_id
                } else {
                    false
                }
            })
            .map(|conn| conn.id)
            .collect()
    }

    pub async fn set_connection_user(&self, id: ConnectionId, user: Option<User>) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(&id) {
            if let Some(user) = user {
                connection.set_user(user);
            }
        }
    }

    // Test helper methods
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    pub async fn user_connection_count(&self) -> usize {
        let user_connections = self.user_to_connection.read().await;
        user_connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_connection_creation_and_removal() {
        let manager = ConnectionManager::new();
        let conn_id = ConnectionId::new();

        // Create connection
        let _receiver = manager.create_connection(conn_id).await;
        assert_eq!(manager.connection_count().await, 1);

        // Remove connection
        manager.remove_connection(conn_id).await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_rapid_connect_disconnect_cycles() {
        let manager = ConnectionManager::new();
        let mut connections = Vec::new();

        // Rapidly create 100 connections
        for _ in 0..100 {
            let conn_id = ConnectionId::new();
            let _receiver = manager.create_connection(conn_id).await;
            connections.push(conn_id);
        }

        assert_eq!(manager.connection_count().await, 100);

        // Rapidly remove all connections
        for conn_id in connections {
            manager.remove_connection(conn_id).await;
        }

        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_authentication_prevents_duplicate_users() {
        let manager = ConnectionManager::new();
        let conn_id1 = ConnectionId::new();
        let conn_id2 = ConnectionId::new();

        let _receiver1 = manager.create_connection(conn_id1).await;
        let _receiver2 = manager.create_connection(conn_id2).await;

        // First authentication should succeed
        let result1 = manager
            .authenticate_connection(conn_id1, "user1".to_string())
            .await;
        assert!(result1.is_ok());

        // Second authentication with same user should fail
        let result2 = manager
            .authenticate_connection(conn_id2, "user1".to_string())
            .await;
        assert!(result2.is_err());
        assert_eq!(result2.unwrap_err(), "User already connected");

        // Verify user mapping is consistent
        assert_eq!(manager.user_connection_count().await, 1);
    }

    #[tokio::test]
    async fn test_authentication_cleanup_on_disconnect() {
        let manager = ConnectionManager::new();
        let conn_id = ConnectionId::new();

        let _receiver = manager.create_connection(conn_id).await;
        manager
            .authenticate_connection(conn_id, "user1".to_string())
            .await
            .unwrap();

        assert_eq!(manager.user_connection_count().await, 1);

        // Remove connection should clean up user mapping
        manager.remove_connection(conn_id).await;
        assert_eq!(manager.connection_count().await, 0);
        assert_eq!(manager.user_connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_activity_tracking_and_timeout() {
        let manager = ConnectionManager::new();
        let conn_id = ConnectionId::new();

        let _receiver = manager.create_connection(conn_id).await;

        // Connection should not be inactive immediately
        let short_timeout = Duration::from_millis(10);
        manager.cleanup_inactive_connections(short_timeout).await;
        assert_eq!(manager.connection_count().await, 1);

        // Wait longer than timeout and verify cleanup
        tokio::time::sleep(Duration::from_millis(20)).await;
        manager.cleanup_inactive_connections(short_timeout).await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_message_sending_to_nonexistent_connection() {
        let manager = ConnectionManager::new();
        let conn_id = ConnectionId::new();

        let result = manager
            .send_to_connection(
                conn_id,
                game_types::ServerMessage::Error {
                    message: "test".to_string(),
                },
            )
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Connection not found");
    }

    #[tokio::test]
    async fn test_message_sending_after_connection_close() {
        let manager = ConnectionManager::new();
        let conn_id = ConnectionId::new();

        let receiver = manager.create_connection(conn_id).await;
        drop(receiver); // Close the receiver to simulate connection close

        let result = manager
            .send_to_connection(
                conn_id,
                game_types::ServerMessage::Error {
                    message: "test".to_string(),
                },
            )
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Connection closed");
    }

    #[tokio::test]
    async fn test_game_assignment_and_messaging() {
        let manager = ConnectionManager::new();
        let conn_id1 = ConnectionId::new();
        let conn_id2 = ConnectionId::new();
        let game_id = "test_game";

        let mut receiver1 = manager.create_connection(conn_id1).await;
        let mut receiver2 = manager.create_connection(conn_id2).await;

        // Assign both connections to same game
        manager
            .set_connection_game(conn_id1, Some(game_id.to_string()))
            .await;
        manager
            .set_connection_game(conn_id2, Some(game_id.to_string()))
            .await;

        // Send message to game
        let test_message = game_types::ServerMessage::Error {
            message: "game_message".to_string(),
        };
        manager.send_to_game(game_id, test_message).await;

        // Both connections should receive the message
        let msg1 = receiver1.try_recv();
        let msg2 = receiver2.try_recv();

        assert!(msg1.is_ok());
        assert!(msg2.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_connection_operations() {
        let manager = std::sync::Arc::new(ConnectionManager::new());
        let mut handles = Vec::new();

        // Spawn 50 concurrent tasks creating and removing connections
        for i in 0..50 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                let conn_id = ConnectionId::new();
                let _receiver = manager_clone.create_connection(conn_id).await;

                // Simulate some work
                tokio::time::sleep(Duration::from_millis(1)).await;

                manager_clone
                    .authenticate_connection(conn_id, format!("user_{}", i))
                    .await
                    .unwrap();
                manager_clone.remove_connection(conn_id).await;
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All connections should be cleaned up
        assert_eq!(manager.connection_count().await, 0);
        assert_eq!(manager.user_connection_count().await, 0);
    }
}
