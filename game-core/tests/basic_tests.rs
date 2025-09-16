mod common;

use common::*;
use game_types::{GamePhase, GameStatus};

#[test]
fn test_game_creation() {
    let game = create_standard_game();
    assert_eq!(game.state.players.len(), 2);
    assert_eq!(game.state.status, GameStatus::Starting);
    assert_eq!(game.current_phase, GamePhase::Waiting);
}

#[test]
fn test_word_validator() {
    let validator = create_test_validator();
    assert!(validator.is_valid_word("tests"));
    assert!(validator.is_valid_word("hello"));
    assert!(!validator.is_valid_word("invalid"));
}

#[test]
fn test_game_manager_creation() {
    let manager = create_test_manager();
    assert_eq!(manager.active_games.len(), 0);
}

#[test]
fn test_player_creation() {
    let player = create_test_player("TestPlayer");
    assert_eq!(player.display_name, "TestPlayer");
    assert_eq!(player.points, 0);
    assert!(player.is_connected);
}