use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub points_to_win: u32,
    pub max_players_per_game: usize,
    pub min_players_per_game: usize,
    pub queue_timeout_seconds: u64,
    pub game_timeout_minutes: u64,
    pub connection_timeout_seconds: u64,
}

impl Config {
    pub fn new() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("Invalid PORT"),
            points_to_win: env::var("POINTS_TO_WIN")
                .unwrap_or_else(|_| "25".to_string())
                .parse()
                .expect("Invalid POINTS_TO_WIN"),
            max_players_per_game: env::var("MAX_PLAYERS_PER_GAME")
                .unwrap_or_else(|_| "16".to_string())
                .parse()
                .expect("Invalid MAX_PLAYERS_PER_GAME"),
            min_players_per_game: env::var("MIN_PLAYERS_PER_GAME")
                .unwrap_or_else(|_| "2".to_string())
                .parse()
                .expect("Invalid MIN_PLAYERS_PER_GAME"),
            queue_timeout_seconds: env::var("QUEUE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .expect("Invalid QUEUE_TIMEOUT_SECONDS"),
            game_timeout_minutes: env::var("GAME_TIMEOUT_MINUTES")
                .unwrap_or_else(|_| "120".to_string())
                .parse()
                .expect("Invalid GAME_TIMEOUT_MINUTES"),
            connection_timeout_seconds: env::var("CONNECTION_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .expect("Invalid CONNECTION_TIMEOUT_SECONDS"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}