use std::sync::Arc;
use tracing::{error, info, warn};

use crate::auth::AuthService;
use crate::game_manager::GameManager;
use crate::matchmaking::MatchmakingQueue;
use crate::websocket::connection::{ConnectionId, ConnectionManager};
use game_types::{ClientMessage, ServerMessage};

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
        self.connection_manager
            .update_activity(self.connection_id)
            .await;

        match message {
            ClientMessage::Authenticate { token } => self.handle_authenticate(token).await,
            ClientMessage::ForceAuthenticate { token } => {
                self.handle_force_authenticate(token).await
            }
            ClientMessage::JoinQueue => self.handle_join_queue().await,
            ClientMessage::LeaveQueue => self.handle_leave_queue().await,
            ClientMessage::VoteStartGame => self.handle_vote_start_game().await,
            ClientMessage::SubmitGuess { word } => self.handle_submit_guess(word).await,
            ClientMessage::LeaveGame => self.handle_leave_game().await,
            ClientMessage::RejoinGame { game_id } => self.handle_rejoin_game(game_id).await,
            ClientMessage::Heartbeat => self.handle_heartbeat().await,
        }
    }

    pub async fn handle_disconnect(&self) {
        info!("Handling disconnect for connection {}", self.connection_id);

        // Remove from queue if present
        if let Err(e) = self
            .matchmaking_queue
            .remove_player(self.connection_id)
            .await
        {
            // It's okay if they weren't in queue
            info!(
                "Player {} not in queue during disconnect: {}",
                self.connection_id, e
            );
        }

        // Handle game disconnect if in a game
        if let Some(connection) = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
        {
            if let Some(game_id) = connection.game_id {
                if let Err(e) = self
                    .game_manager
                    .handle_player_disconnect(&game_id, self.connection_id)
                    .await
                {
                    error!(
                        "Failed to handle game disconnect for {}: {}",
                        self.connection_id, e
                    );
                }
            }
        }
    }

    async fn handle_authenticate(&self, token: String) -> Result<(), String> {
        info!("Authenticating connection {}", self.connection_id);

        match self.auth_service.validate_token(&token).await {
            Ok(user) => {
                // Check if user already has an active session
                if self
                    .connection_manager
                    .check_existing_session(&user.id.to_string())
                    .await
                {
                    // Send session conflict message
                    return self.send_message(ServerMessage::SessionConflict {
                        existing_connection: "You already have an active session in another browser.".to_string(),
                    }).await;
                }

                // Set user in connection
                self.connection_manager
                    .set_connection_user(self.connection_id, Some(user.clone()))
                    .await;
                self.send_message(ServerMessage::AuthenticationSuccess { user })
                    .await
            }
            Err(e) => {
                warn!(
                    "Authentication failed for connection {}: {}",
                    self.connection_id, e
                );
                self.send_message(ServerMessage::AuthenticationFailed {
                    reason: e.to_string(),
                })
                .await
            }
        }
    }

    async fn handle_force_authenticate(&self, token: String) -> Result<(), String> {
        info!("Force authenticating connection {}", self.connection_id);

        match self.auth_service.validate_token(&token).await {
            Ok(user) => {
                // Force disconnect existing session and authenticate this one
                match self
                    .connection_manager
                    .force_authenticate_connection(self.connection_id, user.id.to_string())
                    .await
                {
                    Ok(old_conn) => {
                        if old_conn.is_some() {
                            info!("Disconnected existing session for user {}", user.id);
                        }
                        // Set user in connection
                        self.connection_manager
                            .set_connection_user(self.connection_id, Some(user.clone()))
                            .await;
                        self.send_message(ServerMessage::AuthenticationSuccess { user })
                            .await
                    }
                    Err(e) => {
                        self.send_message(ServerMessage::AuthenticationFailed {
                            reason: e.to_string(),
                        })
                        .await
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Force authentication failed for connection {}: {}",
                    self.connection_id, e
                );
                self.send_message(ServerMessage::AuthenticationFailed {
                    reason: e.to_string(),
                })
                .await
            }
        }
    }

    async fn handle_join_queue(&self) -> Result<(), String> {
        info!("Player {} joining queue", self.connection_id);

        // Check if player is authenticated
        let connection = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
            .ok_or("Connection not found")?;

        if !connection.is_authenticated {
            return self
                .send_error("Authentication required to join queue")
                .await;
        }

        // Check if already in a game
        if let Some(connection) = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
        {
            if connection.game_id.is_some() {
                return self.send_error("Already in a game").await;
            }
        }

        // Add to queue
        match self.matchmaking_queue.add_player(self.connection_id).await {
            Ok(position) => {
                self.send_message(ServerMessage::QueueJoined { position })
                    .await?;

                // Broadcast countdown info to all players if countdown is active
                self.broadcast_countdown_to_queue().await;

                Ok(())
            }
            Err(e) => {
                self.send_error(&format!("Failed to join queue: {}", e))
                    .await
            }
        }
    }

    async fn handle_leave_queue(&self) -> Result<(), String> {
        info!("Player {} leaving queue", self.connection_id);

        match self
            .matchmaking_queue
            .remove_player(self.connection_id)
            .await
        {
            Ok(_) => self.send_message(ServerMessage::QueueLeft).await,
            Err(e) => {
                self.send_error(&format!("Failed to leave queue: {}", e))
                    .await
            }
        }
    }

    async fn handle_submit_guess(&self, word: String) -> Result<(), String> {
        info!("Player {} submitting guess: {}", self.connection_id, word);

        // Get connection to find game
        let connection = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
            .ok_or("Connection not found")?;

        let game_id = connection.game_id.ok_or("Not in a game")?;

        // Submit guess to game manager
        match self
            .game_manager
            .submit_guess(&game_id, self.connection_id, word)
            .await
        {
            Ok(game_event) => {
                // Handle the game event and send appropriate messages
                self.handle_game_event(&game_id, game_event).await
            }
            Err(e) => self.send_error(&format!("Invalid guess: {}", e)).await,
        }
    }

    async fn handle_leave_game(&self) -> Result<(), String> {
        info!("Player {} leaving game", self.connection_id);

        let connection = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
            .ok_or("Connection not found")?;

        if let Some(game_id) = connection.game_id {
            match self
                .game_manager
                .remove_player(&game_id, self.connection_id)
                .await
            {
                Ok(_) => {
                    self.connection_manager
                        .set_connection_game(self.connection_id, None)
                        .await;
                    self.send_message(ServerMessage::GameLeft).await
                }
                Err(e) => {
                    self.send_error(&format!("Failed to leave game: {}", e))
                        .await
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

    async fn handle_vote_start_game(&self) -> Result<(), String> {
        info!("Player {} voting to start game", self.connection_id);

        // Check if player is authenticated
        let connection = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
            .ok_or("Connection not found")?;

        if !connection.is_authenticated {
            return self.send_error("Authentication required to vote").await;
        }

        // Check if player is in queue and vote
        match self
            .matchmaking_queue
            .vote_to_start(self.connection_id)
            .await
        {
            Ok(has_enough_votes) => {
                // Broadcast updated countdown info to all players in queue
                self.broadcast_countdown_to_queue().await;

                // Check if we should start the match immediately
                if has_enough_votes || self.matchmaking_queue.should_start_match().await {
                    self.create_match_from_queue().await?;
                }

                Ok(())
            }
            Err(e) => self.send_error(&format!("Failed to vote: {}", e)).await,
        }
    }

    async fn handle_rejoin_game(&self, game_id: String) -> Result<(), String> {
        info!(
            "Player {} attempting to rejoin game {}",
            self.connection_id, game_id
        );

        // Check if player is authenticated
        let connection = self
            .connection_manager
            .get_connection(self.connection_id)
            .await
            .ok_or("Connection not found")?;

        if !connection.is_authenticated {
            return self
                .send_error("Authentication required to rejoin game")
                .await;
        }

        // Check if the game exists and if the player was originally in it
        match self
            .game_manager
            .rejoin_player(&game_id, self.connection_id)
            .await
        {
            Ok(current_state) => {
                // Set the game ID in the connection
                self.connection_manager
                    .set_connection_game(self.connection_id, Some(game_id.clone()))
                    .await;

                // Send personalized game state to the rejoining player
                if let Some(ref user) = connection.user {
                    let personalized_state = current_state.personalized_for_player(user.id.clone());
                    self.send_message(ServerMessage::GameStateUpdate {
                        state: personalized_state,
                    })
                    .await?;
                }

                // Notify other players that this player has reconnected
                if let Some(ref user) = connection.user {
                    self.connection_manager
                        .send_to_game_except(
                            &game_id,
                            self.connection_id,
                            ServerMessage::PlayerReconnected {
                                player_id: user.id.clone(),
                            },
                        )
                        .await;
                }

                info!(
                    "Player {} successfully rejoined game {}",
                    self.connection_id, game_id
                );
                Ok(())
            }
            Err(e) => {
                self.send_error(&format!("Failed to rejoin game: {}", e))
                    .await
            }
        }
    }

    async fn handle_game_event(
        &self,
        game_id: &str,
        event: crate::game_manager::GameEvent,
    ) -> Result<(), String> {
        use crate::game_manager::GameEvent;

        match event {
            GameEvent::RoundResult {
                winning_guess,
                player_guesses,
                is_word_completed,
            } => {
                // Get the current game state to determine the next phase
                let next_phase =
                    if let Some(game_state) = self.game_manager.get_game_state(game_id).await {
                        game_state.current_phase
                    } else {
                        game_types::GamePhase::Guessing // Fallback
                    };

                // Send winning guess to all players
                let message = ServerMessage::RoundResult {
                    winning_guess: winning_guess.clone(),
                    your_guess: None, // Will be set per player
                    next_phase,
                    is_word_completed, // Use the flag from the game event
                };

                // Send personalized messages to each player
                for (player_id, personal_guess) in player_guesses {
                    let mut personal_message = message.clone();
                    if let ServerMessage::RoundResult {
                        ref mut your_guess, ..
                    } = personal_message
                    {
                        *your_guess = Some(personal_guess);
                    }

                    if let Err(e) = self
                        .connection_manager
                        .send_to_connection(player_id, personal_message)
                        .await
                    {
                        warn!("Failed to send round result to {}: {}", player_id, e);
                    }
                }

                // After sending round results, send personalized game state updates
                if let Some(updated_state) = self.game_manager.get_game_state(game_id).await {
                    tracing::info!(
                        "Sending GameStateUpdate after RoundResult - phase: {:?}, current_winner: {:?}",
                        updated_state.current_phase,
                        updated_state.current_winner
                    );
                    self.connection_manager
                        .send_personalized_game_state(game_id, &updated_state)
                        .await;
                }
            }
            GameEvent::GameOver {
                winner,
                final_scores,
            } => {
                tracing::info!(
                    "🏆 Game {} completed! Winner: {} ({} points) | Final standings: {:?}",
                    game_id,
                    winner.display_name,
                    winner.points,
                    final_scores
                        .iter()
                        .map(|p| format!("{}: {}", p.display_name, p.points))
                        .collect::<Vec<_>>()
                );

                let message = ServerMessage::GameOver {
                    winner: winner.clone(),
                    final_scores: final_scores.clone(),
                };
                self.connection_manager.send_to_game(game_id, message).await;

                // Clear game from all connections
                let connections = self
                    .connection_manager
                    .get_connections_in_game(game_id)
                    .await;
                for connection_id in connections {
                    self.connection_manager
                        .set_connection_game(connection_id, None)
                        .await;
                }
            }
            GameEvent::StateUpdate { state } => {
                self.connection_manager
                    .send_personalized_game_state(game_id, &state)
                    .await;
            }
        }

        Ok(())
    }

    async fn send_message(&self, message: ServerMessage) -> Result<(), String> {
        self.connection_manager
            .send_to_connection(self.connection_id, message)
            .await
    }

    async fn send_error(&self, error_message: &str) -> Result<(), String> {
        self.send_message(ServerMessage::Error {
            message: error_message.to_string(),
        })
        .await
    }

    // Helper method to broadcast countdown info to all players in queue
    async fn broadcast_countdown_to_queue(&self) {
        if let Some(countdown_info) = self.matchmaking_queue.get_countdown_info().await {
            let message = ServerMessage::MatchmakingCountdown {
                seconds_remaining: countdown_info.seconds_remaining,
                players_ready: countdown_info.players_ready,
                total_players: countdown_info.total_players,
            };

            // Get all players in queue and broadcast to each
            let queue_players = self.matchmaking_queue.get_queue_players().await;
            info!(
                "Broadcasting countdown to {} players in queue",
                queue_players.len()
            );

            for player_id in queue_players {
                if let Err(e) = self
                    .connection_manager
                    .send_to_connection(player_id, message.clone())
                    .await
                {
                    warn!("Failed to send countdown update to {}: {}", player_id, e);
                }
            }
        }
    }

    // Helper method to create a match from the current queue
    async fn create_match_from_queue(&self) -> Result<(), String> {
        if let Ok(Some(match_info)) = self.matchmaking_queue.try_create_match().await {
            info!("Creating match with {} players", match_info.players.len());

            // Get player info for the match
            let mut players_info = Vec::new();
            for &player_id in &match_info.players {
                if let Some(connection) = self.connection_manager.get_connection(player_id).await {
                    if let Some(ref user) = connection.user {
                        players_info.push(game_types::Player {
                            user_id: user.id.clone(),
                            display_name: user.display_name.clone(),
                            points: 0,
                            guess_history: Vec::new(),
                            is_connected: true,
                        });
                    }
                }
            }

            // Create game
            match self
                .game_manager
                .create_game(match_info.players.clone())
                .await
            {
                Ok(game_id) => {
                    // Get initial game state
                    let initial_game_state = self.game_manager.get_game_state(&game_id).await;

                    // Notify all players of match and send initial game state
                    for &player_id in &match_info.players {
                        self.connection_manager
                            .set_connection_game(player_id, Some(game_id.clone()))
                            .await;

                        // Send MatchFound message
                        if let Err(e) = self
                            .connection_manager
                            .send_to_connection(
                                player_id,
                                ServerMessage::MatchFound {
                                    game_id: game_id.clone(),
                                    players: players_info.clone(),
                                },
                            )
                            .await
                        {
                            warn!("Failed to notify player {} of match: {}", player_id, e);
                        }

                        // Send personalized initial game state
                        if let Some(ref game_state) = initial_game_state {
                            if let Some(connection) =
                                self.connection_manager.get_connection(player_id).await
                            {
                                if let Some(ref user) = connection.user {
                                    let personalized_state =
                                        game_state.personalized_for_player(user.id.clone());
                                    if let Err(e) = self
                                        .connection_manager
                                        .send_to_connection(
                                            player_id,
                                            ServerMessage::GameStateUpdate {
                                                state: personalized_state,
                                            },
                                        )
                                        .await
                                    {
                                        warn!(
                                            "Failed to send initial game state to {}: {}",
                                            player_id, e
                                        );
                                    }
                                }
                            }
                        }
                    }

                    info!(
                        "Successfully created match {} with {} players and sent initial state",
                        game_id,
                        match_info.players.len()
                    );
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to create game: {}", e);
                    // Put players back in queue
                    for &player_id in &match_info.players {
                        let _ = self.matchmaking_queue.add_player(player_id).await;
                    }
                    Err(format!("Failed to create game: {}", e))
                }
            }
        } else {
            Ok(()) // No match to create
        }
    }
}
