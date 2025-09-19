use std::sync::Arc;

use game_server::{
    auth::AuthService,
    game_manager::{GameEvent, GameManager},
    websocket::connection::{ConnectionId, ConnectionManager},
};
use game_types::{GamePhase, User};

/// Test helper to create authenticated users and connections
async fn setup_authenticated_connections(
    connection_manager: &ConnectionManager,
    users: Vec<(&str, &str, &str)>, // (user_id, email, display_name)
) -> Vec<(ConnectionId, User)> {
    let mut connections = Vec::new();

    for (user_id, email, display_name) in users {
        let connection_id = ConnectionId::new();
        let _receiver = connection_manager.create_connection(connection_id).await;

        let user = User {
            id: user_id.to_string(),
            email: email.to_string(),
            display_name: display_name.to_string(),
            total_points: 0,
            total_wins: 0,
            total_games: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        connection_manager
            .set_connection_user(connection_id, Some(user.clone()))
            .await;
        connections.push((connection_id, user));
    }

    connections
}

#[tokio::test]
async fn test_create_game_with_authenticated_users() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create two authenticated users
    let connections = setup_authenticated_connections(
        &connection_manager,
        vec![
            (
                "550e8400-e29b-41d4-a716-446655440001",
                "alice@example.com",
                "Alice",
            ),
            (
                "550e8400-e29b-41d4-a716-446655440002",
                "bob@example.com",
                "Bob",
            ),
        ],
    )
    .await;

    let connection_ids: Vec<ConnectionId> = connections.iter().map(|(id, _)| *id).collect();

    // Create game should succeed with authenticated users
    let game_result = game_manager.create_game(connection_ids).await;
    assert!(game_result.is_ok());

    let game_id = game_result.unwrap();

    // Verify game state uses the correct user IDs
    let game_state = game_manager.get_game_state(&game_id).await;
    assert!(game_state.is_some());

    let state = game_state.unwrap();
    assert_eq!(state.players.len(), 2);

    // Verify players have the correct user IDs from authentication
    let alice = state
        .players
        .iter()
        .find(|p| p.display_name == "Alice")
        .unwrap();
    assert_eq!(alice.user_id, "550e8400-e29b-41d4-a716-446655440001");

    let bob = state
        .players
        .iter()
        .find(|p| p.display_name == "Bob")
        .unwrap();
    assert_eq!(bob.user_id, "550e8400-e29b-41d4-a716-446655440002");

    // Verify game is in correct initial state
    assert_eq!(state.current_phase, GamePhase::Guessing);
    assert_eq!(state.status, game_types::GameStatus::Active);
}

#[tokio::test]
async fn test_create_game_with_unauthenticated_users() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create unauthenticated connections
    let connection_id1 = ConnectionId::new();
    let connection_id2 = ConnectionId::new();

    let _receiver1 = connection_manager.create_connection(connection_id1).await;
    let _receiver2 = connection_manager.create_connection(connection_id2).await;

    // Attempt to create game should fail
    let result = game_manager
        .create_game(vec![connection_id1, connection_id2])
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not authenticated"));
}

#[tokio::test]
async fn test_create_game_with_partially_authenticated_users() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create one authenticated and one unauthenticated connection
    let connections = setup_authenticated_connections(
        &connection_manager,
        vec![(
            "550e8400-e29b-41d4-a716-446655440001",
            "alice@example.com",
            "Alice",
        )],
    )
    .await;

    let authenticated_id = connections[0].0;

    let unauthenticated_id = ConnectionId::new();
    let _receiver = connection_manager
        .create_connection(unauthenticated_id)
        .await;

    // Attempt to create game should fail
    let result = game_manager
        .create_game(vec![authenticated_id, unauthenticated_id])
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not authenticated"));
}

#[tokio::test]
async fn test_create_game_with_duplicate_users() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create two connections for the same user (should not be possible in normal flow,
    // but we're testing the validation)
    let user_id = "550e8400-e29b-41d4-a716-446655440001";
    let user = User {
        id: user_id.to_string(),
        email: "alice@example.com".to_string(),
        display_name: "Alice".to_string(),
        total_points: 0,
        total_wins: 0,
        total_games: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let connection_id1 = ConnectionId::new();
    let connection_id2 = ConnectionId::new();

    let _receiver1 = connection_manager.create_connection(connection_id1).await;
    let _receiver2 = connection_manager.create_connection(connection_id2).await;

    connection_manager
        .set_connection_user(connection_id1, Some(user.clone()))
        .await;
    connection_manager
        .set_connection_user(connection_id2, Some(user))
        .await;

    // Attempt to create game should fail due to duplicate user
    let result = game_manager
        .create_game(vec![connection_id1, connection_id2])
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already in the game"));
}

#[tokio::test]
async fn test_create_game_with_nonexistent_connections() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create random connection IDs that don't exist
    let connection_id1 = ConnectionId::new();
    let connection_id2 = ConnectionId::new();

    // Attempt to create game should fail
    let result = game_manager
        .create_game(vec![connection_id1, connection_id2])
        .await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Connection") && error_msg.contains("not found"));
}

#[tokio::test]
async fn test_create_game_with_insufficient_players() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create one authenticated user
    let connections = setup_authenticated_connections(
        &connection_manager,
        vec![(
            "550e8400-e29b-41d4-a716-446655440001",
            "alice@example.com",
            "Alice",
        )],
    )
    .await;

    let connection_ids: Vec<ConnectionId> = connections.iter().map(|(id, _)| *id).collect();

    // Attempt to create game should fail (need at least 2 players)
    let result = game_manager.create_game(connection_ids).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Need at least 2 players"));
}

#[tokio::test]
async fn test_submit_guess_validates_user_identity() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create two authenticated users
    let connections = setup_authenticated_connections(
        &connection_manager,
        vec![
            (
                "550e8400-e29b-41d4-a716-446655440001",
                "alice@example.com",
                "Alice",
            ),
            (
                "550e8400-e29b-41d4-a716-446655440002",
                "bob@example.com",
                "Bob",
            ),
        ],
    )
    .await;

    let connection_ids: Vec<ConnectionId> = connections.iter().map(|(id, _)| *id).collect();

    // Create game
    let game_id = game_manager
        .create_game(connection_ids.clone())
        .await
        .unwrap();

    // First player submits a guess - should work
    let result1 = game_manager
        .submit_guess(&game_id, connection_ids[0], "ABOUT".to_string())
        .await;

    // Verify the guess was associated with the correct user
    if let Ok(GameEvent::StateUpdate { state }) = result1 {
        // Check that Alice's guess is recorded
        let alice = state
            .players
            .iter()
            .find(|p| p.display_name == "Alice")
            .unwrap();
        assert_eq!(alice.user_id, "550e8400-e29b-41d4-a716-446655440001");
    }

    // Second player submits a guess - should also work
    let result2 = game_manager
        .submit_guess(&game_id, connection_ids[1], "BEACH".to_string())
        .await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_is_user_in_game_validates_correctly() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create two authenticated users
    let connections = setup_authenticated_connections(
        &connection_manager,
        vec![
            (
                "550e8400-e29b-41d4-a716-446655440001",
                "alice@example.com",
                "Alice",
            ),
            (
                "550e8400-e29b-41d4-a716-446655440002",
                "bob@example.com",
                "Bob",
            ),
        ],
    )
    .await;

    let connection_ids: Vec<ConnectionId> = connections.iter().map(|(id, _)| *id).collect();
    let alice_id = "550e8400-e29b-41d4-a716-446655440001".to_string();
    let bob_id = "550e8400-e29b-41d4-a716-446655440002".to_string();
    let stranger_id = "test-stranger-id".to_string();

    // Create game
    let game_id = game_manager.create_game(connection_ids).await.unwrap();

    // Alice and Bob should be in the game
    assert!(game_manager.is_user_in_game(&game_id, &alice_id).await);
    assert!(game_manager.is_user_in_game(&game_id, &bob_id).await);

    // Random user should not be in the game
    assert!(!game_manager.is_user_in_game(&game_id, &stranger_id).await);

    // Non-existent game should return false
    let fake_game_id = "fake-game-id".to_string();
    assert!(!game_manager.is_user_in_game(&fake_game_id, &alice_id).await);
}

#[tokio::test]
async fn test_security_user_cannot_access_other_game() {
    let connection_manager = Arc::new(ConnectionManager::new());
    let game_manager = GameManager::new_with_default_words(connection_manager.clone()).unwrap();

    // Create four users for two separate games
    let connections1 = setup_authenticated_connections(
        &connection_manager,
        vec![
            (
                "550e8400-e29b-41d4-a716-446655440001",
                "alice@example.com",
                "Alice",
            ),
            (
                "550e8400-e29b-41d4-a716-446655440002",
                "bob@example.com",
                "Bob",
            ),
        ],
    )
    .await;

    let connections2 = setup_authenticated_connections(
        &connection_manager,
        vec![
            (
                "550e8400-e29b-41d4-a716-446655440003",
                "charlie@example.com",
                "Charlie",
            ),
            (
                "550e8400-e29b-41d4-a716-446655440004",
                "diana@example.com",
                "Diana",
            ),
        ],
    )
    .await;

    let connection_ids1: Vec<ConnectionId> = connections1.iter().map(|(id, _)| *id).collect();
    let connection_ids2: Vec<ConnectionId> = connections2.iter().map(|(id, _)| *id).collect();

    // Create two separate games
    let game_id1 = game_manager.create_game(connection_ids1).await.unwrap();
    let game_id2 = game_manager.create_game(connection_ids2).await.unwrap();

    let alice_id = "550e8400-e29b-41d4-a716-446655440001".to_string();
    let charlie_id = "550e8400-e29b-41d4-a716-446655440003".to_string();

    // Alice should be in game 1 but not game 2
    assert!(game_manager.is_user_in_game(&game_id1, &alice_id).await);
    assert!(!game_manager.is_user_in_game(&game_id2, &alice_id).await);

    // Charlie should be in game 2 but not game 1
    assert!(!game_manager.is_user_in_game(&game_id1, &charlie_id).await);
    assert!(game_manager.is_user_in_game(&game_id2, &charlie_id).await);

    // Charlie should not be able to submit a guess to game 1 (Alice and Bob's game)
    let charlie_connection = connections2[0].0;
    let result = game_manager
        .submit_guess(&game_id1, charlie_connection, "ABOUT".to_string())
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Player not in game"));
}

#[tokio::test]
async fn test_auth_service_dev_mode_creates_proper_users() {
    let auth_service = AuthService::new_dev_mode();

    // Test string format token
    let string_token = "550e8400-e29b-41d4-a716-446655440001:alice@example.com:Alice";
    let user_result = auth_service.validate_token(string_token).await;

    assert!(user_result.is_ok());
    let user = user_result.unwrap();
    assert_eq!(user.id, "550e8400-e29b-41d4-a716-446655440001");
    assert_eq!(user.email, "alice@example.com");
    assert_eq!(user.display_name, "Alice");

    // Test JSON format token
    let json_token = r#"{"user_id":"550e8400-e29b-41d4-a716-446655440002","email":"bob@example.com","name":"Bob"}"#;
    let json_user_result = auth_service.validate_token(json_token).await;
    assert!(json_user_result.is_ok());
    let json_user = json_user_result.unwrap();
    assert_eq!(json_user.id, "550e8400-e29b-41d4-a716-446655440002");
    assert_eq!(json_user.email, "bob@example.com");
    assert_eq!(json_user.display_name, "Bob");
}
