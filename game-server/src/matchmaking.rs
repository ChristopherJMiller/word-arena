use std::collections::{HashMap, HashSet, VecDeque};
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

#[derive(Debug, Clone)]
pub struct CountdownInfo {
    pub seconds_remaining: u32,
    pub players_ready: u32,
    pub total_players: u32,
}

pub struct MatchmakingQueue {
    queue: RwLock<VecDeque<QueuedPlayer>>,
    player_positions: RwLock<HashMap<ConnectionId, usize>>,
    min_players: usize,
    max_players: usize,
    queue_timeout: Duration,
    // Countdown timer fields
    countdown_started_at: RwLock<Option<Instant>>,
    countdown_duration: Duration,
    votes_to_start: RwLock<HashSet<ConnectionId>>,
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
            countdown_started_at: RwLock::new(None),
            countdown_duration: Duration::from_secs(60), // 60 second countdown
            votes_to_start: RwLock::new(HashSet::new()),
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
        
        // Start countdown if we reach minimum players for the first time
        if queue.len() == self.min_players {
            let mut countdown_started = self.countdown_started_at.write().await;
            if countdown_started.is_none() {
                *countdown_started = Some(Instant::now());
                info!("Countdown started: {} players in queue", queue.len());
            }
        }

        info!(
            "Player {} added to queue at position {}",
            connection_id, position
        );

        Ok(position)
    }

    pub async fn remove_player(&self, connection_id: ConnectionId) -> Result<(), String> {
        let mut queue = self.queue.write().await;
        let mut positions = self.player_positions.write().await;
        let mut votes = self.votes_to_start.write().await;

        if let Some(_position) = positions.remove(&connection_id) {
            // Find and remove the player from the queue
            if let Some(index) = queue.iter().position(|p| p.connection_id == connection_id) {
                queue.remove(index);

                // Update positions for remaining players
                positions.clear();
                for (i, player) in queue.iter().enumerate() {
                    positions.insert(player.connection_id, i);
                }

                // Remove their vote if they had one
                votes.remove(&connection_id);

                // Clear countdown if we fall below minimum players
                if queue.len() < self.min_players {
                    let mut countdown_started = self.countdown_started_at.write().await;
                    if countdown_started.is_some() {
                        *countdown_started = None;
                        info!("Countdown stopped: not enough players ({}/{})", queue.len(), self.min_players);
                    }
                    votes.clear(); // Clear all votes when countdown stops
                }

                info!("Player {} removed from queue", connection_id);
                Ok(())
            } else {
                Err("Player not found in queue".to_string())
            }
        } else {
            Err("Player not in queue".to_string())
        }
    }

    // This should only be called when countdown expires or enough votes are accumulated
    pub async fn try_create_match(&self) -> Result<Option<MatchInfo>, String> {
        let mut queue = self.queue.write().await;
        let mut positions = self.player_positions.write().await;

        // Check if we have enough players for a match
        if queue.len() < self.min_players {
            return Ok(None);
        }

        // Check if countdown is active and whether we should actually create the match
        let countdown_started = self.countdown_started_at.read().await;
        let votes = self.votes_to_start.read().await;
        
        let should_create_match = if let Some(started_at) = *countdown_started {
            let elapsed = Instant::now().duration_since(started_at);
            
            // Only create match if countdown has expired OR enough votes
            if elapsed >= self.countdown_duration {
                true // Countdown expired
            } else {
                // Check if enough players voted (60% threshold)
                let total_players = queue.len();
                let votes_needed = ((total_players as f64 * 0.6).ceil() as usize).max(1);
                votes.len() >= votes_needed
            }
        } else {
            // No countdown active, don't create match
            false
        };
        
        if !should_create_match {
            return Ok(None);
        }
        
        // Release the read locks before getting write locks
        drop(countdown_started);
        drop(votes);

        // Clear countdown state since we're creating a match
        {
            let mut countdown_started = self.countdown_started_at.write().await;
            *countdown_started = None;
        }
        {
            let mut votes = self.votes_to_start.write().await;
            votes.clear();
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

    // Check if countdown has expired or if there are enough votes to start
    pub async fn should_start_match(&self) -> bool {
        let countdown_started = self.countdown_started_at.read().await;
        let queue = self.queue.read().await;
        let votes = self.votes_to_start.read().await;

        if let Some(started_at) = *countdown_started {
            let elapsed = Instant::now().duration_since(started_at);
            
            // Check if countdown has expired
            if elapsed >= self.countdown_duration {
                return true;
            }

            // Check if enough players voted (60% threshold)
            let total_players = queue.len();
            let votes_needed = ((total_players as f64 * 0.6).ceil() as usize).max(1);
            if votes.len() >= votes_needed {
                return true;
            }
        }

        false
    }

    pub async fn vote_to_start(&self, connection_id: ConnectionId) -> Result<bool, String> {
        let queue = self.queue.read().await;
        let positions = self.player_positions.read().await;
        let mut votes = self.votes_to_start.write().await;

        // Check if player is in queue
        if !positions.contains_key(&connection_id) {
            return Err("Player not in queue".to_string());
        }

        // Check if countdown is active
        let countdown_started = self.countdown_started_at.read().await;
        if countdown_started.is_none() {
            return Err("No countdown active".to_string());
        }

        // Add vote
        votes.insert(connection_id);

        // Check if we have enough votes (60% threshold)
        let total_players = queue.len();
        let votes_needed = ((total_players as f64 * 0.6).ceil() as usize).max(1);
        let has_enough_votes = votes.len() >= votes_needed;

        info!("Player {} voted to start. Votes: {}/{} (need {})", 
              connection_id, votes.len(), total_players, votes_needed);

        Ok(has_enough_votes)
    }

    pub async fn get_countdown_info(&self) -> Option<CountdownInfo> {
        let countdown_started = self.countdown_started_at.read().await;
        let queue = self.queue.read().await;
        let votes = self.votes_to_start.read().await;

        if let Some(started_at) = *countdown_started {
            let elapsed = Instant::now().duration_since(started_at);
            let remaining = self.countdown_duration.saturating_sub(elapsed);
            
            Some(CountdownInfo {
                seconds_remaining: remaining.as_secs() as u32,
                players_ready: votes.len() as u32,
                total_players: queue.len() as u32,
            })
        } else {
            None
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

    pub async fn get_queue_players(&self) -> Vec<ConnectionId> {
        let queue = self.queue.read().await;
        queue.iter().map(|player| player.connection_id).collect()
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

        // Clear countdown if we fall below minimum players
        if queue.len() < self.min_players {
            let mut countdown_started = self.countdown_started_at.write().await;
            if countdown_started.is_some() {
                *countdown_started = None;
                let mut votes = self.votes_to_start.write().await;
                votes.clear();
                info!("Countdown stopped due to player cleanup");
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_queue_stats(&self) -> QueueStats {
        let queue = self.queue.read().await;
        let votes = self.votes_to_start.read().await;
        let countdown_started = self.countdown_started_at.read().await;

        QueueStats {
            total_players: queue.len(),
            votes_count: votes.len(),
            countdown_active: countdown_started.is_some(),
        }
    }
}

#[derive(Debug)]
pub struct QueueStats {
    pub total_players: usize,
    pub votes_count: usize,
    pub countdown_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_queue_operations() {
        let queue = MatchmakingQueue::new();
        let mut players = Vec::new();

        // Add first player - no match should be created
        let conn_id1 = ConnectionId::new();
        queue.add_player(conn_id1).await.unwrap();
        players.push(conn_id1);

        assert_eq!(queue.get_queue_length().await, 1);
        assert!(queue.should_start_match().await == false);

        // Add second player - countdown should start, but no immediate match
        let conn_id2 = ConnectionId::new();
        queue.add_player(conn_id2).await.unwrap();
        players.push(conn_id2);

        assert_eq!(queue.get_queue_length().await, 2);
        
        // Countdown should be active but match should not start immediately
        let countdown_info = queue.get_countdown_info().await;
        assert!(countdown_info.is_some());
        assert!(queue.should_start_match().await == false); // Not enough time elapsed or votes

        // Cleanup
        for player in players {
            queue.remove_player(player).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_countdown_expiration() {
        let queue = MatchmakingQueue::new_with_config(2, 16, Duration::from_secs(300));
        // Override countdown to be very short for testing
        {
            let mut countdown = queue.countdown_started_at.write().await;
            *countdown = Some(Instant::now() - Duration::from_secs(65)); // Simulate expired countdown
        }

        let conn_id1 = ConnectionId::new();
        let conn_id2 = ConnectionId::new();
        queue.add_player(conn_id1).await.unwrap();
        queue.add_player(conn_id2).await.unwrap();

        // Should indicate match should start due to expired countdown
        assert!(queue.should_start_match().await);

        // Now we can create the match
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
    async fn test_voting_mechanism() {
        let queue = MatchmakingQueue::new();
        let conn_id1 = ConnectionId::new();
        let conn_id2 = ConnectionId::new();
        let conn_id3 = ConnectionId::new();

        // Add players to start countdown
        queue.add_player(conn_id1).await.unwrap();
        queue.add_player(conn_id2).await.unwrap();
        queue.add_player(conn_id3).await.unwrap();

        // Test voting
        let has_enough = queue.vote_to_start(conn_id1).await.unwrap();
        assert!(!has_enough); // 1/3 = 33%, need 60%

        let has_enough = queue.vote_to_start(conn_id2).await.unwrap();
        assert!(has_enough); // 2/3 = 67%, exceeds 60%

        // Should indicate match should start due to enough votes
        assert!(queue.should_start_match().await);

        // Cleanup
        queue.remove_player(conn_id1).await.ok();
        queue.remove_player(conn_id2).await.ok();
        queue.remove_player(conn_id3).await.ok();
    }

    #[tokio::test]
    async fn test_countdown_info() {
        let queue = MatchmakingQueue::new();
        
        // No countdown initially
        assert!(queue.get_countdown_info().await.is_none());

        // Add players to start countdown
        let conn_id1 = ConnectionId::new();
        let conn_id2 = ConnectionId::new();
        queue.add_player(conn_id1).await.unwrap();
        queue.add_player(conn_id2).await.unwrap();

        // Should have countdown info now
        let info = queue.get_countdown_info().await;
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert_eq!(info.total_players, 2);
        assert_eq!(info.players_ready, 0); // No votes yet
        assert!(info.seconds_remaining <= 60);

        // Vote and check updated info
        queue.vote_to_start(conn_id1).await.unwrap();
        let info = queue.get_countdown_info().await.unwrap();
        assert_eq!(info.players_ready, 1);

        // Cleanup
        queue.remove_player(conn_id1).await.ok();
        queue.remove_player(conn_id2).await.ok();
    }

    #[tokio::test]
    async fn test_immediate_match_creation_blocked() {
        let queue = MatchmakingQueue::new();

        // Add first player
        let conn_id1 = ConnectionId::new();
        queue.add_player(conn_id1).await.unwrap();

        // Should not be able to create match yet
        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_none());

        // Add second player - countdown starts
        let conn_id2 = ConnectionId::new();
        queue.add_player(conn_id2).await.unwrap();

        // Still should not be able to create match immediately
        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_none()); // This is the key test - no immediate match!

        // But countdown should be active
        assert!(queue.get_countdown_info().await.is_some());

        // Cleanup
        queue.remove_player(conn_id1).await.ok();
        queue.remove_player(conn_id2).await.ok();
    }

    #[tokio::test]
    async fn test_max_players_limit() {
        let queue = MatchmakingQueue::new_with_config(2, 3, Duration::from_secs(300));
        let mut players = Vec::new();

        // Add more than max players
        for _i in 0..5 {
            let conn_id = ConnectionId::new();
            queue.add_player(conn_id).await.unwrap();
            players.push(conn_id);
        }

        // Force match creation (simulate countdown expiration)
        {
            let mut countdown = queue.countdown_started_at.write().await;
            *countdown = Some(Instant::now() - Duration::from_secs(65));
        }

        // Try to create match - should take only 3 players (max)
        let match_result = queue.try_create_match().await.unwrap();
        assert!(match_result.is_some());

        let match_info = match_result.unwrap();
        assert_eq!(match_info.players.len(), 3);

        // Should still have 2 players in queue
        assert_eq!(queue.get_queue_length().await, 2);

        // Cleanup remaining players
        for player in &players[3..] {
            queue.remove_player(*player).await.ok();
        }
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let queue = std::sync::Arc::new(MatchmakingQueue::new());
        let mut players = Vec::new();

        // Add players concurrently
        for _ in 0..3 {
            let conn_id = ConnectionId::new();
            players.push(conn_id);
        }

        for &conn_id in &players {
            queue.add_player(conn_id).await.unwrap();
        }

        // Simulate race condition: try to create match and remove player simultaneously
        let queue_clone1 = queue.clone();
        let queue_clone2 = queue.clone();
        let player_to_remove = players[1];

        // Force countdown expiration for one of the tasks
        {
            let mut countdown = queue.countdown_started_at.write().await;
            *countdown = Some(Instant::now() - Duration::from_secs(65));
        }

        let match_task = tokio::spawn(async move { queue_clone1.try_create_match().await });

        let remove_task =
            tokio::spawn(async move { queue_clone2.remove_player(player_to_remove).await });

        let (match_result, _remove_result) = tokio::join!(match_task, remove_task);

        // At least one operation should succeed
        let match_result = match_result.unwrap();
        assert!(match_result.is_ok());

        // Cleanup remaining players
        for &player in &players {
            queue.remove_player(player).await.ok();
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
        assert_eq!(stats.votes_count, 0);
        assert!(!stats.countdown_active);

        // Try to vote with empty queue
        let vote_result = queue.vote_to_start(ConnectionId::new()).await;
        assert!(vote_result.is_err());

        // Try to remove non-existent player
        let remove_result = queue.remove_player(ConnectionId::new()).await;
        assert!(remove_result.is_err());
    }

    #[tokio::test]
    async fn test_queue_player_tracking() {
        let queue = MatchmakingQueue::new();
        
        // Initially no players
        let players = queue.get_queue_players().await;
        assert_eq!(players.len(), 0);
        
        // Add players and verify they're tracked
        let conn_id1 = ConnectionId::new();
        let conn_id2 = ConnectionId::new();
        let conn_id3 = ConnectionId::new();
        
        queue.add_player(conn_id1).await.unwrap();
        let players = queue.get_queue_players().await;
        assert_eq!(players.len(), 1);
        assert!(players.contains(&conn_id1));
        
        queue.add_player(conn_id2).await.unwrap();
        let players = queue.get_queue_players().await;
        assert_eq!(players.len(), 2);
        assert!(players.contains(&conn_id1));
        assert!(players.contains(&conn_id2));
        
        queue.add_player(conn_id3).await.unwrap();
        let players = queue.get_queue_players().await;
        assert_eq!(players.len(), 3);
        assert!(players.contains(&conn_id1));
        assert!(players.contains(&conn_id2));
        assert!(players.contains(&conn_id3));
        
        // Remove player and verify they're no longer tracked
        queue.remove_player(conn_id2).await.unwrap();
        let players = queue.get_queue_players().await;
        assert_eq!(players.len(), 2);
        assert!(players.contains(&conn_id1));
        assert!(!players.contains(&conn_id2));
        assert!(players.contains(&conn_id3));
        
        // Cleanup
        queue.remove_player(conn_id1).await.ok();
        queue.remove_player(conn_id3).await.ok();
    }

    #[tokio::test]
    async fn test_countdown_broadcast_to_all_players() {
        let queue = MatchmakingQueue::new();
        
        // Add first player - no countdown yet
        let conn_id1 = ConnectionId::new();
        queue.add_player(conn_id1).await.unwrap();
        
        let countdown_info = queue.get_countdown_info().await;
        assert!(countdown_info.is_none());
        
        // Add second player - countdown should start
        let conn_id2 = ConnectionId::new();
        queue.add_player(conn_id2).await.unwrap();
        
        // Verify countdown is active
        let countdown_info = queue.get_countdown_info().await;
        assert!(countdown_info.is_some());
        let info = countdown_info.unwrap();
        assert_eq!(info.total_players, 2);
        assert_eq!(info.players_ready, 0); // No votes yet
        
        // Verify both players are in queue for broadcasting
        let queue_players = queue.get_queue_players().await;
        assert_eq!(queue_players.len(), 2);
        assert!(queue_players.contains(&conn_id1));
        assert!(queue_players.contains(&conn_id2));
        
        // Add third player
        let conn_id3 = ConnectionId::new();
        queue.add_player(conn_id3).await.unwrap();
        
        // Verify all three players would receive countdown broadcast
        let queue_players = queue.get_queue_players().await;
        assert_eq!(queue_players.len(), 3);
        assert!(queue_players.contains(&conn_id1));
        assert!(queue_players.contains(&conn_id2));
        assert!(queue_players.contains(&conn_id3));
        
        // Verify countdown info reflects all players
        let countdown_info = queue.get_countdown_info().await.unwrap();
        assert_eq!(countdown_info.total_players, 3);
        
        // Test voting updates the broadcast info
        queue.vote_to_start(conn_id1).await.unwrap();
        let countdown_info = queue.get_countdown_info().await.unwrap();
        assert_eq!(countdown_info.players_ready, 1);
        
        queue.vote_to_start(conn_id2).await.unwrap();
        let countdown_info = queue.get_countdown_info().await.unwrap();
        assert_eq!(countdown_info.players_ready, 2);
        
        // Cleanup
        queue.remove_player(conn_id1).await.ok();
        queue.remove_player(conn_id2).await.ok();
        queue.remove_player(conn_id3).await.ok();
    }
}