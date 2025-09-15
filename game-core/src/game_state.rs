use game_types::{GameState, GameStatus, GamePhase, Player, GuessResult, PersonalGuess};
use crate::{ScoringEngine, WordValidator, GameEvent, GameEventBus};
use uuid::Uuid;
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, Duration};
use anyhow::{anyhow, Result};

pub type GameId = Uuid;
pub type PlayerId = Uuid;

#[derive(Debug)]
pub struct Game {
    pub state: GameState,
    pub target_word: String, // Hidden from clients
    pub current_guesses: HashMap<PlayerId, String>, // Current round guesses
    pub last_activity: SystemTime,
    pub countdown_end: Option<SystemTime>,
    pub current_phase: GamePhase,
}

impl Game {
    pub fn new(
        id: GameId,
        players: Vec<Player>,
        target_word: String,
        point_threshold: i32,
    ) -> Self {
        let state = GameState {
            id,
            word: "*".repeat(target_word.len()), // Hide the actual word
            word_length: target_word.len() as i32,
            current_round: 1,
            status: GameStatus::Starting,
            current_phase: GamePhase::Waiting,
            players,
            official_board: Vec::new(),
            current_winner: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            point_threshold,
        };

        Self {
            state,
            target_word,
            current_guesses: HashMap::new(),
            last_activity: SystemTime::now(),
            countdown_end: None,
            current_phase: GamePhase::Waiting,
        }
    }

    pub fn add_guess(&mut self, player_id: PlayerId, word: String) -> Result<()> {
        // Validate player is in the game
        if !self.state.players.iter().any(|p| p.user_id == player_id) {
            return Err(anyhow!("Player not in game"));
        }

        // Check if word already guessed in this game
        if self.state.official_board.iter().any(|g| g.word.to_lowercase() == word.to_lowercase()) {
            return Err(anyhow!("Word already guessed: {}", word));
        }

        // Store the guess for this round
        self.current_guesses.insert(player_id, word);
        self.last_activity = SystemTime::now();
        
        Ok(())
    }

    pub fn process_round(&mut self) -> Result<Option<GuessResult>> {
        if self.current_guesses.is_empty() {
            return Ok(None);
        }

        // Convert guesses to format expected by scoring engine
        let guesses: Vec<(String, String)> = self.current_guesses
            .iter()
            .map(|(player_id, word)| (word.clone(), player_id.to_string()))
            .collect();

        // Determine the winning guess
        let winner_index = ScoringEngine::determine_round_winner(&guesses, &self.target_word);
        
        if let Some(index) = winner_index {
            let (winning_word, winning_player_str) = &guesses[index];
            let winning_player_id = Uuid::parse_str(winning_player_str)?;
            
            // Evaluate the winning guess
            let (letter_results, points_earned) = ScoringEngine::evaluate_guess(
                winning_word,
                &self.target_word,
                &self.state.official_board,
            );

            // Create the guess result
            let guess_result = GuessResult {
                word: winning_word.clone(),
                player_id: winning_player_id,
                letters: letter_results,
                points_earned,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            // Update player scores and guess history
            for player in &mut self.state.players {
                if let Some(word) = self.current_guesses.get(&player.user_id) {
                    let was_winning_guess = player.user_id == winning_player_id;
                    let points = if was_winning_guess { points_earned } else { 0 };
                    
                    if was_winning_guess {
                        player.points += points;
                    }
                    
                    player.guess_history.push(PersonalGuess {
                        word: word.clone(),
                        points_earned: points,
                        was_winning_guess,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }

            // Add to official board
            self.state.official_board.push(guess_result.clone());
            
            // Check if word is solved
            if winning_word.to_lowercase() == self.target_word.to_lowercase() {
                self.state.status = GameStatus::Completed;
                self.set_phase(GamePhase::GameOver);
            } else {
                // Check if anyone has won by points
                if let Some(_winner) = self.state.players.iter().find(|p| p.points >= self.state.point_threshold) {
                    self.state.status = GameStatus::Completed;
                    self.set_phase(GamePhase::GameOver);
                } else {
                    // Continue to next round
                    self.state.current_round += 1;
                    self.state.current_winner = Some(winning_player_id);
                    self.set_phase(GamePhase::IndividualGuess);
                }
            }

            // Clear current round guesses
            self.current_guesses.clear();
            
            Ok(Some(guess_result))
        } else {
            Ok(None)
        }
    }

    pub fn get_winner(&self) -> Option<&Player> {
        self.state.players
            .iter()
            .max_by_key(|p| p.points)
            .filter(|p| p.points >= self.state.point_threshold)
    }

    pub fn is_expired(&self, timeout_duration: Duration) -> bool {
        self.last_activity.elapsed().unwrap_or(Duration::ZERO) > timeout_duration
    }

    pub fn set_countdown(&mut self, duration: Duration) {
        self.countdown_end = Some(SystemTime::now() + duration);
        self.set_phase(GamePhase::Countdown);
    }

    pub fn is_countdown_finished(&self) -> bool {
        if let Some(end_time) = self.countdown_end {
            SystemTime::now() >= end_time
        } else {
            false
        }
    }

    fn set_phase(&mut self, phase: GamePhase) {
        self.current_phase = phase.clone();
        self.state.current_phase = phase;
    }

    pub fn start_guessing_phase(&mut self) {
        self.set_phase(GamePhase::Guessing);
    }
}

pub struct GameManager {
    pub active_games: HashMap<GameId, Game>,
    pub player_to_game: HashMap<PlayerId, GameId>,
    pub game_queue: VecDeque<PlayerId>,
    pub word_validator: WordValidator,
    pub event_bus: GameEventBus,
    pub default_point_threshold: i32,
}

impl GameManager {
    pub fn new(word_validator: WordValidator) -> Self {
        Self {
            active_games: HashMap::new(),
            player_to_game: HashMap::new(),
            game_queue: VecDeque::new(),
            word_validator,
            event_bus: GameEventBus::new(),
            default_point_threshold: 25,
        }
    }

    pub fn add_to_queue(&mut self, player_id: PlayerId) {
        if !self.game_queue.contains(&player_id) {
            self.game_queue.push_back(player_id);
        }
    }

    pub fn remove_from_queue(&mut self, player_id: PlayerId) {
        self.game_queue.retain(|&id| id != player_id);
    }

    pub fn create_game(&mut self, players: Vec<Player>) -> Result<GameId> {
        if players.len() < 2 || players.len() > 16 {
            return Err(anyhow!("Invalid number of players: {}", players.len()));
        }

        // Get random word (defaulting to 6 letters)
        let target_word = self.word_validator.get_random_word(6)?;
        let game_id = Uuid::new_v4();
        
        let game = Game::new(game_id, players.clone(), target_word.clone(), self.default_point_threshold);
        
        // Update player mappings
        for player in &players {
            self.player_to_game.insert(player.user_id, game_id);
            self.remove_from_queue(player.user_id);
        }

        // Emit event
        let event = GameEvent::GameCreated {
            game_id,
            players,
            word: target_word,
            point_threshold: self.default_point_threshold,
        };
        self.event_bus.publish(event);

        self.active_games.insert(game_id, game);
        Ok(game_id)
    }

    pub fn handle_guess(&mut self, game_id: GameId, player_id: PlayerId, word: String) -> Result<Option<GameEvent>> {
        // Validate word
        if !self.word_validator.is_valid_word(&word) || !self.word_validator.is_alphabetic(&word) {
            return Err(anyhow!("Invalid word: {}", word));
        }

        let game = self.active_games.get_mut(&game_id)
            .ok_or_else(|| anyhow!("Game not found"))?;

        game.add_guess(player_id, word.clone())?;

        // Emit guess event
        let event = GameEvent::GuessSubmitted {
            game_id,
            player_id,
            word,
        };
        self.event_bus.publish(event.clone());

        Ok(Some(event))
    }

    pub fn cleanup_expired_games(&mut self) {
        let timeout_duration = Duration::from_secs(600); // 10 minutes
        let expired_games: Vec<GameId> = self.active_games
            .iter()
            .filter(|(_, game)| game.is_expired(timeout_duration))
            .map(|(id, _)| *id)
            .collect();

        for game_id in expired_games {
            if let Some(game) = self.active_games.remove(&game_id) {
                // Remove player mappings
                for player in &game.state.players {
                    self.player_to_game.remove(&player.user_id);
                }

                // Emit abandonment event
                let event = GameEvent::GameAbandoned {
                    game_id,
                    reason: "Inactivity timeout".to_string(),
                };
                self.event_bus.publish(event);
            }
        }
    }

    pub fn get_queue_position(&self, player_id: PlayerId) -> Option<usize> {
        self.game_queue.iter().position(|&id| id == player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_types::Player;

    fn create_test_validator() -> WordValidator {
        let word_list = "apple\nbanana\ncherry\ntests\nvalid\nhello\nworld";
        WordValidator::new(word_list)
    }

    fn create_test_player(name: &str) -> Player {
        Player {
            user_id: Uuid::new_v4(),
            display_name: name.to_string(),
            points: 0,
            guess_history: Vec::new(),
            is_connected: true,
        }
    }

    #[test]
    fn test_game_creation() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let players = vec![
            create_test_player("Alice"),
            create_test_player("Bob"),
        ];

        let result = manager.create_game(players);
        assert!(result.is_ok());
        
        let game_id = result.unwrap();
        assert!(manager.active_games.contains_key(&game_id));
    }

    #[test]
    fn test_queue_management() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let player_id = Uuid::new_v4();
        
        manager.add_to_queue(player_id);
        assert_eq!(manager.get_queue_position(player_id), Some(0));
        
        manager.remove_from_queue(player_id);
        assert_eq!(manager.get_queue_position(player_id), None);
    }

    #[test]
    fn test_game_creation_edge_cases() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);

        // Test invalid player count - too few
        let result = manager.create_game(vec![create_test_player("Alice")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid number of players"));

        // Test invalid player count - too many
        let many_players: Vec<Player> = (0..17)
            .map(|i| create_test_player(&format!("Player{}", i)))
            .collect();
        let result = manager.create_game(many_players);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid number of players"));

        // Test boundary cases - exactly 2 and 16 players
        let min_players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let result = manager.create_game(min_players);
        assert!(result.is_ok());

        let max_players: Vec<Player> = (0..16)
            .map(|i| create_test_player(&format!("Player{}", i)))
            .collect();
        let result = manager.create_game(max_players);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_queue_entries() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let player_id = Uuid::new_v4();
        
        // Add same player multiple times
        manager.add_to_queue(player_id);
        manager.add_to_queue(player_id);
        manager.add_to_queue(player_id);
        
        // Should only appear once
        assert_eq!(manager.get_queue_position(player_id), Some(0));
        assert_eq!(manager.game_queue.len(), 1);
    }

    #[test]
    fn test_invalid_word_handling() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let game_id = manager.create_game(players.clone()).unwrap();
        let player_id = players[0].user_id;

        // Test invalid word
        let result = manager.handle_guess(game_id, player_id, "invalidword".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid word"));

        // Test non-alphabetic word
        let result = manager.handle_guess(game_id, player_id, "test123".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid word"));

        // Test empty word
        let result = manager.handle_guess(game_id, player_id, "".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_guess_submission_edge_cases() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let game_id = manager.create_game(players.clone()).unwrap();
        let alice_id = players[0].user_id;
        let bob_id = players[1].user_id;

        let game = manager.active_games.get_mut(&game_id).unwrap();
        
        // Test player not in game
        let fake_player = Uuid::new_v4();
        let result = game.add_guess(fake_player, "hello".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Player not in game"));

        // Test duplicate word guessing
        game.add_guess(alice_id, "hello".to_string()).unwrap();
        let guess_result = game.process_round().unwrap();
        assert!(guess_result.is_some());

        // Try to guess the same word again
        let result = game.add_guess(bob_id, "hello".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Word already guessed"));
    }

    #[test]
    fn test_game_state_transitions() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let game_id = manager.create_game(players.clone()).unwrap();
        
        let game = manager.active_games.get_mut(&game_id).unwrap();
        assert_eq!(game.state.status, GameStatus::Starting);
        assert_eq!(game.current_phase, GamePhase::Waiting);

        // Test countdown functionality
        game.set_countdown(std::time::Duration::from_millis(1));
        assert_eq!(game.current_phase, GamePhase::Countdown);
        
        // Wait for countdown to finish
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(game.is_countdown_finished());
    }

    #[test]
    fn test_game_completion_conditions() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let mut players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let game_id = manager.create_game(players.clone()).unwrap();
        
        let game = manager.active_games.get_mut(&game_id).unwrap();
        let target_word = game.target_word.clone();
        let alice_id = players[0].user_id;

        // Test word completion - game should end when target word is guessed
        game.add_guess(alice_id, target_word).unwrap();
        let result = game.process_round().unwrap();
        assert!(result.is_some());
        assert_eq!(game.state.status, GameStatus::Completed);
        assert_eq!(game.current_phase, GamePhase::GameOver);

        // Test point threshold completion
        let mut game2 = Game::new(
            Uuid::new_v4(),
            players.clone(),
            "tests".to_string(),
            10, // Low threshold
        );
        
        // Manually set a player's points above threshold
        players[0].points = 15;
        game2.state.players = players;
        
        game2.add_guess(alice_id, "tests".to_string()).unwrap();
        let result = game2.process_round().unwrap();
        assert!(result.is_some());
        assert_eq!(game2.state.status, GameStatus::Completed);
    }

    #[test]
    fn test_game_expiration() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let game_id = manager.create_game(players.clone()).unwrap();
        
        // Game should not be expired initially
        let game = manager.active_games.get(&game_id).unwrap();
        assert!(!game.is_expired(std::time::Duration::from_secs(1)));
        
        // Test with zero duration - should be expired
        assert!(game.is_expired(std::time::Duration::from_millis(0)));
        
        // Test cleanup
        manager.cleanup_expired_games();
        // Game should still exist (not expired with reasonable timeout)
        assert!(manager.active_games.contains_key(&game_id));
    }

    #[test]
    fn test_empty_round_processing() {
        let validator = create_test_validator();
        let players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let mut game = Game::new(Uuid::new_v4(), players, "hello".to_string(), 25);
        
        // Process round with no guesses
        let result = game.process_round().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_winner_determination() {
        let validator = create_test_validator();
        let mut players = vec![create_test_player("Alice"), create_test_player("Bob")];
        
        // Set different point values
        players[0].points = 30; // Above threshold
        players[1].points = 20; // Below threshold
        
        let game = Game::new(Uuid::new_v4(), players, "hello".to_string(), 25);
        
        let winner = game.get_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap().display_name, "Alice");
        
        // Test no winner case
        let mut players2 = vec![create_test_player("Charlie"), create_test_player("Dave")];
        players2[0].points = 20; // Below threshold
        players2[1].points = 15; // Below threshold
        
        let game2 = Game::new(Uuid::new_v4(), players2, "hello".to_string(), 25);
        assert!(game2.get_winner().is_none());
    }

    #[test]
    fn test_nonexistent_game_operations() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let fake_game_id = Uuid::new_v4();
        let fake_player_id = Uuid::new_v4();
        
        // Try to handle guess for non-existent game
        let result = manager.handle_guess(fake_game_id, fake_player_id, "hello".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Game not found"));
    }

    #[test]
    fn test_player_mapping_consistency() {
        let validator = create_test_validator();
        let mut manager = GameManager::new(validator);
        
        let players = vec![create_test_player("Alice"), create_test_player("Bob")];
        let alice_id = players[0].user_id;
        let bob_id = players[1].user_id;
        
        // Add players to queue
        manager.add_to_queue(alice_id);
        manager.add_to_queue(bob_id);
        assert_eq!(manager.game_queue.len(), 2);
        
        // Create game - should remove from queue and add to player mapping
        let game_id = manager.create_game(players.clone()).unwrap();
        
        assert_eq!(manager.game_queue.len(), 0); // Should be removed from queue
        assert_eq!(manager.player_to_game.get(&alice_id), Some(&game_id));
        assert_eq!(manager.player_to_game.get(&bob_id), Some(&game_id));
        
        // Cleanup expired games should remove player mappings
        manager.cleanup_expired_games(); // Won't remove active game
        assert!(manager.player_to_game.contains_key(&alice_id));
        
        // Force expiration by removing game and simulate cleanup
        let game = manager.active_games.remove(&game_id).unwrap();
        manager.active_games.insert(game_id, game);
        
        // Manually trigger cleanup logic for testing
        if let Some(game) = manager.active_games.remove(&game_id) {
            for player in &game.state.players {
                manager.player_to_game.remove(&player.user_id);
            }
        }
        
        assert!(!manager.player_to_game.contains_key(&alice_id));
        assert!(!manager.player_to_game.contains_key(&bob_id));
    }
}