use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub total_points: i32,
    pub total_wins: i32,
    pub total_games: i32,
    pub created_at: String, // ISO 8601 string for simplicity
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Player {
    pub user_id: Uuid,
    pub display_name: String,
    pub points: i32,
    pub guess_history: Vec<PersonalGuess>,
    pub is_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonalGuess {
    pub word: String,
    pub points_earned: i32,
    pub was_winning_guess: bool,
    pub timestamp: String, // ISO 8601 string
}
