import React, { useEffect, useState } from 'react';
import type { GamePhase } from '../../types/generated';

interface CountdownTimerProps {
  gamePhase: GamePhase;
  endTime?: number;
  onCountdownEnd?: () => void;
}

export const CountdownTimer: React.FC<CountdownTimerProps> = ({
  gamePhase,
  endTime,
  onCountdownEnd,
}) => {
  const [timeLeft, setTimeLeft] = useState(0);

  useEffect(() => {
    if (!endTime || endTime <= Date.now()) {
      setTimeLeft(0);
      return;
    }

    const updateTimer = () => {
      const remaining = Math.max(0, Math.floor((endTime - Date.now()) / 1000));
      setTimeLeft(remaining);
      
      if (remaining === 0 && onCountdownEnd) {
        onCountdownEnd();
      }
    };

    updateTimer();
    const interval = setInterval(updateTimer, 100);

    return () => clearInterval(interval);
  }, [endTime, onCountdownEnd]);

  const getStatusMessage = () => {
    switch (gamePhase) {
      case 'Waiting':
        return 'Waiting for players...';
      case 'Countdown':
        return 'Round starting...';
      case 'Guessing':
        return 'Submit your guess!';
      case 'IndividualGuess':
        return 'Winner is guessing...';
      case 'GameOver':
        return 'Game Over!';
      default:
        return 'Ready';
    }
  };

  const getStatusColor = () => {
    switch (gamePhase) {
      case 'Guessing':
        return 'text-green-600 bg-green-50';
      case 'IndividualGuess':
        return 'text-orange-600 bg-orange-50';
      case 'Countdown':
        return 'text-blue-600 bg-blue-50';
      case 'GameOver':
        return 'text-purple-600 bg-purple-50';
      default:
        return 'text-gray-600 bg-gray-50';
    }
  };

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const progressPercentage = endTime 
    ? Math.max(0, Math.min(100, (timeLeft / 30) * 100)) // Assuming 30 second rounds
    : 0;

  return (
    <div className="flex flex-col gap-3" data-testid="countdown-timer">
      {/* Status and Timer */}
      <div className="flex items-center justify-between">
        <div className={`px-3 py-1 rounded-full font-semibold text-sm ${getStatusColor()}`}>
          {getStatusMessage()}
        </div>
        
        {(gamePhase === 'Guessing' || gamePhase === 'IndividualGuess' || gamePhase === 'Countdown') && timeLeft > 0 && (
          <div className="flex items-center gap-2">
            <svg 
              className="w-5 h-5 text-gray-600" 
              fill="none" 
              stroke="currentColor" 
              viewBox="0 0 24 24"
            >
              <path 
                strokeLinecap="round" 
                strokeLinejoin="round" 
                strokeWidth={2} 
                d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" 
              />
            </svg>
            <span className={`font-mono font-bold text-2xl ${timeLeft <= 5 ? 'text-red-600 animate-pulse' : 'text-gray-800'}`}>
              {formatTime(timeLeft)}
            </span>
          </div>
        )}
      </div>

      {/* Progress Bar */}
      {(gamePhase === 'Guessing' || gamePhase === 'IndividualGuess') && (
        <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
          <div
            className={`h-full transition-all duration-100 ${
              timeLeft <= 5 ? 'bg-red-500' : 'bg-blue-500'
            }`}
            style={{ width: `${progressPercentage}%` }}
            data-testid="timer-progress"
          />
        </div>
      )}

      {/* Additional Info */}
      {gamePhase === 'IndividualGuess' && (
        <div className="text-sm text-gray-600 text-center">
          The round winner gets an extra guess!
        </div>
      )}
      
      {gamePhase === 'Waiting' && (
        <div className="text-sm text-gray-600 text-center">
          Game will start when enough players join
        </div>
      )}
    </div>
  );
};

// Container component that connects to store
export const CountdownTimerContainer: React.FC = () => {
  const { gameState, countdownEndTime, setCountdownEndTime } = useGameStore();

  if (!gameState) {
    return null;
  }

  // Map game status to phase
  const gamePhase: GamePhase = (() => {
    switch (gameState.status) {
      case 'Active':
        return 'Guessing';
      case 'Starting':
        return 'Countdown';
      case 'Completed':
      case 'Abandoned':
      case 'TimedOut':
        return 'GameOver';
      default:
        return 'Waiting';
    }
  })();

  return (
    <CountdownTimer
      gamePhase={gamePhase}
      endTime={countdownEndTime}
      onCountdownEnd={() => setCountdownEndTime(undefined)}
    />
  );
};

// Update game store to include countdown
import { useGameStore } from '../../store/gameStore';