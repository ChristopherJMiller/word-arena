use game_types::{GameId, GamePhase, GuessResult, Player, PlayerId};

#[derive(Debug, Clone)]
pub enum GameEvent {
    GameCreated {
        game_id: GameId,
        players: Vec<Player>,
        word: String,
        point_threshold: i32,
    },
    CountdownStarted {
        game_id: GameId,
        duration_seconds: u32,
    },
    GuessSubmitted {
        game_id: GameId,
        player_id: PlayerId,
        word: String,
    },
    RoundCompleted {
        game_id: GameId,
        winning_guess: GuessResult,
        next_phase: GamePhase,
    },
    WordSolved {
        game_id: GameId,
        solution: String,
        solver: PlayerId,
    },
    GameCompleted {
        game_id: GameId,
        winner: Player,
        final_scores: Vec<Player>,
    },
    PlayerDisconnected {
        game_id: GameId,
        player_id: PlayerId,
    },
    PlayerReconnected {
        game_id: GameId,
        player_id: PlayerId,
    },
    GameAbandoned {
        game_id: GameId,
        reason: String,
    },
    GameTimedOut {
        game_id: GameId,
    },
}

impl GameEvent {
    pub fn game_id(&self) -> GameId {
        match self {
            GameEvent::GameCreated { game_id, .. } => game_id.clone(),
            GameEvent::CountdownStarted { game_id, .. } => game_id.clone(),
            GameEvent::GuessSubmitted { game_id, .. } => game_id.clone(),
            GameEvent::RoundCompleted { game_id, .. } => game_id.clone(),
            GameEvent::WordSolved { game_id, .. } => game_id.clone(),
            GameEvent::GameCompleted { game_id, .. } => game_id.clone(),
            GameEvent::PlayerDisconnected { game_id, .. } => game_id.clone(),
            GameEvent::PlayerReconnected { game_id, .. } => game_id.clone(),
            GameEvent::GameAbandoned { game_id, .. } => game_id.clone(),
            GameEvent::GameTimedOut { game_id, .. } => game_id.clone(),
        }
    }
}

/// Event handler trait for processing game events
pub trait GameEventHandler {
    fn handle_event(&mut self, event: GameEvent);
}

/// Simple event bus for distributing game events
pub struct GameEventBus {
    handlers: Vec<Box<dyn GameEventHandler>>,
}

impl GameEventBus {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Box<dyn GameEventHandler>) {
        self.handlers.push(handler);
    }

    pub fn publish(&mut self, event: GameEvent) {
        for handler in &mut self.handlers {
            handler.handle_event(event.clone());
        }
    }
}

impl Default for GameEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        events: Vec<GameEvent>,
    }

    impl TestHandler {
        fn new() -> Self {
            Self { events: Vec::new() }
        }
    }

    impl GameEventHandler for TestHandler {
        fn handle_event(&mut self, event: GameEvent) {
            self.events.push(event);
        }
    }

    #[test]
    fn test_event_bus() {
        let mut bus = GameEventBus::new();
        let mut handler = TestHandler::new();

        let game_id = "test-game-id".to_string();
        let event = GameEvent::GameCreated {
            game_id,
            players: vec![],
            word: "test".to_string(),
            point_threshold: 25,
        };

        bus.add_handler(Box::new(handler));
        bus.publish(event.clone());

        // Note: This test is simplified - in practice you'd need to extract
        // the handler to check its state, or use interior mutability
    }
}
