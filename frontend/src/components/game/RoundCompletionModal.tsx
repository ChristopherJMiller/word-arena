import React, { useEffect, useState } from "react";
import type { GuessResult, Player } from "../../types/generated";

interface RoundCompletionModalProps {
  isOpen: boolean;
  onClose: () => void;
  winningGuess: GuessResult;
  currentRound: number;
  players: Player[];
  autoCloseDelay?: number; // Auto-close after N milliseconds
}

export const RoundCompletionModal: React.FC<RoundCompletionModalProps> = ({
  isOpen,
  onClose,
  winningGuess,
  currentRound,
  players,
  autoCloseDelay = 4000, // 4 seconds default
}) => {
  const [countdown, setCountdown] = useState(Math.floor(autoCloseDelay / 1000));

  // Find the player who guessed the word
  const winningPlayer = players.find(p => p.user_id === winningGuess?.player_id);

  useEffect(() => {
    if (!isOpen) return;

    setCountdown(Math.floor(autoCloseDelay / 1000));

    const countdownInterval = setInterval(() => {
      setCountdown(prev => {
        if (prev <= 1) {
          onClose();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(countdownInterval);
  }, [isOpen, autoCloseDelay, onClose]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" data-testid="round-completion-modal">
      <div className="bg-white rounded-lg p-8 max-w-md w-full mx-4 text-center shadow-2xl">
        {/* Celebration Header */}
        <div className="mb-6">
          <div className="text-6xl mb-4">🎉</div>
          <h2 className="text-3xl font-bold text-green-600 mb-2">
            Word Found!
          </h2>
          <div className="text-2xl font-mono font-bold text-gray-800 bg-green-100 py-3 px-4 rounded-lg border-2 border-green-300">
            {winningGuess?.word?.toUpperCase() || "UNKNOWN"}
          </div>
        </div>

        {/* Winner Information */}
        <div className="mb-6">
          <p className="text-lg text-gray-700 mb-2">
            <span className="font-semibold text-blue-600">
              {winningPlayer?.display_name || "Unknown Player"}
            </span>
            {" "}solved the word!
          </p>
          <div className="flex justify-center items-center gap-2 text-lg">
            <span className="text-gray-600">Points earned:</span>
            <span className="font-bold text-green-600 text-xl">
              +{winningGuess?.points_earned || 0}
            </span>
          </div>
        </div>

        {/* Round Progress */}
        <div className="mb-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
          <h3 className="font-semibold text-blue-800 mb-2">Round Complete!</h3>
          <p className="text-blue-700">
            Round {currentRound} → Round {currentRound + 1}
          </p>
          <p className="text-sm text-blue-600 mt-1">
            Starting new round with a fresh word...
          </p>
        </div>

        {/* Auto-close Countdown */}
        <div className="flex justify-between items-center">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors"
            data-testid="close-modal-button"
          >
            Continue
          </button>
          <div className="text-sm text-gray-500">
            Auto-closing in {countdown}s
          </div>
        </div>
      </div>
    </div>
  );
};