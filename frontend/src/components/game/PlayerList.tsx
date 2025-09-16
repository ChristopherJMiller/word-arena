import React from "react";
import type { Player } from "../../types/generated";

interface PlayerListProps {
  players: Player[];
  currentPlayerId?: string;
  currentWinnerId?: string | null;
  pointThreshold: number;
}

interface PlayerItemProps {
  player: Player;
  isCurrentPlayer: boolean;
  isCurrentWinner: boolean;
  pointThreshold: number;
  rank: number;
}

const PlayerItem: React.FC<PlayerItemProps> = ({
  player,
  isCurrentPlayer,
  isCurrentWinner,
  pointThreshold,
  rank,
}) => {
  const progressPercentage = Math.min(
    (player.points / pointThreshold) * 100,
    100,
  );

  return (
    <div
      className={`
        relative p-3 rounded-lg transition-all duration-200
        ${isCurrentPlayer ? "bg-blue-50 border-2 border-blue-300" : "bg-white border border-gray-200"}
        ${isCurrentWinner ? "ring-2 ring-orange-400" : ""}
      `}
      data-testid={`player-item-${player.user_id}`}
    >
      {/* Player Info */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          {/* Rank */}
          <span
            className={`
              text-sm font-bold w-6 h-6 rounded-full flex items-center justify-center
              ${rank === 1 ? "bg-yellow-400 text-white" : ""}
              ${rank === 2 ? "bg-gray-400 text-white" : ""}
              ${rank === 3 ? "bg-orange-600 text-white" : ""}
              ${rank > 3 ? "bg-gray-200 text-gray-700" : ""}
            `}
          >
            {rank}
          </span>

          {/* Name */}
          <span className="font-semibold text-gray-800 truncate max-w-[120px]">
            {player.display_name}
            {isCurrentPlayer && (
              <span className="ml-1 text-xs text-blue-600">(You)</span>
            )}
          </span>
        </div>

        {/* Points */}
        <span className="font-bold text-lg">
          {player.points}
          <span className="text-xs text-gray-500 ml-1">pts</span>
        </span>
      </div>

      {/* Progress Bar */}
      <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
        <div
          className={`
            h-full transition-all duration-500 ease-out
            ${progressPercentage >= 100 ? "bg-green-500" : "bg-blue-500"}
          `}
          style={{ width: `${progressPercentage}%` }}
          data-testid="progress-bar"
        />
      </div>

      {/* Connection Status */}
      {!player.is_connected && (
        <div className="absolute top-1 right-1">
          <div
            className="w-2 h-2 bg-red-500 rounded-full"
            title="Disconnected"
          />
        </div>
      )}

      {/* Current Round Winner Badge */}
      {isCurrentWinner && (
        <div className="absolute -top-2 -right-2">
          <span className="bg-orange-500 text-white text-xs px-2 py-1 rounded-full font-semibold">
            Round Winner
          </span>
        </div>
      )}
    </div>
  );
};

export const PlayerList: React.FC<PlayerListProps> = ({
  players,
  currentPlayerId,
  currentWinnerId,
  pointThreshold,
}) => {
  // Sort players by points (descending)
  const sortedPlayers = [...players].sort((a, b) => b.points - a.points);

  return (
    <div className="flex flex-col gap-3" data-testid="player-list">
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-lg font-bold text-gray-800">Players</h3>
        <span className="text-sm text-gray-500">
          First to {pointThreshold} wins
        </span>
      </div>

      <div className="space-y-2">
        {sortedPlayers.map((player, index) => (
          <PlayerItem
            key={player.user_id}
            player={player}
            isCurrentPlayer={player.user_id === currentPlayerId}
            isCurrentWinner={player.user_id === currentWinnerId}
            pointThreshold={pointThreshold}
            rank={index + 1}
          />
        ))}
      </div>

      {players.length === 0 && (
        <div className="text-center py-8 text-gray-500">
          Waiting for players to join...
        </div>
      )}
    </div>
  );
};

// Container component that connects to store
export const PlayerListContainer: React.FC = () => {
  const { gameState } = useGameStore();
  const { user } = useAuthStore();

  if (!gameState) {
    return <div className="text-center py-8 text-gray-500">No active game</div>;
  }

  return (
    <PlayerList
      players={gameState.players}
      currentPlayerId={user?.id}
      currentWinnerId={gameState.current_winner}
      pointThreshold={gameState.point_threshold}
    />
  );
};

// Imports (will be at top in final version)
import { useGameStore } from "../../store/gameStore";
import { useAuthStore } from "../../store/authStore";
