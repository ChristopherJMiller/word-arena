use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::user::Player;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GameState {
    pub id: Uuid,
    pub word: String,
    pub word_length: i32,
    pub current_round: i32,
    pub status: GameStatus,
    pub current_phase: GamePhase,
    pub players: Vec<Player>,
    pub official_board: Vec<GuessResult>,
    pub current_winner: Option<Uuid>,
    pub created_at: String,   // ISO 8601 string
    pub point_threshold: i32, // Configurable win condition
}

/// Safe version of GameState that doesn't expose the target word
/// Used for HTTP responses where we need to protect game integrity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SafeGameState {
    pub id: Uuid,
    pub word_length: i32,
    pub current_round: i32,
    pub status: GameStatus,
    pub current_phase: GamePhase,
    pub players: Vec<Player>,
    pub official_board: Vec<GuessResult>,
    pub current_winner: Option<Uuid>,
    pub created_at: String,
    pub point_threshold: i32,
}

impl From<&GameState> for SafeGameState {
    fn from(game_state: &GameState) -> Self {
        SafeGameState {
            id: game_state.id,
            word_length: game_state.word_length,
            current_round: game_state.current_round,
            status: game_state.status.clone(),
            current_phase: game_state.current_phase.clone(),
            players: game_state.players.clone(),
            official_board: game_state.official_board.clone(),
            current_winner: game_state.current_winner,
            created_at: game_state.created_at.clone(),
            point_threshold: game_state.point_threshold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GameStatus {
    Queuing,   // Players in matchmaking
    Starting,  // Game created, waiting for players to connect
    Active,    // Game in progress
    Paused,    // Temporarily paused (disconnections)
    Completed, // Game finished normally
    Abandoned, // Game abandoned due to disconnections
    TimedOut,  // Game exceeded maximum duration
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GamePhase {
    Waiting,
    Countdown,
    Guessing,
    IndividualGuess,
    GameOver,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GuessResult {
    pub word: String,
    pub player_id: Uuid,
    pub letters: Vec<LetterResult>,
    pub points_earned: i32,
    pub timestamp: String, // ISO 8601 string
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LetterResult {
    pub letter: String,
    pub status: LetterStatus,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LetterStatus {
    Correct, // Blue - correct letter in correct position
    Present, // Orange - correct letter in wrong position
    Absent,  // Gray - letter not in word
}
