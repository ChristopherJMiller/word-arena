use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{GamePhase, GameState, GuessResult, PersonalGuess, Player};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ClientMessage {
    Authenticate { token: String },
    ForceAuthenticate { token: String }, // Force disconnect existing session
    JoinQueue,
    LeaveQueue,
    VoteStartGame,
    SubmitGuess { word: String },
    LeaveGame,
    RejoinGame { game_id: String },
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ServerMessage {
    AuthenticationSuccess {
        user: crate::User,
    },
    AuthenticationFailed {
        reason: String,
    },
    SessionConflict {
        existing_connection: String, // Description of existing session
    },
    QueueJoined {
        position: u32,
    },
    QueueLeft,
    MatchmakingCountdown {
        seconds_remaining: u32,
        players_ready: u32,
        total_players: u32,
    },
    MatchFound {
        game_id: String,
        players: Vec<Player>,
    },
    GameStateUpdate {
        state: GameState,
    },
    CountdownStart {
        seconds: u32,
    },
    RoundResult {
        winning_guess: GuessResult,
        your_guess: Option<PersonalGuess>,
        next_phase: GamePhase,
        is_word_completed: bool, // Explicit flag for word completion
    },
    GameOver {
        winner: Player,
        final_scores: Vec<Player>,
    },
    GameLeft,
    PlayerDisconnected {
        player_id: Uuid,
    },
    PlayerReconnected {
        player_id: Uuid,
    },
    SessionDisconnected {
        reason: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConnectionInfo {
    pub session_token: String,
    pub user_id: Uuid,
    pub reconnection_token: Option<String>,
}
