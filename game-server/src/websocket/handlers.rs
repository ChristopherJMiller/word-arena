use std::sync::Arc;
use tracing::{info, warn, error};

use game_types::{ClientMessage, ServerMessage};
use crate::auth::AuthService;
use crate::websocket::connection::{ConnectionManager, ConnectionId};
use crate::game_manager::GameManager;
use crate::matchmaking::MatchmakingQueue;

#[derive(Clone)]
pub struct MessageHandler {
    connection_id: ConnectionId,
    connection_manager: Arc<ConnectionManager>,
    game_manager: Arc<GameManager>,
    matchmaking_queue: Arc<MatchmakingQueue>,
    auth_service: Arc<AuthService>,
}

impl MessageHandler {
    pub fn new(
        connection_id: ConnectionId,
        connection_manager: Arc<ConnectionManager>,
        game_manager: Arc<GameManager>,
        matchmaking_queue: Arc<MatchmakingQueue>,
        auth_service: Arc<AuthService>,
    ) -> Self {
        Self {
            connection_id,
            connection_manager,
            game_manager,
            matchmaking_queue,
            auth_service,
        }
    }
    
    pub async fn handle_message(&self, message: ClientMessage) -> Result<(), String> {
        // Update connection activity
        self.connection_manager.update_activity(self.connection_id).await;
        
        match message {
            ClientMessage::Authenticate { token } => {
                self.handle_authenticate(token).await
            },
            ClientMessage::JoinQueue => {
                self.handle_join_queue().await
            },
            ClientMessage::LeaveQueue => {
                self.handle_leave_queue().await
            },
            ClientMessage::SubmitGuess { word } => {
                self.handle_submit_guess(word).await
            },
            ClientMessage::LeaveGame => {
                self.handle_leave_game().await
            },
            ClientMessage::RejoinGame { game_id } => {
                self.handle_rejoin_game(game_id).await
            },
            ClientMessage::Heartbeat => {
                self.handle_heartbeat().await
            },
        }
    }
    
    pub async fn handle_disconnect(&self) {
        info!("Handling disconnect for connection {}", self.connection_id);
        
        // Remove from queue if present
        if let Err(e) = self.matchmaking_queue.remove_player(self.connection_id).await {
            // It's okay if they weren't in queue
            info!("Player {} not in queue during disconnect: {}", self.connection_id, e);
        }
        
        // Handle game disconnect if in a game
        if let Some(connection) = self.connection_manager.get_connection(self.connection_id).await {
            if let Some(game_id) = connection.game_id {
                if let Err(e) = self.game_manager.handle_player_disconnect(&game_id, self.connection_id).await {
                    error!("Failed to handle game disconnect for {}: {}", self.connection_id, e);
                }
            }
        }
    }
    
    async fn handle_authenticate(&self, token: String) -> Result<(), String> {
        info!("Authenticating connection {}", self.connection_id);
        
        match self.auth_service.validate_token(&token).await {
            Ok(user) => {
                // Set user in connection
                self.connection_manager.set_connection_user(self.connection_id, Some(user.clone())).await;
                self.send_message(ServerMessage::AuthenticationSuccess { user }).await
            },
            Err(e) => {
                warn!("Authentication failed for connection {}: {}", self.connection_id, e);
                self.send_message(ServerMessage::AuthenticationFailed { 
                    reason: e.to_string() 
                }).await
            }
        }
    }
    
    async fn handle_join_queue(&self) -> Result<(), String> {
        info!("Player {} joining queue", self.connection_id);
        
        // Check if player is authenticated
        let connection = self.connection_manager.get_connection(self.connection_id).await
            .ok_or("Connection not found")?;
        
        if !connection.is_authenticated {
            return self.send_error("Authentication required to join queue").await;
        }
        
        // Check if already in a game
        if let Some(connection) = self.connection_manager.get_connection(self.connection_id).await {
            if connection.game_id.is_some() {
                return self.send_error("Already in a game").await;
            }
        }
        
        // Add to queue
        match self.matchmaking_queue.add_player(self.connection_id).await {
            Ok(position) => {
                self.send_message(ServerMessage::QueueJoined { position }).await?;
                
                // Try to create a match
                if let Ok(Some(match_info)) = self.matchmaking_queue.try_create_match().await {
                    info!("Created match with {} players", match_info.players.len());
                    
                    // Create game
                    match self.game_manager.create_game(match_info.players.clone()).await {
                        Ok(game_id) => {
                            // Notify all players
                            for &player_id in &match_info.players {
                                self.connection_manager.set_connection_game(player_id, Some(game_id.clone())).await;
                                
                                if let Err(e) = self.connection_manager.send_to_connection(
                                    player_id,
                                    ServerMessage::MatchFound {
                                        game_id: game_id.clone(),
                                        players: Vec::new(), // TODO: Get player info
                                    }
                                ).await {
                                    warn!("Failed to notify player {} of match: {}", player_id, e);
                                }
                            }
                        },
                        Err(e) => {
                            error!("Failed to create game: {}", e);
                            // Put players back in queue
                            for &player_id in &match_info.players {
                                let _ = self.matchmaking_queue.add_player(player_id).await;
                            }
                        }
                    }
                }
                
                Ok(())
            },
            Err(e) => {
                self.send_error(&format!("Failed to join queue: {}", e)).await
            }
        }
    }
    
    async fn handle_leave_queue(&self) -> Result<(), String> {
        info!("Player {} leaving queue", self.connection_id);
        
        match self.matchmaking_queue.remove_player(self.connection_id).await {
            Ok(_) => {
                self.send_message(ServerMessage::QueueLeft).await
            },
            Err(e) => {
                self.send_error(&format!("Failed to leave queue: {}", e)).await
            }
        }
    }
    
    async fn handle_submit_guess(&self, word: String) -> Result<(), String> {
        info!("Player {} submitting guess: {}", self.connection_id, word);
        
        // Get connection to find game
        let connection = self.connection_manager.get_connection(self.connection_id).await
            .ok_or("Connection not found")?;
        
        let game_id = connection.game_id
            .ok_or("Not in a game")?;
        
        // Submit guess to game manager
        match self.game_manager.submit_guess(&game_id, self.connection_id, word).await {
            Ok(game_event) => {
                // Handle the game event and send appropriate messages
                self.handle_game_event(&game_id, game_event).await
            },
            Err(e) => {
                self.send_error(&format!("Invalid guess: {}", e)).await
            }
        }
    }
    
    async fn handle_leave_game(&self) -> Result<(), String> {
        info!("Player {} leaving game", self.connection_id);
        
        let connection = self.connection_manager.get_connection(self.connection_id).await
            .ok_or("Connection not found")?;
        
        if let Some(game_id) = connection.game_id {
            match self.game_manager.remove_player(&game_id, self.connection_id).await {
                Ok(_) => {
                    self.connection_manager.set_connection_game(self.connection_id, None).await;
                    self.send_message(ServerMessage::GameLeft).await
                },
                Err(e) => {
                    self.send_error(&format!("Failed to leave game: {}", e)).await
                }
            }
        } else {
            self.send_error("Not in a game").await
        }
    }
    
    async fn handle_heartbeat(&self) -> Result<(), String> {
        // Heartbeat just updates activity (already done in handle_message)
        Ok(())
    }
    
    async fn handle_rejoin_game(&self, game_id: String) -> Result<(), String> {
        info!("Player {} attempting to rejoin game {}", self.connection_id, game_id);
        
        // Check if player is authenticated
        let connection = self.connection_manager.get_connection(self.connection_id).await
            .ok_or("Connection not found")?;
        
        if !connection.is_authenticated {
            return self.send_error("Authentication required to rejoin game").await;
        }
        
        // Check if the game exists and if the player was originally in it
        match self.game_manager.rejoin_player(&game_id, self.connection_id).await {
            Ok(current_state) => {
                // Set the game ID in the connection
                self.connection_manager.set_connection_game(self.connection_id, Some(game_id.clone())).await;
                
                // Send current game state to the rejoining player
                self.send_message(ServerMessage::GameStateUpdate { state: current_state }).await?;
                
                // Notify other players that this player has reconnected
                if let Some(ref user) = connection.user {
                    self.connection_manager.send_to_game_except(
                        &game_id, 
                        self.connection_id,
                        ServerMessage::PlayerReconnected { player_id: user.id }
                    ).await;
                }
                
                info!("Player {} successfully rejoined game {}", self.connection_id, game_id);
                Ok(())
            },
            Err(e) => {
                self.send_error(&format!("Failed to rejoin game: {}", e)).await
            }
        }
    }
    
    async fn handle_game_event(&self, game_id: &str, event: crate::game_manager::GameEvent) -> Result<(), String> {
        use crate::game_manager::GameEvent;
        
        match event {
            GameEvent::RoundResult { winning_guess, player_guesses } => {
                // Send winning guess to all players
                let message = ServerMessage::RoundResult {
                    winning_guess: winning_guess.clone(),
                    your_guess: None, // Will be set per player
                    next_phase: game_types::GamePhase::Guessing, // TODO: Determine correct phase
                };
                
                // Send personalized messages to each player
                for (player_id, personal_guess) in player_guesses {
                    let mut personal_message = message.clone();
                    if let ServerMessage::RoundResult { ref mut your_guess, .. } = personal_message {
                        *your_guess = Some(personal_guess);
                    }
                    
                    if let Err(e) = self.connection_manager.send_to_connection(player_id, personal_message).await {
                        warn!("Failed to send round result to {}: {}", player_id, e);
                    }
                }
            },
            GameEvent::GameOver { winner, final_scores } => {
                let message = ServerMessage::GameOver { winner, final_scores };
                self.connection_manager.send_to_game(game_id, message).await;
                
                // Clear game from all connections
                let connections = self.connection_manager.get_connections_in_game(game_id).await;
                for connection_id in connections {
                    self.connection_manager.set_connection_game(connection_id, None).await;
                }
            },
            GameEvent::StateUpdate { state } => {
                let message = ServerMessage::GameStateUpdate { state };
                self.connection_manager.send_to_game(game_id, message).await;
            },
        }
        
        Ok(())
    }
    
    async fn send_message(&self, message: ServerMessage) -> Result<(), String> {
        self.connection_manager.send_to_connection(self.connection_id, message).await
    }
    
    async fn send_error(&self, error_message: &str) -> Result<(), String> {
        self.send_message(ServerMessage::Error { 
            message: error_message.to_string() 
        }).await
    }
}