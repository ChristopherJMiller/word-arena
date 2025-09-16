import React, { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { GameLayout } from "./GameLayout";
import { RoundCompletionModal } from "./RoundCompletionModal";
import { GameOverModal } from "./GameOverModal";
import { useGameStore } from "../../store/gameStore";
import { useWebSocket } from "../../hooks/useWebSocket";
import type { ServerMessage } from "../../types/generated/ServerMessage";
import type { GuessResult } from "../../types/generated";

export const Game: React.FC = () => {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();
  const {
    setGameId,
    gameState,
    reconnectToGame,
    rejoinAfterDisconnect,
    setGameState,
    setCountdownEndTime,
    addPersonalGuess,
    setLastError,
    setCurrentGuess,
    setPendingGuess,
    pendingGuess,
  } = useGameStore();
  const {
    addMessageHandler,
    removeMessageHandler,
    isAuthenticated: isWSAuthenticated,
  } = useWebSocket();
  const [roundCompletionModal, setRoundCompletionModal] = useState<{
    isOpen: boolean;
    winningGuess: GuessResult | null;
    roundNumber: number;
  }>({ isOpen: false, winningGuess: null, roundNumber: 0 });
  const [gameOverModal, setGameOverModal] = useState<{
    isOpen: boolean;
    winner: any;
    finalScores: any[];
  }>({ isOpen: false, winner: null, finalScores: [] });
  const [isRejoining, setIsRejoining] = useState(false);

  // Create message handler with fresh state access
  const gameMessageHandler = useCallback((message: ServerMessage) => {
      if (typeof message === "object" && message !== null) {
        if ("GameStateUpdate" in message) {
          const newState = message.GameStateUpdate.state;
          const oldPhase = gameState?.current_phase;

          setGameState(newState);
          // Clear rejoining state since we successfully received game state
          setIsRejoining(false);
          console.log("Game state updated:", newState);

          // Clear pending guess when:
          // 1. Moving to a non-guessing phase
          // 2. Moving to a new round 
          // 3. Moving from IndividualGuess back to Guessing (phase transition)
          if (
            pendingGuess &&
            (newState.current_phase !== "Guessing" ||
              (oldPhase === "Guessing" &&
                newState.current_phase === "Guessing" &&
                newState.current_round !== gameState?.current_round) ||
              (oldPhase === "IndividualGuess" &&
                newState.current_phase === "Guessing"))
          ) {
            setPendingGuess(null);
            setCurrentGuess("");
          }
        } else if ("CountdownStart" in message) {
          const endTime = Date.now() + message.CountdownStart.seconds * 1000;
          setCountdownEndTime(endTime);
          console.log(
            "Countdown started:",
            message.CountdownStart.seconds,
            "seconds",
          );
        } else if ("RoundResult" in message) {
          // Add the winning guess to the official board (handled by GameStateUpdate)
          // Add personal guess to history if present
          if (message.RoundResult.your_guess) {
            addPersonalGuess(message.RoundResult.your_guess);
          }
          
          // Check if this was a word completion using the explicit backend flag
          const winningGuess = message.RoundResult.winning_guess;
          const isWordCompletion = message.RoundResult.is_word_completed;
          
          if (isWordCompletion) {
            console.log("Word completed! Showing completion modal for:", winningGuess);
            // Use the current round number from the existing game state (before it gets updated)
            const completedRound = gameState?.current_round || 1;
            setRoundCompletionModal({
              isOpen: true,
              winningGuess: winningGuess,
              roundNumber: completedRound,
            });
          }
          
          // Don't clear pending guess here - let GameStateUpdate handle it when phase changes
          console.log("Round result:", message.RoundResult);
        } else if ("GameOver" in message) {
          console.log("Game over:", message.GameOver);
          setGameOverModal({
            isOpen: true,
            winner: message.GameOver.winner,
            finalScores: message.GameOver.final_scores,
          });
        } else if ("PlayerDisconnected" in message) {
          console.log(
            "Player disconnected:",
            message.PlayerDisconnected.player_id,
          );
        } else if ("PlayerReconnected" in message) {
          console.log(
            "Player reconnected:",
            message.PlayerReconnected.player_id,
          );
        } else if ("GameLeft" in message) {
          navigate("/");
        } else if ("Error" in message) {
          // Don't log "No disconnected players to rejoin" as an error since it's expected
          // when navigating to a game you're already in
          if (
            message.Error.message.includes("No disconnected players to rejoin")
          ) {
            console.log("Already in game, no need to rejoin");
            // Clear rejoining state since we're already in the game
            setIsRejoining(false);
          } else {
            console.error("Game error:", message.Error.message);
            // Store game errors for display to user and clear pending state
            if (message.Error.message.includes("already guessed")) {
              setLastError("Word already guessed - try a different word");
              // Clear pending guess and restore it to current guess so user can edit
              if (pendingGuess) {
                setCurrentGuess(pendingGuess);
                setPendingGuess(null);
              }
            } else if (message.Error.message.includes("Invalid guess") || message.Error.message.includes("not in word list")) {
              setLastError("Invalid word - not in our word list");
              // Clear pending guess and restore it to current guess so user can edit
              if (pendingGuess) {
                setCurrentGuess(pendingGuess);
                setPendingGuess(null);
              }
            } else {
              setLastError(message.Error.message);
            }
          }
        }
      }
  }, [gameState, pendingGuess, setGameState, setCountdownEndTime, addPersonalGuess, setLastError, setCurrentGuess, setPendingGuess, navigate]);

  // Set up global game message handler
  useEffect(() => {
    if (!isWSAuthenticated || !gameId) return;

    addMessageHandler(gameMessageHandler);
    return () => removeMessageHandler(gameMessageHandler);
  }, [
    isWSAuthenticated,
    gameId,
    gameMessageHandler,
    addMessageHandler,
    removeMessageHandler,
  ]);

  useEffect(() => {
    if (!gameId) {
      // Invalid URL, redirect to lobby
      navigate("/");
      return;
    }

    // Set the gameId in the store
    setGameId(gameId);

    // Always try to rejoin when we have WebSocket authentication and a gameId
    // This handles both initial loads and page refreshes
    if (isWSAuthenticated && gameId) {
      setIsRejoining(true);
      // First try to reconnect (HTTP-based state fetch)
      // If that fails, fall back to WebSocket rejoin
      reconnectToGame(gameId)
        .then(() => {
          // HTTP reconnect succeeded, clear rejoining state
          setIsRejoining(false);
        })
        .catch((httpError) => {
          // Check if it's a 404 error (game not found)
          if (httpError.message.includes("404") || httpError.message.includes("Game not found")) {
            console.log("Game no longer exists, redirecting to lobby");
            setIsRejoining(false);
            navigate("/");
            return Promise.reject(new Error("Game not found - redirected to lobby"));
          }
          
          console.log(
            "HTTP reconnect failed, trying WebSocket rejoin:",
            httpError,
          );
          return rejoinAfterDisconnect(gameId);
        })
        .then(() => {
          // WebSocket rejoin completed, state will be cleared by GameStateUpdate message
        })
        .catch((error) => {
          console.error("Failed to load/rejoin game:", error);
          setIsRejoining(false);
          // If we can't load or rejoin the game, redirect to lobby
          // unless we already redirected due to game not found
          if (!error.message.includes("Game not found - redirected to lobby")) {
            navigate("/");
          }
        });
    }
  }, [gameId, isWSAuthenticated]); // Depend on both gameId and WebSocket authentication

  if (!gameId) {
    return null; // Will redirect to lobby
  }

  if (!gameState) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="bg-white rounded-lg shadow-md p-8 text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <h2 className="text-xl font-semibold text-gray-800 mb-2">
            {isRejoining ? "Rejoining game..." : "Loading..."}
          </h2>
          <p className="text-gray-600">
            {isRejoining 
              ? "Checking if game still exists and reconnecting..." 
              : `Loading game state for ${gameId}`}
          </p>
        </div>
      </div>
    );
  }

  return (
    <>
      <GameLayout isRejoining={isRejoining} />
      {gameState && (
        <RoundCompletionModal
          isOpen={roundCompletionModal.isOpen}
          onClose={() => setRoundCompletionModal({ isOpen: false, winningGuess: null, roundNumber: 0 })}
          winningGuess={roundCompletionModal.winningGuess!}
          currentRound={roundCompletionModal.roundNumber}
          players={gameState.players}
        />
      )}
      <GameOverModal
        isOpen={gameOverModal.isOpen}
        onClose={() => {
          setGameOverModal({ isOpen: false, winner: null, finalScores: [] });
          navigate("/");
        }}
        winner={gameOverModal.winner}
        finalScores={gameOverModal.finalScores}
      />
    </>
  );
};
