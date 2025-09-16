use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GameError {
    InvalidWord { word: String },
    GameNotFound { game_id: String },
    PlayerNotFound { player_id: String },
    NotYourTurn,
    GameAlreadyCompleted,
    WordAlreadyGuessed { word: String },
    RateLimitExceeded,
    AuthenticationRequired,
    InvalidGameState { current_state: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ConnectionError {
    InvalidToken,
    SessionExpired,
    UserAlreadyConnected,
    ServerOverloaded,
    InternalError { message: String },
}
