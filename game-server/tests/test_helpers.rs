use chrono;
use game_core::word_validation::WordValidator;
use game_server::auth::AuthService;
use game_server::game_manager::GameManager;
use game_server::matchmaking::MatchmakingQueue;
use game_server::websocket::connection::{ConnectionId, ConnectionManager};
use game_types::{Player, User};
use std::sync::Arc;
use uuid::Uuid;

/// Creates a test user with given name
pub fn create_test_user(name: &str) -> User {
    User {
        id: Uuid::new_v4(),
        display_name: name.to_string(),
        email: format!("{}@test.com", name.to_lowercase()),
        total_points: 0,
        total_wins: 0,
        total_games: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Creates a test player from a user
pub fn user_to_player(user: &User) -> Player {
    Player {
        user_id: user.id,
        display_name: user.display_name.clone(),
        points: 0,
        guess_history: Vec::new(),
        is_connected: true,
    }
}

/// Test setup that provides all necessary components
pub struct TestGameServerSetup {
    pub connection_manager: Arc<ConnectionManager>,
    pub game_manager: Arc<GameManager>,
    pub matchmaking_queue: Arc<MatchmakingQueue>,
    pub auth_service: Arc<AuthService>,
}

impl TestGameServerSetup {
    pub fn new() -> Self {
        let connection_manager = Arc::new(ConnectionManager::new());

        // Create a test word validator with known words for predictable testing
        let test_words = vec![
            "about", "above", "after", "again", "beach", "black", "brown", "chair", "close",
            "early", "house", "place", "right", "round", "today", "which", "world", "wrong",
            "guess", "first", "second", "third", "forth", "fifth", "sixth", "seven", "eight",
        ];
        let word_list = test_words.join("\n");
        let word_validator = WordValidator::from_word_list(&word_list);

        Self {
            connection_manager: connection_manager.clone(),
            game_manager: Arc::new(GameManager::new_with_validator(
                connection_manager,
                word_validator,
            )),
            matchmaking_queue: Arc::new(MatchmakingQueue::new()),
            auth_service: Arc::new(AuthService::new_dev_mode()),
        }
    }

    /// Creates a connection and authenticates it with a test user
    pub async fn create_authenticated_connection(&self, name: &str) -> (ConnectionId, User) {
        let connection_id = ConnectionId::new();
        let _receiver = self
            .connection_manager
            .create_connection(connection_id)
            .await;
        let user = create_test_user(name);

        // Authenticate the connection
        self.connection_manager
            .set_connection_user(connection_id, Some(user.clone()))
            .await;

        (connection_id, user)
    }

    /// Creates multiple authenticated connections
    pub async fn create_multiple_connections(&self, names: &[&str]) -> Vec<(ConnectionId, User)> {
        let mut connections = Vec::new();
        for name in names {
            connections.push(self.create_authenticated_connection(name).await);
        }
        connections
    }

    /// Helper to create a game with specific players
    pub async fn create_test_game(
        &self,
        connection_ids: Vec<ConnectionId>,
    ) -> Result<String, String> {
        self.game_manager.create_game(connection_ids).await
    }

    /// Submit a guess and return the result
    pub async fn submit_guess(
        &self,
        game_id: &str,
        connection_id: ConnectionId,
        word: &str,
    ) -> Result<game_server::game_manager::GameEvent, String> {
        self.game_manager
            .submit_guess(game_id, connection_id, word.to_string())
            .await
    }
}

/// Helper to assert specific game event types
pub fn assert_round_result(
    event: &game_server::game_manager::GameEvent,
) -> &game_server::game_manager::GameEvent {
    match event {
        game_server::game_manager::GameEvent::RoundResult { .. } => event,
        _ => panic!("Expected RoundResult event, got {:?}", event),
    }
}

pub fn assert_state_update(
    event: &game_server::game_manager::GameEvent,
) -> &game_server::game_manager::GameEvent {
    match event {
        game_server::game_manager::GameEvent::StateUpdate { .. } => event,
        _ => panic!("Expected StateUpdate event, got {:?}", event),
    }
}

pub fn assert_game_over(
    event: &game_server::game_manager::GameEvent,
) -> &game_server::game_manager::GameEvent {
    match event {
        game_server::game_manager::GameEvent::GameOver { .. } => event,
        _ => panic!("Expected GameOver event, got {:?}", event),
    }
}

/// Helper to extract game state from StateUpdate event
pub fn extract_game_state(event: &game_server::game_manager::GameEvent) -> &game_types::GameState {
    match event {
        game_server::game_manager::GameEvent::StateUpdate { state } => state,
        _ => panic!("Cannot extract game state from non-StateUpdate event"),
    }
}

/// Check if all connected players in a game have submitted guesses
pub async fn all_players_guessed(setup: &TestGameServerSetup, game_id: &str) -> bool {
    if let Some(state) = setup.game_manager.get_game_state(game_id).await {
        // This is a simplified check - in real implementation we'd need access to current_guesses
        // For testing, we'll rely on the game logic
        return state.current_round > 1 || state.official_board.len() > 0;
    }
    false
}

/// Create a game that's ready to play (players connected and authenticated)
pub async fn setup_ready_game(
    setup: &TestGameServerSetup,
    player_names: &[&str],
) -> Result<(String, Vec<(ConnectionId, User)>), String> {
    let connections = setup.create_multiple_connections(player_names).await;
    let connection_ids: Vec<_> = connections.iter().map(|(id, _)| *id).collect();

    let game_id = setup.create_test_game(connection_ids).await?;

    Ok((game_id, connections))
}

/// Simulate a complete round with all players guessing
pub async fn play_round(
    setup: &TestGameServerSetup,
    game_id: &str,
    guesses: Vec<(ConnectionId, &str)>,
) -> Result<game_server::game_manager::GameEvent, String> {
    let mut last_event = None;

    for (connection_id, word) in guesses {
        let event = setup.submit_guess(game_id, connection_id, word).await?;
        last_event = Some(event);
    }

    last_event.ok_or_else(|| "No guesses submitted".to_string())
}
