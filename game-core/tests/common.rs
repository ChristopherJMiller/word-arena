use game_core::{Game, GameEvent, GameEventHandler, GameManager, WordValidator};
use game_types::{GameId, GamePhase, GameStatus, Player, PlayerId};
use std::sync::{Arc, Mutex};

/// Creates a test WordValidator with a known set of words
pub fn create_test_validator() -> WordValidator {
    let word_list = "apple\nbanana\ncherry\ntests\nvalid\nhello\nworld\nhouse\nmouse\ntrain\nplane\nwater\nstone\nbread\ncream";
    WordValidator::from_word_list(word_list)
}

/// Creates a test player with specified attributes
pub fn create_test_player(name: &str) -> Player {
    create_test_player_with_points(name, 0)
}

/// Creates a test player with specified points
pub fn create_test_player_with_points(name: &str, points: i32) -> Player {
    Player {
        user_id: format!("test-player-{}", name.to_lowercase()),
        display_name: name.to_string(),
        points,
        guess_history: Vec::new(),
        is_connected: true,
    }
}

/// Creates a game with a specific target word
pub fn create_game_with_word(players: Vec<Player>, word: &str, threshold: i32) -> Game {
    let game_id = uuid::Uuid::new_v4().to_string();
    Game::new(game_id, players, word.to_string(), threshold)
}

/// Creates a standard test game with 2 players
pub fn create_standard_game() -> Game {
    let players = vec![create_test_player("Alice"), create_test_player("Bob")];
    create_game_with_word(players, "tests", 25)
}

/// Creates a game manager with test word list
pub fn create_test_manager() -> GameManager {
    GameManager::new(create_test_validator())
}

/// Event collector for testing event emissions
#[derive(Clone)]
pub struct EventCollector {
    events: Arc<Mutex<Vec<GameEvent>>>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_events(&self) -> Vec<GameEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    pub fn last_event(&self) -> Option<GameEvent> {
        self.events.lock().unwrap().last().cloned()
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn has_event_type(&self, check_fn: impl Fn(&GameEvent) -> bool) -> bool {
        self.events.lock().unwrap().iter().any(check_fn)
    }
}

impl GameEventHandler for EventCollector {
    fn handle_event(&mut self, event: GameEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Helper to advance game to a specific phase
pub fn advance_to_phase(game: &mut Game, target_phase: GamePhase) {
    match target_phase {
        GamePhase::Waiting => {
            // Already in waiting by default
        }
        GamePhase::Countdown => {
            game.set_countdown(std::time::Duration::from_secs(5));
        }
        GamePhase::Guessing => {
            game.start_guessing_phase();
        }
        GamePhase::IndividualGuess => {
            // Need to process a round first
            if let Some(player) = game.state.players.first() {
                let player_id = player.user_id.clone();
                game.add_guess(&player_id, "hello".to_string()).ok();
                game.process_round().ok();
            }
        }
        GamePhase::GameOver => {
            game.state.status = GameStatus::Completed;
            game.current_phase = GamePhase::GameOver;
            game.state.current_phase = GamePhase::GameOver;
        }
    }
}

/// Helper to simulate multiple players submitting guesses
pub fn submit_guesses(game: &mut Game, guesses: Vec<(&str, &str)>) {
    for (player_name, word) in guesses {
        if let Some(player) = game
            .state
            .players
            .iter()
            .find(|p| p.display_name == player_name)
        {
            let player_id = player.user_id.clone();
            game.add_guess(&player_id, word.to_string()).ok();
        }
    }
}

/// Asserts that a game is in a specific state
pub fn assert_game_state(game: &Game, expected_status: GameStatus, expected_phase: GamePhase) {
    assert_eq!(
        game.state.status, expected_status,
        "Expected status {:?}, got {:?}",
        expected_status, game.state.status
    );
    assert_eq!(
        game.current_phase, expected_phase,
        "Expected phase {:?}, got {:?}",
        expected_phase, game.current_phase
    );
}

/// Creates a game that's one guess away from completion by points
pub fn create_near_win_game() -> Game {
    let players = vec![
        create_test_player_with_points("Alice", 23),
        create_test_player_with_points("Bob", 20),
    ];

    let mut game = create_game_with_word(players.clone(), "tests", 25);

    // Update the game's internal player state
    game.state.players[0].points = 23;
    game.state.players[1].points = 20;
    game.state.status = GameStatus::Active;
    game.start_guessing_phase();

    game
}

/// Creates a game with many players for testing scaling
pub fn create_multiplayer_game(player_count: usize) -> Game {
    let players: Vec<Player> = (0..player_count)
        .map(|i| create_test_player(&format!("Player{}", i + 1)))
        .collect();

    create_game_with_word(players, "tests", 25)
}

/// Helper to verify guess result
pub fn assert_guess_result(
    result: &Option<game_types::GuessResult>,
    expected_word: &str,
    expected_points: i32,
) {
    assert!(result.is_some(), "Expected a guess result");
    let guess = result.as_ref().unwrap();
    assert_eq!(guess.word, expected_word);
    assert_eq!(guess.points_earned, expected_points);
}

/// Helper to get player by name
pub fn get_player_by_name<'a>(game: &'a Game, name: &str) -> Option<&'a Player> {
    game.state.players.iter().find(|p| p.display_name == name)
}

/// Helper to get mutable player by name  
pub fn get_player_by_name_mut<'a>(game: &'a mut Game, name: &str) -> Option<&'a mut Player> {
    game.state
        .players
        .iter_mut()
        .find(|p| p.display_name == name)
}
