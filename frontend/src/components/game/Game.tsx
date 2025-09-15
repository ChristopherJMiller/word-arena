import React, { useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { GameLayout } from './GameLayout';
import { useGameStore } from '../../store/gameStore';
import { useWebSocket } from '../../hooks/useWebSocket';
import type { ServerMessage } from '../../types/generated/ServerMessage';

export const Game: React.FC = () => {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();
  const { setGameId, gameState, reconnectToGame, setGameState, setCountdownEndTime, addPersonalGuess } = useGameStore();
  const { addMessageHandler, removeMessageHandler, isAuthenticated: isWSAuthenticated } = useWebSocket();

  // Set up global game message handler
  useEffect(() => {
    if (!isWSAuthenticated || !gameId) return;

    const gameMessageHandler = (message: ServerMessage) => {
      if (typeof message === 'object' && message !== null) {
        if ('GameStateUpdate' in message) {
          setGameState(message.GameStateUpdate.state);
          console.log('Game state updated:', message.GameStateUpdate.state);
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
          console.error('Game error:', message.Error.message);
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

    // If we don't have game state, attempt to reconnect
    if (!gameState || gameState.id !== gameId) {
      reconnectToGame(gameId);
    }
  }, [gameId, gameState, setGameId, reconnectToGame, navigate]);

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