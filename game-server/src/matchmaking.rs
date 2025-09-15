use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::websocket::connection::ConnectionId;

#[derive(Debug, Clone)]
pub struct QueuedPlayer {
    pub connection_id: ConnectionId,
    pub queued_at: Instant,
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub players: Vec<ConnectionId>,
    pub created_at: Instant,
}

pub struct MatchmakingQueue {
    queue: RwLock<VecDeque<QueuedPlayer>>,
    player_positions: RwLock<HashMap<ConnectionId, usize>>,
    min_players: usize,
    max_players: usize,
    queue_timeout: Duration,
}

impl MatchmakingQueue {
    pub fn new() -> Self {
        Self::new_with_config(2, 16, Duration::from_secs(300))
    }

    pub fn new_with_config(
        min_players: usize,
        max_players: usize,
        queue_timeout: Duration,
    ) -> Self {
        Self {
            queue: RwLock::new(VecDeque::new()),
            player_positions: RwLock::new(HashMap::new()),
            min_players,
            max_players,
            queue_timeout,
        }
    }

    pub async fn add_player(&self, connection_id: ConnectionId) -> Result<u32, String> {
        let mut queue = self.queue.write().await;
        let mut positions = self.player_positions.write().await;

        // Check if player is already in queue
        if positions.contains_key(&connection_id) {
            return Err("Player already in queue".to_string());
        }

        let player = QueuedPlayer {
            connection_id,
            queued_at: Instant::now(),
        };

        queue.push_back(player);
        positions.insert(connection_id, queue.len() - 1);

        let position = queue.len() as u32;
        info!(
            "Player {} added to queue at position {}",
            connection_id, position
        );

        Ok(position)
    }

    pub async fn remove_player(&self, connection_id: ConnectionId) -> Result<(), String> {
        let mut queue = self.queue.write().await;
        let mut positions = self.player_positions.write().await;

        if let Some(_position) = positions.remove(&connection_id) {
            // Find and remove the player from the queue
            if let Some(index) = queue.iter().position(|p| p.connection_id == connection_id) {
                queue.remove(index);

                // Update positions for remaining players
                positions.clear();
                for (i, player) in queue.iter().enumerate() {
                    positions.insert(player.connection_id, i);
                }

                info!("Player {} removed from queue", connection_id);
                Ok(())
            } else {
                warn!("Player {} was in positions but not in queue", connection_id);
                Err("Player not found in queue".to_string())
            }
        } else {
            Err("Player not in queue".to_string())
        }
    }

    pub async fn try_create_match(&self) -> Result<Option<MatchInfo>, String> {
        let mut queue = self.queue.write().await;
        let mut positions = self.player_positions.write().await;

        // Check if we have enough players for a match
        if queue.len() < self.min_players {
            return Ok(None);
        }

        // Determine how many players to take (up to max_players)
        let players_to_take = queue.len().min(self.max_players);

        // Take players from the front of the queue
        let mut match_players = Vec::with_capacity(players_to_take);
        for _ in 0..players_to_take {
            if let Some(player) = queue.pop_front() {
                match_players.push(player.connection_id);
                positions.remove(&player.connection_id);
            }
        }

        // Update positions for remaining players
        positions.clear();
        for (i, player) in queue.iter().enumerate() {
            positions.insert(player.connection_id, i);
        }

        if !match_players.is_empty() {
            let match_info = MatchInfo {
                players: match_players,
                created_at: Instant::now(),
            };

            info!("Created match with {} players", match_info.players.len());
            Ok(Some(match_info))
        } else {
            Ok(None)
        }
    }

    pub async fn get_queue_position(&self, connection_id: ConnectionId) -> Option<u32> {
        let positions = self.player_positions.read().await;
        positions.get(&connection_id).map(|&pos| (pos + 1) as u32)
    }

    pub async fn get_queue_length(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    pub async fn cleanup_expired_players(&self) {
        let mut queue = self.queue.write().await;
        let mut positions = self.player_positions.write().await;

        let now = Instant::now();
        let expired_players: Vec<ConnectionId> = queue
            .iter()
            .filter(|player| now.duration_since(player.queued_at) > self.queue_timeout)
            .map(|player| player.connection_id)
            .collect();

        for connection_id in expired_players {
            if let Some(index) = queue.iter().position(|p| p.connection_id == connection_id) {
                queue.remove(index);
                positions.remove(&connection_id);
                warn!("Removed expired player {} from queue", connection_id);
            }
        }

        // Update positions for remaining players
        positions.clear();
        for (i, player) in queue.iter().enumerate() {
            positions.insert(player.connection_id, i);
        }
    }

    pub async fn is_player_in_queue(&self, connection_id: ConnectionId) -> bool {
        let positions = self.player_positions.read().await;
        positions.contains_key(&connection_id)
    }

    pub async fn get_queue_stats(&self) -> QueueStats {
        let queue = self.queue.read().await;
        let now = Instant::now();

        let average_wait_time = if queue.is_empty() {
            Duration::ZERO
        } else {
            let total_wait: Duration = queue
                .iter()
                .map(|player| now.duration_since(player.queued_at))
                .sum();
            total_wait / queue.len() as u32
        };

        QueueStats {
            total_players: queue.len(),
            average_wait_time,
            min_players_needed: self.min_players,
            max_players_per_match: self.max_players,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_players: usize,
    pub average_wait_time: Duration,
    pub min_players_needed: usize,
    pub max_players_per_match: usize,
}

impl Default for MatchmakingQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::connection::ConnectionId;

    #[tokio::test]
    async fn test_basic_queue_operations() {
        let queue = MatchmakingQueue::new();
        let conn_id = ConnectionId::new();

        // Add player to queue
        let position = queue.add_player(conn_id).await.unwrap();
        assert_eq!(position, 1);
        assert_eq!(queue.get_queue_length().await, 1);
        assert!(queue.is_player_in_queue(conn_id).await);

        // Remove player from queue
        queue.remove_player(conn_id).await.unwrap();
        assert_eq!(queue.get_queue_length().await, 0);
        assert!(!queue.is_player_in_queue(conn_id).await);
    }

    #[tokio::test]
    async fn test_prevent_duplicate_queue_entries() {
        let queue = MatchmakingQueue::new();
        let conn_id = ConnectionId::new();

        // First add should succeed
        let result1 = queue.add_player(conn_id).await;
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 1);

        // Second add should fail
        let result2 = queue.add_player(conn_id).await;
        assert!(result2.is_err());
        assert_eq!(result2.unwrap_err(), "Player already in queue");

        // Queue should still have only one player
        assert_eq!(queue.get_queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_queue_position_consistency() {
        let queue = MatchmakingQueue::new();
        let mut players = Vec::new();

        // Add 5 players
        for i in 0..5 {
            let conn_id = ConnectionId::new();
            let position = queue.add_player(conn_id).await.unwrap();
            assert_eq!(position, (i + 1) as u32);
            players.push(conn_id);
        }

        // Verify positions are correct
        for (i, &conn_id) in players.iter().enumerate() {
            let position = queue.get_queue_position(conn_id).await.unwrap();
            assert_eq!(position, (i + 1) as u32);
        }

        // Remove middle player (index 2, position 3)
        queue.remove_player(players[2]).await.unwrap();

        // Verify positions are updated correctly
        assert_eq!(queue.get_queue_position(players[0]).await.unwrap(), 1);
        assert_eq!(queue.get_queue_position(players[1]).await.unwrap(), 2);
        assert_eq!(queue.get_queue_position(players[3]).await.unwrap(), 3); // Was 4, now 3
        assert_eq!(queue.get_queue_position(players[4]).await.unwrap(), 4); // Was 5, now 4
        assert!(queue.get_queue_position(players[2]).await.is_none()); // Removed player
    }

    #[tokio::test]
    async fn test_match_creation_with_minimum_players() {
        let queue = MatchmakingQueue::new_with_config(2, 4, Duration::from_secs(300));
        let mut players = Vec::new();

        // Add first player - no match should be created
        let conn_id1 = ConnectionId::new();
        queue.add_player(conn_id1).await.unwrap();
        players.push(conn_id1);

        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_none());
        assert_eq!(queue.get_queue_length().await, 1);

        // Add second player - match should be created
        let conn_id2 = ConnectionId::new();
        queue.add_player(conn_id2).await.unwrap();
        players.push(conn_id2);

        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_some());

        let match_info = match_result.unwrap();
        assert_eq!(match_info.players.len(), 2);
        assert!(match_info.players.contains(&conn_id1));
        assert!(match_info.players.contains(&conn_id2));

        // Queue should be empty after match creation
        assert_eq!(queue.get_queue_length().await, 0);
    }

    #[tokio::test]
    async fn test_match_creation_respects_max_players() {
        let queue = MatchmakingQueue::new_with_config(2, 3, Duration::from_secs(300));
        let mut players = Vec::new();

        // Add 5 players
        for _ in 0..5 {
            let conn_id = ConnectionId::new();
            queue.add_player(conn_id).await.unwrap();
            players.push(conn_id);
        }

        // Try to create match - should take only 3 players (max)
        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_some());

        let match_info = match_result.unwrap();
        assert_eq!(match_info.players.len(), 3);

        // Queue should have 2 remaining players
        assert_eq!(queue.get_queue_length().await, 2);

        // Remaining players should have correct positions
        let remaining_players: Vec<ConnectionId> = players.into_iter().skip(3).collect();
        assert_eq!(
            queue
                .get_queue_position(remaining_players[0])
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            queue
                .get_queue_position(remaining_players[1])
                .await
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn test_remove_nonexistent_player() {
        let queue = MatchmakingQueue::new();
        let conn_id = ConnectionId::new();

        let result = queue.remove_player(conn_id).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player not in queue");
    }

    #[tokio::test]
    async fn test_rapid_queue_operations() {
        let queue = MatchmakingQueue::new();
        let mut players = Vec::new();

        // Rapidly add 100 players
        for _ in 0..100 {
            let conn_id = ConnectionId::new();
            queue.add_player(conn_id).await.unwrap();
            players.push(conn_id);
        }

        assert_eq!(queue.get_queue_length().await, 100);

        // Rapidly remove every other player
        for (i, &conn_id) in players.iter().enumerate() {
            if i % 2 == 0 {
                queue.remove_player(conn_id).await.unwrap();
            }
        }

        assert_eq!(queue.get_queue_length().await, 50);

        // Verify remaining players have correct positions
        let mut expected_position = 1;
        for (i, &conn_id) in players.iter().enumerate() {
            if i % 2 == 1 {
                // Players that weren't removed
                let position = queue.get_queue_position(conn_id).await.unwrap();
                assert_eq!(position, expected_position);
                expected_position += 1;
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_queue_operations() {
        let queue = std::sync::Arc::new(MatchmakingQueue::new());
        let mut handles = Vec::new();
        let all_players = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

        // Spawn 20 concurrent tasks adding players
        for _i in 0..20 {
            let queue_clone = queue.clone();
            let players_clone = all_players.clone();
            let handle = tokio::spawn(async move {
                let conn_id = ConnectionId::new();

                // Add to shared player list
                {
                    let mut players = players_clone.lock().await;
                    players.push(conn_id);
                }

                // Add to queue
                let _position = queue_clone.add_player(conn_id).await.unwrap();

                // Simulate some work
                tokio::time::sleep(Duration::from_millis(1)).await;
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All players should be in queue
        assert_eq!(queue.get_queue_length().await, 20);

        // Verify all players have valid positions
        let players = all_players.lock().await;
        for &conn_id in players.iter() {
            let position = queue.get_queue_position(conn_id).await;
            assert!(position.is_some());
            assert!(position.unwrap() >= 1 && position.unwrap() <= 20);
        }
    }

    #[tokio::test]
    async fn test_race_condition_during_match_creation() {
        let queue = std::sync::Arc::new(MatchmakingQueue::new_with_config(
            2,
            4,
            Duration::from_secs(300),
        ));

        // Add 3 players to queue
        let mut players = Vec::new();
        for _ in 0..3 {
            let conn_id = ConnectionId::new();
            queue.add_player(conn_id).await.unwrap();
            players.push(conn_id);
        }

        // Simulate race condition: try to create match and remove player simultaneously
        let queue_clone1 = queue.clone();
        let queue_clone2 = queue.clone();
        let player_to_remove = players[1];

        let match_task = tokio::spawn(async move { queue_clone1.try_create_match().await });

        let remove_task =
            tokio::spawn(async move { queue_clone2.remove_player(player_to_remove).await });

        let (match_result, _remove_result) = tokio::join!(match_task, remove_task);

        // One of the operations should succeed, but state should be consistent
        let final_queue_length = queue.get_queue_length().await;

        if let Ok(Some(match_info)) = match_result.unwrap() {
            // Match was created successfully
            assert!(match_info.players.len() >= 2);
            // Queue should have remaining players (if any)
            assert!(final_queue_length <= 1);
        } else {
            // Match creation failed or no match created
            // Should have 2 or 3 players depending on whether remove succeeded
            assert!(final_queue_length >= 2);
        }

        // Remove operation may succeed or fail depending on timing
        // The important thing is that the final state is consistent
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let queue = MatchmakingQueue::new_with_config(3, 8, Duration::from_secs(300));

        // Add some players
        for _ in 0..5 {
            let conn_id = ConnectionId::new();
            queue.add_player(conn_id).await.unwrap();
        }

        let stats = queue.get_queue_stats().await;
        assert_eq!(stats.total_players, 5);
        assert_eq!(stats.min_players_needed, 3);
        assert_eq!(stats.max_players_per_match, 8);
        assert!(stats.average_wait_time.as_millis() >= 0);
    }

    #[tokio::test]
    async fn test_cleanup_expired_players() {
        let short_timeout = Duration::from_millis(10);
        let queue = MatchmakingQueue::new_with_config(2, 4, short_timeout);

        // Add players
        let mut players = Vec::new();
        for _ in 0..3 {
            let conn_id = ConnectionId::new();
            queue.add_player(conn_id).await.unwrap();
            players.push(conn_id);
        }

        assert_eq!(queue.get_queue_length().await, 3);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Cleanup expired players
        queue.cleanup_expired_players().await;

        // All players should be removed due to timeout
        assert_eq!(queue.get_queue_length().await, 0);

        // Verify players are no longer in queue
        for conn_id in players {
            assert!(!queue.is_player_in_queue(conn_id).await);
        }
    }

    #[tokio::test]
    async fn test_edge_case_empty_queue_operations() {
        let queue = MatchmakingQueue::new();

        // Try to create match with empty queue
        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_none());

        // Try to get stats with empty queue
        let stats = queue.get_queue_stats().await;
        assert_eq!(stats.total_players, 0);
        assert_eq!(stats.average_wait_time, Duration::ZERO);

        // Cleanup empty queue should not panic
        queue.cleanup_expired_players().await;
    }
}
