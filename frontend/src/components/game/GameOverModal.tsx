import React, { useEffect, useState } from "react";
import type { Player } from "../../types/generated";

interface GameOverModalProps {
  isOpen: boolean;
  onClose: () => void;
  winner: Player;
  finalScores: Player[];
  autoCloseDelay?: number; // Auto-close after N milliseconds
}

export const GameOverModal: React.FC<GameOverModalProps> = ({
  isOpen,
  onClose,
  winner,
  finalScores,
  autoCloseDelay = 8000, // 8 seconds default for game over
}) => {
  const [countdown, setCountdown] = useState(Math.floor(autoCloseDelay / 1000));

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

  // Sort players by points for final standings
  const sortedPlayers = [...finalScores].sort((a, b) => b.points - a.points);

  // Get placement info for styling
  const getPlacementStyle = (index: number) => {
    switch (index) {
      case 0: // 1st place
        return {
          bg: "bg-yellow-50 border-2 border-yellow-300",
          textColor: "text-yellow-700",
          nameColor: "text-yellow-800",
          emoji: "🥇",
          place: "1st"
        };
      case 1: // 2nd place
        return {
          bg: "bg-gray-100 border-2 border-gray-400",
          textColor: "text-gray-600",
          nameColor: "text-gray-800",
          emoji: "🥈",
          place: "2nd"
        };
      case 2: // 3rd place
        return {
          bg: "bg-orange-50 border-2 border-orange-300",
          textColor: "text-orange-600",
          nameColor: "text-orange-800",
          emoji: "🥉",
          place: "3rd"
        };
      default:
        return {
          bg: "bg-gray-50",
          textColor: "text-gray-600",
          nameColor: "text-gray-800",
          emoji: "",
          place: `${index + 1}th`
        };
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" data-testid="game-over-modal">
      <div className="bg-white rounded-lg p-8 max-w-lg w-full mx-4 text-center shadow-2xl">
        {/* Victory Header */}
        <div className="mb-6">
          <div className="text-8xl mb-4">🏆</div>
          <h2 className="text-4xl font-bold text-purple-600 mb-2">
            Game Over!
          </h2>
          <div className="text-2xl font-bold text-green-600 mb-4">
            {winner.display_name} Wins!
          </div>
          <div className="text-lg text-gray-600">
            Final Score: <span className="font-bold text-green-600">{winner.points} points</span>
          </div>
        </div>

        {/* Final Standings */}
        <div className="mb-6">
          <h3 className="text-xl font-semibold text-gray-800 mb-4">Final Standings</h3>
          <div className="space-y-2">
            {sortedPlayers.map((player, index) => {
              const style = getPlacementStyle(index);
              return (
                <div 
                  key={player.user_id} 
                  className={`flex items-center justify-between p-3 rounded-lg ${style.bg}`}
                >
                  <div className="flex items-center space-x-3">
                    <span className={`font-bold text-lg ${style.textColor}`}>
                      {style.place}
                    </span>
                    <span className={`font-medium ${style.nameColor}`}>
                      {player.display_name}
                    </span>
                    {style.emoji && <span className="text-lg">{style.emoji}</span>}
                  </div>
                  <div className={`font-bold ${style.textColor}`}>
                    {player.points} pts
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex justify-between items-center">
          <button
            onClick={onClose}
            className="px-6 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors font-medium"
            data-testid="return-to-lobby-button"
          >
            Return to Lobby
          </button>
          <div className="text-sm text-gray-500">
            Auto-returning in {countdown}s
          </div>
        </div>
      </div>
    </div>
  );
};