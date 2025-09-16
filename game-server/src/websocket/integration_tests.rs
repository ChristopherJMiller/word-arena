use futures_util::StreamExt;
use serde_json;
use std::time::Duration;
use tokio::time::timeout;
use warp::test::ws;

use super::connection::ConnectionManager;
use crate::auth::AuthService;
use crate::create_routes;
use crate::game_manager::GameManager;
use crate::matchmaking::MatchmakingQueue;
use game_persistence::repositories::UserRepository;
use game_types::{ClientMessage, ServerMessage};
use migration::{Migrator, MigratorTrait};
use std::sync::Arc;

#[tokio::test]
async fn test_complete_matchmaking_flow() {
    // Initialize test components
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = Arc::new(GameManager::new(connection_manager.clone()));
    let matchmaking_queue = Arc::new(MatchmakingQueue::new());
    let auth_service = Arc::new(AuthService::new_dev_mode());

    // Create in-memory database for tests
    let db = game_persistence::connection::connect_to_memory_database()
        .await
        .unwrap();
    Migrator::up(&db, None).await.unwrap();
    let user_repository = Arc::new(UserRepository::new(db));

    let routes = create_routes(
        connection_manager.clone(),
        game_manager.clone(),
        matchmaking_queue.clone(),
        auth_service,
        user_repository,
    );

    // Test the complete flow with two players
    let mut ws1 = ws()
        .path("/ws")
        .handshake(routes.clone())
        .await
        .expect("WebSocket handshake failed");

    let mut ws2 = ws()
        .path("/ws")
        .handshake(routes)
        .await
        .expect("WebSocket handshake failed");

    // Step 1: Authenticate both players
    let auth_msg1 = ClientMessage::Authenticate {
        token: "user1:alice@example.com:Alice".to_string(),
    };
    let auth_msg2 = ClientMessage::Authenticate {
        token: "user2:bob@example.com:Bob".to_string(),
    };

    ws1.send(warp::ws::Message::text(
        serde_json::to_string(&auth_msg1).unwrap(),
    ))
    .await;

    ws2.send(warp::ws::Message::text(
        serde_json::to_string(&auth_msg2).unwrap(),
    ))
    .await;

    // Receive auth success for both
    let auth_response1 = timeout(Duration::from_secs(1), ws1.next())
        .await
        .expect("Timeout waiting for auth response")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let auth_response2 = timeout(Duration::from_secs(1), ws2.next())
        .await
        .expect("Timeout waiting for auth response")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    // Verify auth success messages
    assert!(auth_response1.is_text());
    assert!(auth_response2.is_text());

    let auth_msg1: ServerMessage = serde_json::from_str(auth_response1.to_str().unwrap()).unwrap();
    let auth_msg2: ServerMessage = serde_json::from_str(auth_response2.to_str().unwrap()).unwrap();

    assert!(matches!(
        auth_msg1,
        ServerMessage::AuthenticationSuccess { .. }
    ));
    assert!(matches!(
        auth_msg2,
        ServerMessage::AuthenticationSuccess { .. }
    ));

    // Step 2: Player 1 joins queue
    let join_queue_msg = ClientMessage::JoinQueue;
    ws1.send(warp::ws::Message::text(
        serde_json::to_string(&join_queue_msg).unwrap(),
    ))
    .await;

    // Receive QueueJoined for player 1
    let queue_joined1 = timeout(Duration::from_secs(1), ws1.next())
        .await
        .expect("Timeout waiting for queue joined")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let queue_msg1: ServerMessage = serde_json::from_str(queue_joined1.to_str().unwrap()).unwrap();
    if let ServerMessage::QueueJoined { position } = queue_msg1 {
        assert_eq!(position, 1);
    } else {
        panic!("Expected QueueJoined message, got: {:?}", queue_msg1);
    }

    // Step 3: Player 2 joins queue (should trigger countdown)
    ws2.send(warp::ws::Message::text(
        serde_json::to_string(&join_queue_msg).unwrap(),
    ))
    .await;

    // Receive QueueJoined for player 2
    let queue_joined2 = timeout(Duration::from_secs(1), ws2.next())
        .await
        .expect("Timeout waiting for queue joined")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let queue_msg2: ServerMessage = serde_json::from_str(queue_joined2.to_str().unwrap()).unwrap();
    if let ServerMessage::QueueJoined { position } = queue_msg2 {
        assert_eq!(position, 2);
    } else {
        panic!("Expected QueueJoined message, got: {:?}", queue_msg2);
    }

    // Both players should receive MatchmakingCountdown messages
    let countdown1 = timeout(Duration::from_secs(1), ws1.next())
        .await
        .expect("Timeout waiting for countdown")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let countdown2 = timeout(Duration::from_secs(1), ws2.next())
        .await
        .expect("Timeout waiting for countdown")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let countdown_msg1: ServerMessage = serde_json::from_str(countdown1.to_str().unwrap()).unwrap();
    let countdown_msg2: ServerMessage = serde_json::from_str(countdown2.to_str().unwrap()).unwrap();

    // Verify countdown messages
    if let ServerMessage::MatchmakingCountdown {
        seconds_remaining,
        players_ready,
        total_players,
    } = countdown_msg1
    {
        assert!(seconds_remaining <= 60);
        assert_eq!(players_ready, 0); // No votes yet
        assert_eq!(total_players, 2);
    } else {
        panic!(
            "Expected MatchmakingCountdown message, got: {:?}",
            countdown_msg1
        );
    }

    // Player 2 should get the same countdown info
    assert!(matches!(
        countdown_msg2,
        ServerMessage::MatchmakingCountdown { .. }
    ));

    // Step 4: Both players vote to start
    let vote_msg = ClientMessage::VoteStartGame;

    ws1.send(warp::ws::Message::text(
        serde_json::to_string(&vote_msg).unwrap(),
    ))
    .await;

    ws2.send(warp::ws::Message::text(
        serde_json::to_string(&vote_msg).unwrap(),
    ))
    .await;

    // Both players should receive updated countdown messages after each vote
    // After player 1 votes
    let _vote_update1_p1 = timeout(Duration::from_secs(1), ws1.next())
        .await
        .expect("Timeout")
        .expect("WebSocket closed")
        .expect("WebSocket error");
    let _vote_update1_p2 = timeout(Duration::from_secs(1), ws2.next())
        .await
        .expect("Timeout")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    // After player 2 votes
    let _vote_update2_p1 = timeout(Duration::from_secs(1), ws1.next())
        .await
        .expect("Timeout")
        .expect("WebSocket closed")
        .expect("WebSocket error");
    let _vote_update2_p2 = timeout(Duration::from_secs(1), ws2.next())
        .await
        .expect("Timeout")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    // Step 5: Match should be found (since both players voted)
    let match_found1 = timeout(Duration::from_secs(2), ws1.next())
        .await
        .expect("Timeout waiting for match found")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let match_found2 = timeout(Duration::from_secs(2), ws2.next())
        .await
        .expect("Timeout waiting for match found")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let match_msg1: ServerMessage = serde_json::from_str(match_found1.to_str().unwrap()).unwrap();
    let match_msg2: ServerMessage = serde_json::from_str(match_found2.to_str().unwrap()).unwrap();

    // Verify MatchFound messages
    let game_id = if let ServerMessage::MatchFound { game_id, players } = match_msg1 {
        assert!(!game_id.is_empty());
        assert_eq!(players.len(), 2); // Should have player info now
        game_id
    } else {
        panic!("Expected MatchFound message, got: {:?}", match_msg1);
    };

    // Player 2 should get the same game ID
    if let ServerMessage::MatchFound {
        game_id: game_id2,
        players,
    } = match_msg2
    {
        assert_eq!(game_id, game_id2);
        assert_eq!(players.len(), 2);
    } else {
        panic!("Expected MatchFound message, got: {:?}", match_msg2);
    }

    // Step 6: Both players should receive initial GameStateUpdate
    let game_state1 = timeout(Duration::from_secs(2), ws1.next())
        .await
        .expect("Timeout waiting for game state")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let game_state2 = timeout(Duration::from_secs(2), ws2.next())
        .await
        .expect("Timeout waiting for game state")
        .expect("WebSocket closed")
        .expect("WebSocket error");

    let state_msg1: ServerMessage = serde_json::from_str(game_state1.to_str().unwrap()).unwrap();
    let state_msg2: ServerMessage = serde_json::from_str(game_state2.to_str().unwrap()).unwrap();

    // Verify GameStateUpdate messages
    if let ServerMessage::GameStateUpdate { state } = state_msg1 {
        assert_eq!(state.id.to_string(), game_id);
        assert_eq!(state.players.len(), 2);
        assert_eq!(state.current_round, 1);
        assert!(matches!(state.status, game_types::GameStatus::Active));
        // Word should be hidden for game state updates (masked with asterisks)
        assert_eq!(state.word, "*****");
    } else {
        panic!("Expected GameStateUpdate message, got: {:?}", state_msg1);
    }

    // Player 2 should get the same state
    assert!(matches!(state_msg2, ServerMessage::GameStateUpdate { .. }));

    println!("✅ Complete matchmaking flow test passed!");
    println!("   - Both players authenticated");
    println!("   - Queue joining triggered countdown");
    println!("   - Voting triggered match creation");
    println!("   - Match found with proper player info");
    println!("   - Initial game state sent to both players");
}

#[tokio::test]
async fn test_countdown_expiration_flow() {
    // This test would verify that matches are created when countdown expires
    // even without enough votes. Due to the 60-second timer, we'll simulate
    // by manipulating the countdown_started_at time in the queue.

    // For now, this is a placeholder for a more complex test that would
    // require more sophisticated mocking or time manipulation.
    println!("⏰ Countdown expiration flow test - placeholder for future implementation");
}

#[tokio::test]
async fn test_player_disconnect_during_countdown() {
    // Test what happens when a player disconnects during countdown
    // Should remove them from queue and potentially stop countdown
    println!("🔌 Player disconnect during countdown test - placeholder for future implementation");
}
