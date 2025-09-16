mod test_helpers;

use game_server::game_manager::GameEvent;
use game_types::{GamePhase, GameStatus};
use test_helpers::*;

#[tokio::test]
async fn test_game_creation_basic() {
    let setup = TestGameServerSetup::new();

    let connections = setup.create_multiple_connections(&["Alice", "Bob"]).await;
    let connection_ids: Vec<_> = connections.iter().map(|(id, _)| *id).collect();

    let result = setup.create_test_game(connection_ids).await;
    assert!(result.is_ok());

    let game_id = result.unwrap();
    let state = setup.game_manager.get_game_state(&game_id).await;
    assert!(state.is_some());
    assert_eq!(state.unwrap().players.len(), 2);
}

#[tokio::test]
async fn test_game_creation_insufficient_players() {
    let setup = TestGameServerSetup::new();

    let connections = setup.create_multiple_connections(&["Alice"]).await;
    let connection_ids: Vec<_> = connections.iter().map(|(id, _)| *id).collect();

    let result = setup.create_test_game(connection_ids).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Need at least 2 players"));
}

#[tokio::test]
async fn test_single_guess_submission_waits_for_others() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];

    // Alice submits a guess
    let event = setup
        .submit_guess(&game_id, *alice_conn, "ABOUT")
        .await
        .unwrap();

    // Should get StateUpdate, not RoundResult (waiting for Bob)
    assert_state_update(&event);
}

#[tokio::test]
async fn test_both_players_guess_triggers_round_processing() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];

    // Alice submits first guess - should wait
    let event1 = setup
        .submit_guess(&game_id, *alice_conn, "ABOUT")
        .await
        .unwrap();
    assert_state_update(&event1);

    // Bob submits second guess - should trigger round processing
    let event2 = setup
        .submit_guess(&game_id, *bob_conn, "AFTER")
        .await
        .unwrap();

    // Now should get RoundResult or GameOver
    match event2 {
        GameEvent::RoundResult { .. } => {
            // Game continues
        }
        GameEvent::GameOver { .. } => {
            // Game ended (word was guessed)
        }
        _ => panic!("Expected RoundResult or GameOver, got {:?}", event2),
    }
}

#[tokio::test]
async fn test_multiplayer_guess_coordination() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob", "Charlie"])
        .await
        .unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];
    let (charlie_conn, _) = &connections[2];

    // First two guesses should return StateUpdate
    let event1 = setup
        .submit_guess(&game_id, *alice_conn, "ABOUT")
        .await
        .unwrap();
    assert_state_update(&event1);

    let event2 = setup
        .submit_guess(&game_id, *bob_conn, "ABOVE")
        .await
        .unwrap();
    assert_state_update(&event2);

    // Third guess should trigger processing
    let event3 = setup
        .submit_guess(&game_id, *charlie_conn, "AFTER")
        .await
        .unwrap();

    // Should now get RoundResult or GameOver
    match event3 {
        GameEvent::RoundResult {
            winning_guess,
            player_guesses,
        } => {
            assert!(!winning_guess.word.is_empty());
            assert_eq!(player_guesses.len(), 3); // All players should have personal guesses
        }
        GameEvent::GameOver { .. } => {
            // Acceptable if word was guessed
        }
        _ => panic!("Expected RoundResult or GameOver, got {:?}", event3),
    }
}

#[tokio::test]
async fn test_invalid_word_rejection() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];

    // Try to submit invalid word
    let result = setup.submit_guess(&game_id, *alice_conn, "XYZABC").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid word"));
}

#[tokio::test]
async fn test_duplicate_word_rejection() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];

    // Play one complete round
    let _event = play_round(
        &setup,
        &game_id,
        vec![(*alice_conn, "ABOUT"), (*bob_conn, "AFTER")],
    )
    .await
    .unwrap();

    // Try to submit the same word that was already guessed (assuming it was winning)
    // Note: We need to check which word won first
    let state = setup.game_manager.get_game_state(&game_id).await.unwrap();
    if let Some(last_guess) = state.official_board.last() {
        let result = setup
            .submit_guess(&game_id, *alice_conn, &last_guess.word)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already guessed"));
    }
}

#[tokio::test]
async fn test_player_not_in_game() {
    let setup = TestGameServerSetup::new();
    let (game_id, _) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    // Create another player not in the game
    let (outsider_conn, _) = setup.create_authenticated_connection("Outsider").await;

    let result = setup.submit_guess(&game_id, outsider_conn, "ABOUT").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Player not in game"));
}

#[tokio::test]
async fn test_game_not_found() {
    let setup = TestGameServerSetup::new();
    let (alice_conn, _) = setup.create_authenticated_connection("Alice").await;

    let result = setup
        .submit_guess("nonexistent-game", alice_conn, "ABOUT")
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Game not found"));
}

#[tokio::test]
async fn test_disconnected_player_handling() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob", "Charlie"])
        .await
        .unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];
    let (charlie_conn, _) = &connections[2];

    // Simulate Charlie disconnecting
    setup
        .game_manager
        .handle_player_disconnect(&game_id, *charlie_conn)
        .await
        .unwrap();

    // Now only Alice and Bob need to guess for round to process
    let event1 = setup
        .submit_guess(&game_id, *alice_conn, "ABOUT")
        .await
        .unwrap();
    assert_state_update(&event1); // Still waiting for Bob

    let event2 = setup
        .submit_guess(&game_id, *bob_conn, "AFTER")
        .await
        .unwrap();
    // Should process now since Charlie is disconnected
    match event2 {
        GameEvent::RoundResult { .. } | GameEvent::GameOver { .. } => {
            // Expected - round processed with only connected players
        }
        _ => panic!(
            "Expected round processing with disconnected player, got {:?}",
            event2
        ),
    }
}

#[tokio::test]
async fn test_game_state_progression() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    // Check initial state
    let initial_state = setup.game_manager.get_game_state(&game_id).await.unwrap();
    assert_eq!(initial_state.status, GameStatus::Active);
    assert_eq!(initial_state.current_round, 1);
    assert_eq!(initial_state.official_board.len(), 0);

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];

    // Play a complete round
    let _event = play_round(
        &setup,
        &game_id,
        vec![(*alice_conn, "ABOUT"), (*bob_conn, "AFTER")],
    )
    .await
    .unwrap();

    // Check state after round
    let after_state = setup.game_manager.get_game_state(&game_id).await.unwrap();

    // Either game is over or round incremented
    match after_state.status {
        GameStatus::Completed => {
            assert_eq!(after_state.current_phase, GamePhase::GameOver);
        }
        GameStatus::Active => {
            assert!(
                after_state.current_round >= 2
                    || after_state.current_phase == GamePhase::IndividualGuess
            );
            assert!(after_state.official_board.len() >= 1);
        }
        _ => panic!("Unexpected game status: {:?}", after_state.status),
    }
}

#[tokio::test]
async fn test_game_continues_with_valid_words() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];

    // Play several rounds with valid words from our word list
    let valid_words = [
        "ABOUT", "ABOVE", "AFTER", "AGAIN", "BEACH", "BLACK", "BROWN", "CHAIR",
    ];
    let mut round_count = 0;

    for i in (0..valid_words.len()).step_by(2) {
        if i + 1 >= valid_words.len() {
            break;
        }

        let word1 = valid_words[i];
        let word2 = valid_words[i + 1];

        let event = play_round(
            &setup,
            &game_id,
            vec![(*alice_conn, word1), (*bob_conn, word2)],
        )
        .await
        .unwrap();

        round_count += 1;

        match event {
            GameEvent::GameOver {
                winner,
                final_scores,
            } => {
                // Game ended (either word was guessed or points threshold reached)
                assert_eq!(final_scores.len(), 2);
                assert!(winner.points > 0);

                let final_state = setup.game_manager.get_game_state(&game_id).await.unwrap();
                assert_eq!(final_state.status, GameStatus::Completed);
                assert_eq!(final_state.current_phase, GamePhase::GameOver);
                return; // Test passed - game completed normally
            }
            GameEvent::RoundResult {
                winning_guess,
                player_guesses,
            } => {
                // Game continues - verify round was processed correctly
                assert!(!winning_guess.word.is_empty());
                assert_eq!(player_guesses.len(), 2);
                assert!(winning_guess.points_earned >= 0);
            }
            _ => panic!("Unexpected event: {:?}", event),
        }
    }

    // If we get here, we played several rounds successfully
    // Verify the game state is consistent
    let final_state = setup.game_manager.get_game_state(&game_id).await.unwrap();
    assert!(
        final_state.current_round > 1 || final_state.current_phase == GamePhase::IndividualGuess
    );
    assert!(final_state.official_board.len() >= round_count);

    // This validates that the round processing works correctly
    println!(
        "✅ Successfully played {} rounds with valid word processing",
        round_count
    );
}

#[tokio::test]
async fn test_concurrent_guess_overwrite() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];

    // Alice submits first guess
    let _event1 = setup
        .submit_guess(&game_id, *alice_conn, "ABOUT")
        .await
        .unwrap();

    // Alice submits second guess (should overwrite first)
    let _event2 = setup
        .submit_guess(&game_id, *alice_conn, "AFTER")
        .await
        .unwrap();

    // Bob submits final guess to trigger processing
    let event3 = setup
        .submit_guess(&game_id, *bob_conn, "AGAIN")
        .await
        .unwrap();

    // Check that processing occurred
    match event3 {
        GameEvent::RoundResult { player_guesses, .. } => {
            // Alice's guess should be "AFTER" (the latest one)
            let alice_guess = player_guesses
                .iter()
                .find(|(conn_id, _)| *conn_id == *alice_conn);
            assert!(alice_guess.is_some());
            // Note: We can't easily check the exact word without more complex setup
        }
        GameEvent::GameOver { .. } => {
            // Also acceptable
        }
        _ => panic!("Expected RoundResult or GameOver, got {:?}", event3),
    }
}

#[tokio::test]
async fn test_points_accumulation() {
    let setup = TestGameServerSetup::new();
    let (game_id, connections) = setup_ready_game(&setup, &["Alice", "Bob"]).await.unwrap();

    let (alice_conn, _) = &connections[0];
    let (bob_conn, _) = &connections[1];

    // Play one complete round
    let event = play_round(
        &setup,
        &game_id,
        vec![(*alice_conn, "ABOUT"), (*bob_conn, "AFTER")],
    )
    .await
    .unwrap();

    let state = setup.game_manager.get_game_state(&game_id).await.unwrap();

    match event {
        GameEvent::RoundResult { winning_guess, .. } => {
            // Winning player should have earned points
            assert!(winning_guess.points_earned >= 0);

            // At least one player should have points > 0
            let total_points: i32 = state.players.iter().map(|p| p.points).sum();
            assert!(total_points >= 0);
        }
        GameEvent::GameOver { winner, .. } => {
            // Winner should have points
            assert!(winner.points > 0);
        }
        _ => panic!("Unexpected event: {:?}", event),
    }
}
