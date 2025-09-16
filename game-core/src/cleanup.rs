use crate::{GameEvent, GameManager};
use std::time::Duration;

pub struct GameCleanup {
    pub abandoned_threshold: Duration,  // 10 minutes no activity
    pub completion_threshold: Duration, // 2 hours max game length
    pub queue_timeout: Duration,        // 5 minutes in queue
}

impl Default for GameCleanup {
    fn default() -> Self {
        Self {
            abandoned_threshold: Duration::from_secs(600), // 10 minutes
            completion_threshold: Duration::from_secs(7200), // 2 hours
            queue_timeout: Duration::from_secs(300),       // 5 minutes
        }
    }
}

impl GameCleanup {
    pub fn new(
        abandoned_threshold: Duration,
        completion_threshold: Duration,
        queue_timeout: Duration,
    ) -> Self {
        Self {
            abandoned_threshold,
            completion_threshold,
            queue_timeout,
        }
    }

    /// Cleanup abandoned games based on various criteria
    pub fn cleanup_abandoned_games(&self, game_manager: &mut GameManager) {
        // Track games to remove
        let mut games_to_remove = Vec::new();

        for (game_id, game) in &game_manager.active_games {
            let should_remove = self.should_cleanup_game(game);

            if should_remove {
                games_to_remove.push(*game_id);
            }
        }

        // Remove games and emit events
        for game_id in games_to_remove {
            if let Some(game) = game_manager.active_games.remove(&game_id) {
                // Remove player mappings
                for player in &game.state.players {
                    game_manager.player_to_game.remove(&player.user_id);
                }

                // Determine cleanup reason
                let reason = if self.is_all_players_disconnected(&game) {
                    "All players disconnected".to_string()
                } else if game.is_expired(self.completion_threshold) {
                    "Game exceeded maximum duration".to_string()
                } else {
                    "Inactivity timeout".to_string()
                };

                // Emit appropriate event
                let event = if game.is_expired(self.completion_threshold) {
                    GameEvent::GameTimedOut { game_id }
                } else {
                    GameEvent::GameAbandoned { game_id, reason }
                };

                game_manager.event_bus.publish(event);
            }
        }
    }

    /// Check if a game should be cleaned up
    fn should_cleanup_game(&self, game: &crate::Game) -> bool {
        // Check for inactivity
        if game.is_expired(self.abandoned_threshold) {
            return true;
        }

        // Check for maximum duration exceeded
        if game.is_expired(self.completion_threshold) {
            return true;
        }

        // Check if all players are disconnected
        if self.is_all_players_disconnected(game) {
            return true;
        }

        false
    }

    /// Check if all players in the game are disconnected
    fn is_all_players_disconnected(&self, game: &crate::Game) -> bool {
        !game.state.players.is_empty()
            && game.state.players.iter().all(|player| !player.is_connected)
    }

    /// Cleanup stale queue entries (players who have been in queue too long)
    pub fn cleanup_stale_queue(&self, game_manager: &mut GameManager) {
        // Note: In a real implementation, you'd track when players joined the queue
        // For now, this is a placeholder that could be expanded with timestamp tracking

        // Clear the queue if it's been too long (simplified approach)
        if game_manager.game_queue.len() > 100 {
            game_manager.game_queue.clear();
        }
    }

    /// Perform all cleanup operations
    pub fn cleanup_all(&self, game_manager: &mut GameManager) {
        self.cleanup_abandoned_games(game_manager);
        self.cleanup_stale_queue(game_manager);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, WordValidator};
    use game_types::{GameStatus, Player};
    use std::time::SystemTime;
    use uuid::Uuid;

    fn create_test_game() -> Game {
        let player = Player {
            user_id: Uuid::new_v4(),
            display_name: "Test".to_string(),
            points: 0,
            guess_history: Vec::new(),
            is_connected: true,
        };

        Game::new(Uuid::new_v4(), vec![player], "test".to_string(), 25)
    }

    #[test]
    fn test_cleanup_configuration() {
        let cleanup = GameCleanup::default();

        assert_eq!(cleanup.abandoned_threshold, Duration::from_secs(600));
        assert_eq!(cleanup.completion_threshold, Duration::from_secs(7200));
        assert_eq!(cleanup.queue_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_all_players_disconnected() {
        let cleanup = GameCleanup::default();
        let mut game = create_test_game();

        // Initially connected
        assert!(!cleanup.is_all_players_disconnected(&game));

        // Disconnect all players
        for player in &mut game.state.players {
            player.is_connected = false;
        }

        assert!(cleanup.is_all_players_disconnected(&game));
    }
}
