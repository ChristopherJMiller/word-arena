import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { GameLayout } from './GameLayout';
import { GameNotFound } from './GameNotFound';
import { useGameStore } from '../../store/gameStore';
import { useWebSocket } from '../../hooks/useWebSocket';
import type { ServerMessage } from '../../types/generated/ServerMessage';

export const Game: React.FC = () => {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();
  const { setGameId, gameState, reconnectToGame, rejoinAfterDisconnect, setGameState, setCountdownEndTime, addPersonalGuess, setLastError, setCurrentGuess, setPendingGuess, pendingGuess } = useGameStore();
  const { addMessageHandler, removeMessageHandler, isAuthenticated: isWSAuthenticated } = useWebSocket();
  const [gameNotFound, setGameNotFound] = useState(false);

  // Set up global game message handler
  useEffect(() => {
    if (!isWSAuthenticated || !gameId) return;

    const gameMessageHandler = (message: ServerMessage) => {
      if (typeof message === 'object' && message !== null) {
        if ('GameStateUpdate' in message) {
          const newState = message.GameStateUpdate.state;
          const oldPhase = gameState?.current_phase;
          
          setGameState(newState);
          console.log('Game state updated:', newState);
          
          // Clear pending guess when moving to a new round or phase that's not guessing
          if (pendingGuess && (newState.current_phase !== 'Guessing' || 
              (oldPhase === 'Guessing' && newState.current_phase === 'Guessing' && newState.current_round !== gameState?.current_round))) {
            setPendingGuess(null);
            setCurrentGuess('');
          }
        } else if ('CountdownStart' in message) {
          const endTime = Date.now() + (message.CountdownStart.seconds * 1000);
          setCountdownEndTime(endTime);
          console.log('Countdown started:', message.CountdownStart.seconds, 'seconds');
        } else if ('RoundResult' in message) {
          // Add the winning guess to the official board (handled by GameStateUpdate)
          // Add personal guess to history if present
          if (message.RoundResult.your_guess) {
            addPersonalGuess(message.RoundResult.your_guess);
          }
          // Don't clear pending guess here - let GameStateUpdate handle it when phase changes
          console.log('Round result:', message.RoundResult);
        } else if ('GameOver' in message) {
          console.log('Game over:', message.GameOver);
          // Could show a game over modal or navigate back to lobby
        } else if ('PlayerDisconnected' in message) {
          console.log('Player disconnected:', message.PlayerDisconnected.player_id);
        } else if ('PlayerReconnected' in message) {
          console.log('Player reconnected:', message.PlayerReconnected.player_id);
        } else if ('GameLeft' in message) {
          navigate('/');
        } else if ('Error' in message) {
          // Don't log "No disconnected players to rejoin" as an error since it's expected
          // when navigating to a game you're already in
          if (message.Error.message.includes('No disconnected players to rejoin')) {
            console.log('Already in game, no need to rejoin');
          } else {
            console.error('Game error:', message.Error.message);
            // Store game errors for display to user and clear pending state
            if (message.Error.message.includes('Invalid guess')) {
              setLastError('Invalid word - not in our word list');
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
    };

    addMessageHandler(gameMessageHandler);
    return () => removeMessageHandler(gameMessageHandler);
  }, [isWSAuthenticated, gameId, setGameState, setCountdownEndTime, addPersonalGuess, addMessageHandler, removeMessageHandler, navigate]);

  useEffect(() => {
    if (!gameId) {
      // Invalid URL, redirect to lobby
      navigate('/');
      return;
    }

    // Set the gameId in the store
    setGameId(gameId);

    // If we don't have game state or it doesn't match, try to load/rejoin the game
    if (!gameState || gameState.id !== gameId) {
      // First try to reconnect (HTTP-based state fetch)
      // If that fails, fall back to WebSocket rejoin
      reconnectToGame(gameId).catch((httpError) => {
        console.log('HTTP reconnect failed, trying WebSocket rejoin:', httpError);
        return rejoinAfterDisconnect(gameId);
      }).catch((error) => {
        console.error('Failed to load/rejoin game:', error);
        // If we can't load or rejoin the game, redirect to lobby
        navigate('/');
      });
    }
  }, [gameId]); // Only depend on gameId to prevent unnecessary re-renders

  if (!gameId) {
    return null; // Will redirect to lobby
  }

  if (!gameState) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="bg-white rounded-lg shadow-md p-8 text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <h2 className="text-xl font-semibold text-gray-800 mb-2">Reconnecting...</h2>
          <p className="text-gray-600">Loading game state for {gameId}</p>
        </div>
      </div>
    );
  }

  return <GameLayout />;
};