import React from "react";
import type { PersonalGuess } from "../../types/generated";
import { useGameStore } from "../../store/gameStore";
import { useAuthStore } from "../../store/authStore";

interface GuessHistoryProps {
  guesses: PersonalGuess[];
  currentRound: number;
}

interface GuessItemProps {
  guess: PersonalGuess;
  index: number;
}

const GuessItem: React.FC<GuessItemProps> = ({ guess, index }) => {
  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  return (
    <div
      className={`
        p-3 rounded-lg border transition-all duration-200
        ${
          guess.was_winning_guess
            ? "bg-green-50 border-green-300"
            : "bg-white border-gray-200 hover:bg-gray-50"
        }
      `}
      data-testid={`guess-item-${index}`}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {/* Guess Number */}
          <span className="text-sm text-gray-500 font-medium">
            #{index + 1}
          </span>

          {/* Word */}
          <span className="font-mono font-bold text-lg tracking-wide uppercase">
            {guess.word}
          </span>

          {/* Winner Badge */}
          {guess.was_winning_guess && (
            <span className="bg-green-500 text-white text-xs px-2 py-0.5 rounded-full font-semibold">
              Winner
            </span>
          )}
        </div>

        {/* Points */}
        <div className="text-right">
          <div className="font-bold text-lg">
            +{guess.points_earned}
            <span className="text-xs text-gray-500 ml-1">pts</span>
          </div>
          <div className="text-xs text-gray-400">
            {formatTime(guess.timestamp)}
          </div>
        </div>
      </div>
    </div>
  );
};

export const GuessHistory: React.FC<GuessHistoryProps> = ({
  guesses,
  currentRound,
}) => {
  // Group guesses by round (assuming guesses are in chronological order)
  const guessesByRound: PersonalGuess[][] = [];
  let currentRoundGuesses: PersonalGuess[] = [];

  guesses.forEach((guess) => {
    currentRoundGuesses.push(guess);
    if (guess.was_winning_guess) {
      guessesByRound.push([...currentRoundGuesses]);
      currentRoundGuesses = [];
    }
  });

  // Add any remaining guesses for the current round
  if (currentRoundGuesses.length > 0) {
    guessesByRound.push(currentRoundGuesses);
  }

  const totalPoints = guesses.reduce(
    (sum, guess) => sum + guess.points_earned,
    0,
  );

  return (
    <div className="flex flex-col gap-3" data-testid="guess-history">
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-lg font-bold text-gray-800">Your Guesses</h3>
        <span className="text-sm font-semibold text-blue-600">
          Total: {totalPoints} pts
        </span>
      </div>

      {/* Guess List */}
      <div className="space-y-2 max-h-[500px] overflow-y-auto">
        {guesses.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            No guesses yet. Start playing!
          </div>
        ) : (
          <>
            {guessesByRound.map((roundGuesses, roundIndex) => (
              <div key={roundIndex} className="space-y-2">
                {roundIndex > 0 && (
                  <div className="text-xs text-gray-400 font-semibold uppercase tracking-wide px-1">
                    Round {roundIndex + 1}
                  </div>
                )}
                {roundGuesses.map((guess, guessIndex) => (
                  <GuessItem
                    key={`${roundIndex}-${guessIndex}`}
                    guess={guess}
                    index={guesses.indexOf(guess)}
                  />
                ))}
              </div>
            ))}
          </>
        )}
      </div>

      {/* Stats Summary */}
      {guesses.length > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-200">
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div>
              <span className="text-gray-500">Total Guesses:</span>
              <span className="ml-2 font-semibold">{guesses.length}</span>
            </div>
            <div>
              <span className="text-gray-500">Winning:</span>
              <span className="ml-2 font-semibold">
                {guesses.filter((g) => g.was_winning_guess).length}
              </span>
            </div>
            <div>
              <span className="text-gray-500">Avg Points:</span>
              <span className="ml-2 font-semibold">
                {(totalPoints / guesses.length).toFixed(1)}
              </span>
            </div>
            <div>
              <span className="text-gray-500">Current Round:</span>
              <span className="ml-2 font-semibold">{currentRound}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

// Container component that connects to store
export const GuessHistoryContainer: React.FC = () => {
  const { gameState, personalGuessHistory } = useGameStore();
  const { user } = useAuthStore();

  // Debug logging
  console.log("GuessHistoryContainer debug:", {
    gameState: gameState ? "present" : "null",
    user: user ? `id: ${user.id}` : "null",
    players: gameState?.players?.length || 0,
    personalGuessHistory: personalGuessHistory?.length || 0,
  });

  if (!gameState) {
    return (
      <div className="text-center py-8 text-gray-500">
        No active game
        <div className="text-xs mt-2">
          Debug: gameState={gameState ? "✓" : "✗"}, user={user ? "✓" : "✗"}
        </div>
      </div>
    );
  }

  // Find the current player's guess history from game state
  const currentPlayer = gameState.players.find((p) => p.user_id === user?.id);
  const serverGuesses = currentPlayer?.guess_history || [];
  
  // Use server guess history if available, otherwise fall back to local history
  const guesses = serverGuesses.length > 0 ? serverGuesses : personalGuessHistory;

  return (
    <GuessHistory guesses={guesses} currentRound={gameState.current_round} />
  );
};

