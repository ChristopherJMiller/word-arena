use futures_util::{SinkExt, StreamExt};
use serde_json;
use std::sync::Arc;
use tracing::{error, info, warn};
use warp::ws::{Message, WebSocket};

use crate::auth::AuthService;
use crate::game_manager::GameManager;
use crate::matchmaking::MatchmakingQueue;
use game_types::ClientMessage;

pub mod connection;
pub mod handlers;
pub mod rate_limiter;

#[cfg(test)]
pub mod integration_tests;

use connection::ConnectionId;
pub use connection::ConnectionManager;
use handlers::MessageHandler;
use rate_limiter::RateLimiter;

pub async fn handle_connection(
    websocket: WebSocket,
    connection_manager: Arc<ConnectionManager>,
    game_manager: Arc<GameManager>,
    matchmaking_queue: Arc<MatchmakingQueue>,
    auth_service: Arc<AuthService>,
) {
    let connection_id = ConnectionId::new();
    info!("New WebSocket connection: {}", connection_id);

    let (mut ws_sender, mut ws_receiver) = websocket.split();
    let rate_limiter = RateLimiter::new();

    // Create connection and get receiver for outgoing messages
    let message_receiver = connection_manager.create_connection(connection_id).await;

    // Create message handler
    let message_handler = MessageHandler::new(
        connection_id,
        connection_manager.clone(),
        game_manager.clone(),
        matchmaking_queue.clone(),
        auth_service.clone(),
    );

    // Handle incoming messages
    let incoming_handler = {
        let _connection_manager = connection_manager.clone();
        let message_handler = message_handler.clone();
        let mut rate_limiter = rate_limiter.clone();

        async move {
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(msg) => {
                        if let Err(e) =
                            handle_message(msg, &mut rate_limiter, &message_handler, connection_id)
                                .await
                        {
                            error!("Error handling message for {}: {}", connection_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("WebSocket error for {}: {}", connection_id, e);
                        break;
                    }
                }
            }
        }
    };

    // Handle outgoing messages
    let outgoing_handler = {
        async move {
            let mut receiver = message_receiver;

            while let Some(message) = receiver.recv().await {
                let json = match serde_json::to_string(&message) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize message: {:?}", e);
                        continue;
                    }
                };

                if let Err(e) = ws_sender.send(Message::text(json)).await {
                    warn!("Failed to send message to {}: {:?}", connection_id, e);
                    break;
                }
            }
        }
    };

    // Run both handlers concurrently
    tokio::select! {
        _ = incoming_handler => {},
        _ = outgoing_handler => {},
    }

    // Cleanup connection
    info!("Connection {} disconnected", connection_id);
    message_handler.handle_disconnect().await;
    connection_manager.remove_connection(connection_id).await;
}

async fn handle_message(
    msg: Message,
    rate_limiter: &mut RateLimiter,
    message_handler: &MessageHandler,
    connection_id: ConnectionId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check rate limiting
    if !rate_limiter.check_rate_limit().await {
        warn!("Rate limit exceeded for connection {}", connection_id);
        return Err("Rate limit exceeded".into());
    }

    // Only handle text messages
    if !msg.is_text() {
        return Ok(());
    }

    let text = msg.to_str().map_err(|_| "Invalid text message")?;

    // Parse client message
    let client_message: ClientMessage =
        serde_json::from_str(text).map_err(|e| format!("Invalid JSON message: {}", e))?;

    // Handle the message
    message_handler
        .handle_message(client_message)
        .await
        .map_err(|e| format!("Message handling error: {}", e))?;

    Ok(())
}
