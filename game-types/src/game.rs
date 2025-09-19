use crate::{GameId, PlayerId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::user::Player;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RoundCompletion {
    pub word: String,
    pub player_id: PlayerId,
    pub points_earned: i32,
}

#[derive(Debug, Clone)]
pub enum RoundResult {
    Continuing(GuessResult),
    WordCompleted(RoundCompletion),
    GameOver(GuessResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GameState {
    pub id: GameId,
    pub word: String,
    pub word_length: i32,
    pub current_round: i32,
    pub status: GameStatus,
    pub current_phase: GamePhase,
    pub players: Vec<Player>,
    pub official_board: Vec<GuessResult>,
    pub current_winner: Option<PlayerId>,
    pub created_at: String,   // ISO 8601 string
    pub point_threshold: i32, // Configurable win condition
}

/// Safe version of GameState that doesn't expose the target word
/// Used for HTTP responses where we need to protect game integrity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SafeGameState {
    pub id: GameId,
    pub word_length: i32,
    pub current_round: i32,
    pub status: GameStatus,
    pub current_phase: GamePhase,
    pub players: Vec<Player>,
    pub official_board: Vec<GuessResult>,
    pub current_winner: Option<PlayerId>,
    pub created_at: String,
    pub point_threshold: i32,
}

impl From<&GameState> for SafeGameState {
    fn from(game_state: &GameState) -> Self {
        SafeGameState {
            id: game_state.id.clone(),
            word_length: game_state.word_length,
            current_round: game_state.current_round,
            status: game_state.status.clone(),
            current_phase: game_state.current_phase.clone(),
            players: game_state.players.clone(),
            official_board: game_state.official_board.clone(),
            current_winner: game_state.current_winner.clone(),
            created_at: game_state.created_at.clone(),
            point_threshold: game_state.point_threshold,
        }
    }
}

impl GameState {
    /// Create a personalized version of the game state for a specific player
    /// Only includes that player's guess history, while other players' histories are cleared
    pub fn personalized_for_player(&self, player_id: PlayerId) -> Self {
        let filtered_players = self
            .players
            .iter()
            .map(|player| {
                if player.user_id == player_id {
                    // Keep the requesting player's full data
                    player.clone()
                } else {
                    // For other players, clear their guess history to protect privacy
                    Player {
                        user_id: player.user_id.clone(),
                        display_name: player.display_name.clone(),
                        points: player.points,
                        guess_history: Vec::new(), // Clear other players' guess histories
                        is_connected: player.is_connected,
                    }
                }
            })
            .collect();

        GameState {
            id: self.id.clone(),
            word: self.word.clone(),
            word_length: self.word_length,
            current_round: self.current_round,
            status: self.status.clone(),
            current_phase: self.current_phase.clone(),
            players: filtered_players,
            official_board: self.official_board.clone(),
            current_winner: self.current_winner.clone(),
            created_at: self.created_at.clone(),
            point_threshold: self.point_threshold,
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
    pub player_id: PlayerId,
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
