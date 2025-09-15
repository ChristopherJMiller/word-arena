use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use game_core::{Game, WordValidator, PlayerId};
use game_types::{GamePhase, GameState, SafeGameState, GuessResult, PersonalGuess, Player};
use crate::websocket::connection::ConnectionId;

#[derive(Debug, Clone)]
pub enum GameEvent {
    RoundResult {
        winning_guess: GuessResult,
        player_guesses: Vec<(ConnectionId, PersonalGuess)>,
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
    fn new(id: String, players: Vec<ConnectionId>) -> Self {
        let mut connection_to_player = HashMap::new();
        let mut player_to_connection = HashMap::new();
        let mut game_players = Vec::new();
        
        for (i, connection_id) in players.iter().enumerate() {
            let player_id = Uuid::new_v4();
            let player = Player {
                user_id: player_id,
                display_name: format!("Player {}", i + 1),
                points: 0,
                guess_history: Vec::new(),
                is_connected: true,
            };
            
            game_players.push(player);
            connection_to_player.insert(*connection_id, player_id);
            player_to_connection.insert(player_id, *connection_id);
        }
        
        // Create word validator to get a random word
        let word_list = include_str!("../../shared/words/word_list.txt");
        let word_validator = WordValidator::new(word_list);
        let target_word = word_validator.get_random_word(5).expect("Failed to get random word");
        
        let game = Game::new(
            Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4()),
            game_players,
            target_word,
            25, // Points to win from config
        );
        
        let now = Instant::now();
        Self {
            id,
            game,
            connection_to_player,
            player_to_connection,
            created_at: now,
            last_activity: now,
        }
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
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            active_games: RwLock::new(HashMap::new()),
            connection_to_game: RwLock::new(HashMap::new()),
            word_validator: Arc::new(WordValidator::new(include_str!("../../shared/words/word_list.txt"))),
        }
    }
    
    pub async fn create_game(&self, players: Vec<ConnectionId>) -> Result<String, String> {
        if players.len() < 2 {
            return Err("Need at least 2 players to create a game".to_string());
        }
        
        let game_id = Uuid::new_v4().to_string();
        let active_game = ActiveGame::new(game_id.clone(), players.clone());
        
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
        
        info!("Created game {} with {} players", game_id, players.len());
        Ok(game_id)
    }
    
    pub async fn submit_guess(
        &self,
        game_id: &str,
        connection_id: ConnectionId,
        word: String,
    ) -> Result<GameEvent, String> {
        let mut games = self.active_games.write().await;
        let active_game = games.get_mut(game_id)
            .ok_or("Game not found")?;
        
        active_game.update_activity();
        
        let player_id = active_game.connection_to_player.get(&connection_id)
            .ok_or("Player not in game")?
            .clone();
        
        // Validate word
        if !self.word_validator.is_valid_word(&word) {
            return Err("Invalid word".to_string());
        }
        
        // Add guess to game
        if let Err(e) = active_game.game.add_guess(player_id, word.clone()) {
            return Err(format!("Failed to add guess: {:?}", e));
        }
        
        // Try to process the round (this happens when all players have guessed or timeout occurs)
        match active_game.game.process_round() {
            Ok(Some(winning_guess)) => {
                // The winning_guess is already in the correct format from game-core
                // Create personal guess results for each player
                let player_guesses: Vec<(ConnectionId, PersonalGuess)> = active_game.game.state.players.iter()
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
                
                // Check if game is over
                if active_game.game.current_phase == GamePhase::GameOver {
                    // Find the winner
                    if let Some(winner) = active_game.game.state.players.iter()
                        .max_by_key(|p| p.points) {
                        
                        let final_scores = active_game.game.state.players.clone();
                        
                        Ok(GameEvent::GameOver {
                            winner: winner.clone(),
                            final_scores,
                        })
                    } else {
                        Ok(GameEvent::RoundResult {
                            winning_guess,
                            player_guesses,
                        })
                    }
                } else {
                    Ok(GameEvent::RoundResult {
                        winning_guess,
                        player_guesses,
                    })
                }
            },
            Ok(None) => {
                // No round result yet (waiting for more guesses)
                Ok(GameEvent::StateUpdate {
                    state: active_game.game.state.clone(),
                })
            },
            Err(e) => Err(format!("Game error: {:?}", e)),
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
    
    pub async fn handle_player_disconnect(&self, game_id: &str, connection_id: ConnectionId) -> Result<(), String> {
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
    
    pub async fn rejoin_player(&self, game_id: &str, connection_id: ConnectionId) -> Result<GameState, String> {
        let mut games = self.active_games.write().await;
        let active_game = games.get_mut(game_id)
            .ok_or("Game not found")?;
        
        // For now, let's check if this connection was previously in the game
        // In a more robust system, we'd verify the user ID matches a disconnected player
        
        // Find a disconnected player to reconnect to
        let disconnected_player = active_game.game.state.players.iter()
            .find(|p| !p.is_connected)
            .ok_or("No disconnected players to rejoin")?
            .clone();
        
        // Update connection mappings
        active_game.connection_to_player.insert(connection_id, disconnected_player.user_id);
        active_game.player_to_connection.insert(disconnected_player.user_id, connection_id);
        
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

    pub async fn remove_player(&self, game_id: &str, connection_id: ConnectionId) -> Result<(), String> {
        {
            let mut games = self.active_games.write().await;
            if let Some(active_game) = games.get_mut(game_id) {
                if let Some(player_id) = active_game.connection_to_player.remove(&connection_id) {
                    active_game.player_to_connection.remove(&player_id);
                    
                    // Remove player from game state
                    active_game.game.state.players.retain(|p| p.user_id != player_id);
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
                if game.is_expired(timeout) || game.game.state.players.iter().all(|p| !p.is_connected) {
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
    
    pub async fn get_active_games_count(&self) -> usize {
        let games = self.active_games.read().await;
        games.len()
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self::new()
    }
}