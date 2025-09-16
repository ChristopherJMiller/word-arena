import React from "react";
import { useNavigate } from "react-router-dom";

interface GameNotFoundProps {
  gameId?: string;
}

export const GameNotFound: React.FC<GameNotFoundProps> = ({ gameId }) => {
  const navigate = useNavigate();

  const handleReturnToLobby = () => {
    navigate("/");
  };

  return (
    <div className="min-h-screen bg-gray-50 flex items-center justify-center">
      <div className="bg-white rounded-lg shadow-md p-8 text-center max-w-md">
        <div className="text-6xl mb-4">😕</div>
        <h1 className="text-2xl font-bold text-gray-800 mb-4">
          Game Not Found
        </h1>

        {gameId && (
          <p className="text-gray-600 mb-2 text-sm font-mono bg-gray-100 px-3 py-1 rounded">
            Game ID: {gameId}
          </p>
        )}

        <p className="text-gray-600 mb-6">
          This game doesn't exist or may have expired. Games are automatically
          cleaned up after a period of inactivity.
        </p>

        <button
          onClick={handleReturnToLobby}
          className="bg-blue-500 hover:bg-blue-600 text-white px-6 py-3 rounded-lg font-semibold transition-colors"
        >
          Return to Lobby
        </button>
      </div>
    </div>
  );
};
