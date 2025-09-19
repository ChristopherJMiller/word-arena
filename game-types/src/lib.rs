pub mod errors;
pub mod game;
pub mod messages;
pub mod user;

// Re-export all types
pub use errors::*;
pub use game::*;
pub use messages::*;
pub use user::*;

// Shared type aliases for cross-tenant support
pub type PlayerId = String; // Supports compound IDs like "user.tenant"
pub type GameId = String;
