use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;
use chrono;

use crate::websocket::connection::{ConnectionId, ConnectionManager};
use game_core::{Game, PlayerId, WordValidator};
use game_types::{GamePhase, GameState, GuessResult, PersonalGuess, Player, SafeGameState, User, RoundCompletion, RoundResult};

#[derive(Debug, Clone)]
pub enum GameEvent {
    RoundResult {
        winning_guess: GuessResult,
        player_guesses: Vec<(ConnectionId, PersonalGuess)>,
        is_word_completed: bool,
    },
    GameOver {
        winner: Player,
        final_scores: Vec<Player>,
    },
    StateUpdate {
        state: GameState,
    },
}

#[derive(Debug)]
struct ActiveGame {
    id: String,
    game: Game,
    connection_to_player: HashMap<ConnectionId, PlayerId>,
    player_to_connection: HashMap<PlayerId, ConnectionId>,
    created_at: Instant,
    last_activity: Instant,
}

impl ActiveGame {
    fn new(
        id: String, 
        authenticated_players: Vec<(ConnectionId, User)>,
        word_validator: &WordValidator,
    ) -> Result<Self, String> {
        if authenticated_players.is_empty() {
            return Err("Cannot create game with no players".to_string());
        }

        let mut connection_to_player = HashMap::new();
        let mut player_to_connection = HashMap::new();
        let mut game_players = Vec::new();

        for (connection_id, user) in authenticated_players.iter() {
            let player = Player {
                user_id: user.id,
                display_name: user.display_name.clone(),
                points: 0,
                guess_history: Vec::new(),
                is_connected: true,
            };

            game_players.push(player);
            connection_to_player.insert(*connection_id, user.id);
            player_to_connection.insert(user.id, *connection_id);
        }

        // Get a random word from the shared word validator
        let target_word = word_validator
            .get_random_word_random_length()
            .expect("Failed to get random word");

        let mut game = Game::new(
            Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4()),
            game_players,
            target_word,
            25, // Points to win from config
        );

        // Start the first round immediately
        game.state.status = game_types::GameStatus::Active;
        game.start_guessing_phase();

        let now = Instant::now();
        Ok(Self {
            id,
            game,
            connection_to_player,
            player_to_connection,
            created_at: now,
            last_activity: now,
        })
    }

    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    fn is_expired(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    fn convert_to_api_state(&self) -> GameState {
        // For now, return the game state directly since it's already in the right format
        self.game.state.clone()
    }
}

pub struct GameManager {
    active_games: RwLock<HashMap<String, ActiveGame>>,
    connection_to_game: RwLock<HashMap<ConnectionId, String>>,
    word_validator: Arc<WordValidator>,
    connection_manager: Arc<ConnectionManager>,
}

impl GameManager {
    /// Create a new GameManager with word lists loaded from a directory
    pub fn new<P: AsRef<std::path::Path>>(
        connection_manager: Arc<ConnectionManager>,
        words_dir: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let word_validator = WordValidator::new(words_dir)?;
        Ok(Self {
            active_games: RwLock::new(HashMap::new()),
            connection_to_game: RwLock::new(HashMap::new()),
            word_validator: Arc::new(word_validator),
            connection_manager,
        })
    }

    /// Create a new GameManager with word lists loaded from the default directory
    /// Uses WORD_LISTS_DIR environment variable or falls back to "../word_lists"
    pub fn new_with_default_words(
        connection_manager: Arc<ConnectionManager>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let words_dir = std::env::var("WORD_LISTS_DIR").unwrap_or_else(|_| "../word_lists".to_string());
        Self::new(connection_manager, words_dir)
    }

    pub async fn create_game(&self, players: Vec<ConnectionId>) -> Result<String, String> {
        if players.len() < 2 {
            return Err("Need at least 2 players to create a game".to_string());
        }

        // Validate that all players are authenticated and get their user info
        let mut authenticated_players = Vec::new();
        for connection_id in &players {
            let connection = self.connection_manager.get_connection(*connection_id).await
                .ok_or_else(|| format!("Connection {} not found", connection_id))?;
            
            if !connection.is_authenticated {
                return Err(format!("Connection {} is not authenticated", connection_id));
            }
            
            let user = connection.user
                .ok_or_else(|| format!("No user info for connection {}", connection_id))?;
            
            authenticated_players.push((*connection_id, user));
        }

        // Check for duplicate users - prevent same user from joining twice
        let mut user_ids = std::collections::HashSet::new();
        for (_, user) in &authenticated_players {
            if !user_ids.insert(user.id) {
                return Err(format!("User {} is already in the game", user.display_name));
            }
        }

        let game_id = Uuid::new_v4().to_string();
        let active_game = ActiveGame::new(game_id.clone(), authenticated_players, &self.word_validator)?;

        {
            let mut games = self.active_games.write().await;
            games.insert(game_id.clone(), active_game);
        }

        {
            let mut connection_to_game = self.connection_to_game.write().await;
            for &player_id in &players {
                connection_to_game.insert(player_id, game_id.clone());
            }
        }

        info!("Created game {} with {} authenticated players", game_id, players.len());
        Ok(game_id)
    }

    pub async fn submit_guess(
        &self,
        game_id: &str,
        connection_id: ConnectionId,
        word: String,
    ) -> Result<GameEvent, String> {
        let mut games = self.active_games.write().await;
        let active_game = games.get_mut(game_id).ok_or("Game not found")?;

        active_game.update_activity();

        let player_id = active_game
            .connection_to_player
            .get(&connection_id)
            .ok_or("Player not in game")?
            .clone();

        // Validate word
        if !self.word_validator.is_valid_word(&word) {
            return Err("Invalid word".to_string());
        }

        // Handle different game phases
        tracing::info!("Processing guess '{}' from player {} in phase {:?}", word, player_id, active_game.game.current_phase);
        match active_game.game.current_phase {
            GamePhase::IndividualGuess => {
                // Individual guess phase - only winner can guess
                tracing::info!("Individual guess phase - current winner: {:?}, submitting player: {}", active_game.game.state.current_winner, player_id);
                match active_game.game.process_individual_guess(player_id, word) {
                    Ok(Some(round_result)) => {
                        match round_result {
                            RoundResult::Continuing(guess_result) => {
                                // Create personal guess for the player
                                let personal_guess = if let Some(last_guess) = active_game.game.state.players
                                    .iter()
                                    .find(|p| p.user_id == player_id)
                                    .and_then(|p| p.guess_history.last()) {
                                    vec![(connection_id, last_guess.clone())]
                                } else {
                                    vec![]
                                };

                                return Ok(GameEvent::RoundResult {
                                    winning_guess: guess_result,
                                    player_guesses: personal_guess,
                                    is_word_completed: false, // Regular round, not word completion
                                });
                            }
                            RoundResult::WordCompleted(round_completion) => {
                                // Start a new round with a fresh word
                                self.start_new_round(active_game, round_completion).await
                            }
                            RoundResult::GameOver(_guess_result) => {
                                if let Some(winner) = active_game.game.state.players.iter().max_by_key(|p| p.points) {
                                    let final_scores = active_game.game.state.players.clone();
                                    return Ok(GameEvent::GameOver {
                                        winner: winner.clone(),
                                        final_scores,
                                    });
                                } else {
                                    return Err("Game over but no winner found".to_string());
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        return Ok(GameEvent::StateUpdate {
                            state: active_game.game.state.clone(),
                        });
                    }
                    Err(e) => {
                        return Err(format!("Individual guess error: {:?}", e));
                    }
                }
            }
            GamePhase::Guessing => {
                // Collaborative guessing phase
                if let Err(e) = active_game.game.add_guess(player_id, word.clone()) {
                    return Err(format!("Failed to add guess: {:?}", e));
                }

                // Check if all connected players have submitted guesses
                let connected_players: Vec<_> = active_game
                    .game
                    .state
                    .players
                    .iter()
                    .filter(|p| p.is_connected)
                    .map(|p| p.user_id)
                    .collect();

                let all_connected_guessed = connected_players
                    .iter()
                    .all(|player_id| active_game.game.current_guesses.contains_key(player_id));

                // Only process the round if all connected players have guessed
                if !all_connected_guessed {
                    // Not all players have guessed yet, return state update
                    return Ok(GameEvent::StateUpdate {
                        state: active_game.game.state.clone(),
                    });
                }

                // All players have guessed, process the round
                match active_game.game.process_round() {
            Ok(Some(round_result)) => {
                match round_result {
                    RoundResult::Continuing(winning_guess) => {
                        // The winning_guess is already in the correct format from game-core
                        // Create personal guess results for each player
                        let player_guesses: Vec<(ConnectionId, PersonalGuess)> = active_game
                            .game
                            .state
                            .players
                            .iter()
                            .filter_map(|player| {
                                let conn_id = active_game.player_to_connection.get(&player.user_id)?;

                                // Get the last guess from their history (most recent)
                                if let Some(last_guess) = player.guess_history.last() {
                                    Some((*conn_id, last_guess.clone()))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        Ok(GameEvent::RoundResult {
                            winning_guess,
                            player_guesses,
                            is_word_completed: false, // Regular round result
                        })
                    }
                    RoundResult::GameOver(winning_guess) => {
                        // Find the winner
                        if let Some(winner) = active_game
                            .game
                            .state
                            .players
                            .iter()
                            .max_by_key(|p| p.points)
                        {
                            let final_scores = active_game.game.state.players.clone();

                            Ok(GameEvent::GameOver {
                                winner: winner.clone(),
                                final_scores,
                            })
                        } else {
                            // Fallback to round result if no winner found
                            let player_guesses: Vec<(ConnectionId, PersonalGuess)> = active_game
                                .game
                                .state
                                .players
                                .iter()
                                .filter_map(|player| {
                                    let conn_id = active_game.player_to_connection.get(&player.user_id)?;
                                    if let Some(last_guess) = player.guess_history.last() {
                                        Some((*conn_id, last_guess.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            Ok(GameEvent::RoundResult {
                                winning_guess,
                                player_guesses,
                                is_word_completed: false, // Game over case
                            })
                        }
                    }
                    RoundResult::WordCompleted(round_completion) => {
                        // Start a new round with a fresh word
                        self.start_new_round(active_game, round_completion).await
                    }
                }
            }
                Ok(None) => {
                    // No round result yet (waiting for more guesses)
                    Ok(GameEvent::StateUpdate {
                        state: active_game.game.state.clone(),
                    })
                }
                Err(e) => {
                    Err(format!("Game error: {:?}", e))
                }
            }
            }
            _ => {
                // Other phases like Waiting, Countdown, GameOver - no guessing allowed
                Err("Cannot submit guess in current phase".to_string())
            }
        }
    }

    pub async fn get_game_state(&self, game_id: &str) -> Option<GameState> {
        let games = self.active_games.read().await;
        games.get(game_id).map(|game| game.convert_to_api_state())
    }

    /// Get safe game state for HTTP responses - doesn't expose the target word
    pub async fn get_safe_game_state(&self, game_id: &str) -> Option<SafeGameState> {
        let games = self.active_games.read().await;
        games.get(game_id).map(|game| {
            let full_state = game.convert_to_api_state();
            SafeGameState::from(&full_state)
        })
    }

    /// Check if a user is a participant in the given game
    pub async fn is_user_in_game(&self, game_id: &str, user_id: &Uuid) -> bool {
        let games = self.active_games.read().await;
        if let Some(active_game) = games.get(game_id) {
            let full_state = active_game.convert_to_api_state();
            full_state.players.iter().any(|p| &p.user_id == user_id)
        } else {
            false
        }
    }

    pub async fn handle_player_disconnect(
        &self,
        game_id: &str,
        connection_id: ConnectionId,
    ) -> Result<(), String> {
        let mut games = self.active_games.write().await;
        if let Some(active_game) = games.get_mut(game_id) {
            if let Some(player_id) = active_game.connection_to_player.get(&connection_id) {
                // Find player in game state and mark as disconnected
                for player in &mut active_game.game.state.players {
                    if player.user_id == *player_id {
                        player.is_connected = false;
                        info!("Player {} disconnected from game {}", player_id, game_id);
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn rejoin_player(
        &self,
        game_id: &str,
        connection_id: ConnectionId,
    ) -> Result<GameState, String> {
        let mut games = self.active_games.write().await;
        let active_game = games.get_mut(game_id).ok_or("Game not found")?;

        // For now, let's check if this connection was previously in the game
        // In a more robust system, we'd verify the user ID matches a disconnected player

        // Find a disconnected player to reconnect to
        let disconnected_player = active_game
            .game
            .state
            .players
            .iter()
            .find(|p| !p.is_connected)
            .ok_or("No disconnected players to rejoin")?
            .clone();

        // Update connection mappings
        active_game
            .connection_to_player
            .insert(connection_id, disconnected_player.user_id);
        active_game
            .player_to_connection
            .insert(disconnected_player.user_id, connection_id);

        // Mark player as connected in game state
        for player in &mut active_game.game.state.players {
            if player.user_id == disconnected_player.user_id {
                player.is_connected = true;
                break;
            }
        }

        // Update connection_to_game mapping
        {
            let mut connection_to_game = self.connection_to_game.write().await;
            connection_to_game.insert(connection_id, game_id.to_string());
        }

        active_game.update_activity();
        info!("Player {} rejoined game {}", connection_id, game_id);

        Ok(active_game.convert_to_api_state())
    }

    pub async fn remove_player(
        &self,
        game_id: &str,
        connection_id: ConnectionId,
    ) -> Result<(), String> {
        {
            let mut games = self.active_games.write().await;
            if let Some(active_game) = games.get_mut(game_id) {
                if let Some(player_id) = active_game.connection_to_player.remove(&connection_id) {
                    active_game.player_to_connection.remove(&player_id);

                    // Remove player from game state
                    active_game
                        .game
                        .state
                        .players
                        .retain(|p| p.user_id != player_id);
                }

                // If no players left, remove the game
                if active_game.game.state.players.is_empty() {
                    games.remove(game_id);
                    info!("Removed empty game {}", game_id);
                }
            }
        }

        {
            let mut connection_to_game = self.connection_to_game.write().await;
            connection_to_game.remove(&connection_id);
        }

        Ok(())
    }

    pub async fn cleanup_abandoned_games(&self, timeout: Duration) {
        let mut games_to_remove = Vec::new();

        {
            let games = self.active_games.read().await;
            for (game_id, game) in games.iter() {
                if game.is_expired(timeout)
                    || game.game.state.players.iter().all(|p| !p.is_connected)
                {
                    games_to_remove.push(game_id.clone());
                }
            }
        }

        if !games_to_remove.is_empty() {
            let mut games = self.active_games.write().await;
            let mut connection_to_game = self.connection_to_game.write().await;

            for game_id in games_to_remove {
                if let Some(game) = games.remove(&game_id) {
                    // Remove all connections for this game
                    for connection_id in game.connection_to_player.keys() {
                        connection_to_game.remove(connection_id);
                    }
                    info!("Removed abandoned game {}", game_id);
                }
            }
        }
    }

    /// Starts a new round with a fresh word after completing a word
    async fn start_new_round(&self, active_game: &mut ActiveGame, round_completion: RoundCompletion) -> Result<GameEvent, String> {
        // Get a new random word with random length (5-8 letters)
        let new_word = self.word_validator
            .get_random_word_random_length()
            .map_err(|e| format!("Failed to get new random word: {:?}", e))?;
        
        println!("Starting new round: completed word '{}' by player '{}', new word '{}'", round_completion.word, round_completion.player_id, new_word);
        
        // Update game with new word and reset state for new round
        active_game.game.target_word = new_word.clone();
        active_game.game.state.word = "*".repeat(new_word.len()); // Masked word for display
        active_game.game.state.word_length = new_word.len() as i32;
        
        println!("Before round increment: round = {}", active_game.game.state.current_round);
        active_game.game.state.current_round += 1;
        println!("After round increment: round = {}", active_game.game.state.current_round);
        active_game.game.state.official_board.clear(); // Clear the official board for new round
        active_game.game.state.current_winner = None;
        active_game.game.current_guesses.clear();
        
        // Reset to collaborative guessing phase
        active_game.game.current_phase = GamePhase::Guessing;
        active_game.game.start_guessing_phase();
        
        tracing::info!("New round started: round {}, new word length {}", 
                      active_game.game.state.current_round, 
                      active_game.game.state.word_length);
        
        // Get all players to send the round completion message to everyone
        let player_guesses: Vec<(ConnectionId, PersonalGuess)> = active_game
            .player_to_connection
            .iter()
            .filter_map(|(player_id, conn_id)| {
                // Find the player's last guess from their history
                let player = active_game.game.state.players
                    .iter()
                    .find(|p| &p.user_id == player_id)?;
                
                if let Some(last_guess) = player.guess_history.last() {
                    Some((*conn_id, last_guess.clone()))
                } else {
                    // Even if they didn't guess, they should still get the notification
                    // Create a dummy personal guess just for the notification
                    Some((*conn_id, PersonalGuess {
                        word: String::new(),
                        points_earned: 0,
                        was_winning_guess: false,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    }))
                }
            })
            .collect();
        
        // Return a special round completion event that includes the completed word
        Ok(GameEvent::RoundResult {
            winning_guess: GuessResult {
                word: round_completion.word,
                player_id: round_completion.player_id,
                letters: vec![], // Frontend will handle displaying the completed word
                points_earned: round_completion.points_earned,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
            player_guesses, // Now includes all players so everyone gets notified
            is_word_completed: true, // This is explicitly a word completion event
        })
    }

    pub async fn get_active_games_count(&self) -> usize {
        let games = self.active_games.read().await;
        games.len()
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self::new(Arc::new(ConnectionManager::new()), "./shared/words")
            .expect("Failed to load word directory for GameManager. Run './scripts/download_and_split_words.sh' to generate word lists.")
    }
}
