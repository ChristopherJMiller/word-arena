import React, { useRef, useEffect, useState } from 'react';

interface GuessInputProps {
  wordLength: number;
  currentGuess: string;
  isDisabled?: boolean;
  onGuessChange: (guess: string) => void;
  onSubmit: (guess: string) => void;
}

interface LetterInputProps {
  value: string;
  index: number;
  isActive: boolean;
  onChange: (index: number, value: string) => void;
  onKeyDown: (index: number, e: React.KeyboardEvent) => void;
  disabled?: boolean;
}

const LetterInput: React.FC<LetterInputProps> = ({
  value,
  index,
  isActive,
  onChange,
  onKeyDown,
  disabled,
}) => {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isActive && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isActive]);

  return (
    <input
      ref={inputRef}
      type="text"
      maxLength={1}
      value={value}
      disabled={disabled}
      onChange={(e) => {
        const newValue = e.target.value.toUpperCase();
        if (/^[A-Z]?$/.test(newValue)) {
          onChange(index, newValue);
        }
      }}
      onKeyDown={(e) => onKeyDown(index, e)}
      className={`
        w-12 h-12 md:w-14 md:h-14
        text-center text-xl md:text-2xl font-bold uppercase
        border-2 rounded-lg
        transition-all duration-200
        ${isActive && !disabled ? 'border-blue-500 ring-2 ring-blue-200' : 'border-gray-300'}
        ${disabled ? 'bg-gray-100 cursor-not-allowed' : 'bg-white hover:border-gray-400'}
        focus:outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-200
      `}
      data-testid={`letter-input-${index}`}
    />
  );
};

export const GuessInput: React.FC<GuessInputProps> = ({
  wordLength,
  currentGuess,
  isDisabled = false,
  onGuessChange,
  onSubmit,
}) => {
  const [activeIndex, setActiveIndex] = useState(0);
  const letters = currentGuess.padEnd(wordLength, ' ').split('').slice(0, wordLength);

  const handleLetterChange = (index: number, value: string) => {
    const newLetters = [...letters];
    newLetters[index] = value || ' ';
    const newGuess = newLetters.join('').trimEnd();
    onGuessChange(newGuess);

    // Move to next input if a letter was entered
    if (value && index < wordLength - 1) {
      setActiveIndex(index + 1);
    }
  };

  const handleKeyDown = (index: number, e: React.KeyboardEvent) => {
    if (e.key === 'Backspace') {
      if (!letters[index] || letters[index] === ' ') {
        // If current cell is empty, move to previous and delete
        if (index > 0) {
          e.preventDefault();
          const newLetters = [...letters];
          newLetters[index - 1] = ' ';
          const newGuess = newLetters.join('').trimEnd();
          onGuessChange(newGuess);
          setActiveIndex(index - 1);
        }
      } else {
        // Clear current cell
        e.preventDefault();
        const newLetters = [...letters];
        newLetters[index] = ' ';
        const newGuess = newLetters.join('').trimEnd();
        onGuessChange(newGuess);
      }
    } else if (e.key === 'ArrowLeft' && index > 0) {
      e.preventDefault();
      setActiveIndex(index - 1);
    } else if (e.key === 'ArrowRight' && index < wordLength - 1) {
      e.preventDefault();
      setActiveIndex(index + 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const trimmedGuess = currentGuess.trim();
      if (trimmedGuess.length === wordLength) {
        onSubmit(trimmedGuess);
      }
    }
  };

  const handleSubmitClick = () => {
    const trimmedGuess = currentGuess.trim();
    if (trimmedGuess.length === wordLength) {
      onSubmit(trimmedGuess);
    }
  };

  const isComplete = currentGuess.trim().length === wordLength;

  return (
    <div className="flex flex-col gap-4" data-testid="guess-input">
      <div className="flex gap-2 justify-center">
        {letters.map((letter, index) => (
          <LetterInput
            key={index}
            value={letter.trim()}
            index={index}
            isActive={activeIndex === index}
            onChange={handleLetterChange}
            onKeyDown={handleKeyDown}
            disabled={isDisabled}
          />
        ))}
      </div>
      <button
        onClick={handleSubmitClick}
        disabled={!isComplete || isDisabled}
        className={`
          px-6 py-3 rounded-lg font-semibold text-white
          transition-all duration-200
          ${
            isComplete && !isDisabled
              ? 'bg-blue-500 hover:bg-blue-600 active:scale-95'
              : 'bg-gray-300 cursor-not-allowed'
          }
        `}
        data-testid="submit-guess-button"
      >
        Submit Guess
      </button>
    </div>
  );
};

// Container component that connects to store
export const GuessInputContainer: React.FC = () => {
  const { gameState, currentGuess, isSubmitting, setCurrentGuess, lastError, clearError, pendingGuess, setPendingGuess } = useGameStore();
  const { sendMessage, isConnected, isAuthenticated } = useWebSocket();

  if (!gameState) {
    return null;
  }

  const handleSubmit = (guess: string) => {
    console.log('Attempting to submit guess:', guess);
    console.log('WebSocket connected:', isConnected);
    console.log('WebSocket authenticated:', isAuthenticated);
    
    // Clear any previous errors
    clearError();
    
    try {
      sendMessage({ SubmitGuess: { word: guess } });
      // Set the guess as pending and clear the current input
      setPendingGuess(guess);
      setCurrentGuess('');
    } catch (error) {
      console.error('Failed to send guess:', error);
    }
  };

  const handleGuessChange = (guess: string) => {
    // Clear error and pending guess when user starts typing a new guess
    if (lastError) {
      clearError();
    }
    if (pendingGuess) {
      setPendingGuess(null);
    }
    setCurrentGuess(guess);
  };

  // Determine if input should be disabled based on game state
  const shouldDisableInput = () => {
    if (isSubmitting) return true;
    
    // Only allow input during Active status with Guessing phase
    if (gameState.status !== 'Active') return true;
    
    // Only allow input during Guessing phase
    return gameState.current_phase !== 'Guessing';
  };

  // Show pending guess if it exists, otherwise show current guess
  const displayGuess = pendingGuess || currentGuess;
  const isShowingPending = !!pendingGuess;

  return (
    <div className="space-y-3">
      <GuessInput
        wordLength={gameState.word_length}
        currentGuess={displayGuess}
        isDisabled={shouldDisableInput() || isShowingPending}
        onGuessChange={handleGuessChange}
        onSubmit={handleSubmit}
      />
      {isShowingPending && (
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-2 text-center">
          <p className="text-blue-600 text-sm">Waiting for other players to submit...</p>
        </div>
      )}
      {lastError && (
        <div 
          className="bg-red-50 border border-red-200 rounded-lg p-3 text-center"
          data-testid="error-message"
        >
          <p className="text-red-600 text-sm font-medium">{lastError}</p>
        </div>
      )}
    </div>
  );
};

// Imports (will be at top in final version)
import { useGameStore } from '../../store/gameStore';
import { useWebSocket } from '../../hooks/useWebSocket';