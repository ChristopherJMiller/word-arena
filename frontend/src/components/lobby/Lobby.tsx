import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthProvider";
import { useWebSocket } from "../../hooks/useWebSocket";
import { useGameStore } from "../../store/gameStore";
import { gameHttpClient } from "../../services/gameHttpClient";
import type { User } from "../../types/generated/User";
import type { ServerMessage } from "../../types/generated/ServerMessage";

const Lobby: React.FC = () => {
  const navigate = useNavigate();
  const { isAuthenticated } = useAuth();
  const {
    isConnected,
    isAuthenticated: isWSAuthenticated,
    sendMessage,
    addMessageHandler,
    removeMessageHandler,
  } = useWebSocket();
  const { gameId, reconnectToGame, resetGame } = useGameStore();
  const [queuePosition, setQueuePosition] = useState<number | null>(null);
  const [isInQueue, setIsInQueue] = useState(false);
  const [isCheckingActiveGame, setIsCheckingActiveGame] = useState(false);
  const [hasValidatedGameId, setHasValidatedGameId] = useState(false);
  const [gameIdIsValid, setGameIdIsValid] = useState(false);
  const [countdownInfo, setCountdownInfo] = useState<{
    secondsRemaining: number;
    playersReady: number;
    totalPlayers: number;
  } | null>(null);
  const [localCountdown, setLocalCountdown] = useState<number>(0);

  useEffect(() => {
    // Handle WebSocket messages
    const messageHandler = (message: ServerMessage) => {
      if (typeof message === "object" && message !== null) {
        if ("QueueJoined" in message) {
          setIsInQueue(true);
          setQueuePosition(message.QueueJoined.position);
        } else if ("QueueLeft" in message) {
          setIsInQueue(false);
          setQueuePosition(null);
          setCountdownInfo(null);
          setLocalCountdown(0);
        } else if ("MatchFound" in message) {
          setIsInQueue(false);
          setQueuePosition(null);
          setCountdownInfo(null);
          setLocalCountdown(0);
          // Navigate to the game page
          navigate(`/game/${message.MatchFound.game_id}`);
          console.log(
            "Match found, navigating to game:",
            message.MatchFound.game_id,
          );
        } else if ("MatchmakingCountdown" in message) {
          const countdown = message.MatchmakingCountdown;
          setCountdownInfo({
            secondsRemaining: countdown.seconds_remaining,
            playersReady: countdown.players_ready,
            totalPlayers: countdown.total_players,
          });
          setLocalCountdown(countdown.seconds_remaining);
          console.log("Countdown update:", countdown);
        } else if ("Error" in message) {
          console.error("Server error:", message.Error.message);
          // Show error to user
        }
      }
    };

    if (isWSAuthenticated) {
      addMessageHandler(messageHandler);
    }

    return () => {
      if (isWSAuthenticated) {
        removeMessageHandler(messageHandler);
      }
    };
  }, [isWSAuthenticated, addMessageHandler, removeMessageHandler]);

  // Local countdown timer
  useEffect(() => {
    if (localCountdown > 0) {
      const timer = setInterval(() => {
        setLocalCountdown((prev) => {
          const newValue = prev - 1;
          return newValue <= 0 ? 0 : newValue;
        });
      }, 1000);

      return () => clearInterval(timer);
    }
  }, [localCountdown]);

  // Validate stored gameId on component mount
  useEffect(() => {
    const validateGameId = async () => {
      if (!gameId || !isAuthenticated) {
        setHasValidatedGameId(true);
        setGameIdIsValid(false);
        return;
      }

      try {
        // Try to fetch the game state to validate it exists
        await gameHttpClient.getGameState(gameId);
        setGameIdIsValid(true);
        console.log(`Validated existing game session: ${gameId}`);
      } catch (error) {
        console.log(`Game ${gameId} no longer exists, clearing from localStorage`);
        setGameIdIsValid(false);
        // Clear the invalid game ID from storage
        resetGame();
      } finally {
        setHasValidatedGameId(true);
      }
    };

    // Only validate once when component mounts and user is authenticated
    if (!hasValidatedGameId && isAuthenticated) {
      validateGameId();
    }
  }, [gameId, isAuthenticated, hasValidatedGameId, resetGame]);

  const handleJoinQueue = () => {
    if (!isWSAuthenticated) return;

    try {
      if (isInQueue) {
        sendMessage("LeaveQueue");
      } else {
        sendMessage("JoinQueue");
      }
    } catch (error) {
      console.error("Failed to send queue message:", error);
    }
  };

  const handleRejoinGame = async () => {
    if (!gameId || !isWSAuthenticated) return;

    setIsCheckingActiveGame(true);
    try {
      await reconnectToGame(gameId);
      navigate(`/game/${gameId}`);
    } catch (error) {
      console.error("Failed to rejoin game:", error);
      // If we can't rejoin, the game probably ended - clear the stored gameId
      resetGame();
    } finally {
      setIsCheckingActiveGame(false);
    }
  };

  const handleClearActiveGame = () => {
    resetGame();
  };

  const handleVoteToStart = () => {
    if (!isWSAuthenticated || !isInQueue) return;

    try {
      sendMessage("VoteStartGame");
      console.log("Voting to start game early");
    } catch (error) {
      console.error("Failed to vote to start:", error);
    }
  };

  // Mock data for testing
  const mockLeaderboard: User[] = [
    {
      id: "1",
      email: "player1@example.com",
      display_name: "Player One",
      total_points: 150,
      total_wins: 5,
      total_games: 12,
      created_at: new Date().toISOString(),
    },
    {
      id: "2",
      email: "player2@example.com",
      display_name: "Player Two",
      total_points: 120,
      total_wins: 3,
      total_games: 8,
      created_at: new Date().toISOString(),
    },
  ];

  return (
    <div className="max-w-4xl mx-auto">
      <div className="text-center mb-8">
        <h1 className="text-4xl font-bold text-gray-900 mb-4">
          Welcome to Word Arena
        </h1>
        <p className="text-lg text-gray-600">
          Collaborative Wordle where players compete for the best guesses!
        </p>
      </div>

      {/* Active Game Section - Loading State */}
      {gameId && isAuthenticated && !hasValidatedGameId && (
        <div className="card mb-6 bg-gray-50 border-gray-200">
          <h2 className="text-2xl font-semibold mb-4 text-gray-900">
            Checking Game Session
          </h2>
          <p className="text-gray-600 mb-4">
            Validating your game session...
          </p>
          <div className="flex items-center space-x-2">
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
            <span className="text-sm text-gray-500">Game ID: {gameId}</span>
          </div>
        </div>
      )}

      {/* Active Game Section */}
      {gameId && isAuthenticated && hasValidatedGameId && gameIdIsValid && (
        <div className="card mb-6 bg-blue-50 border-blue-200">
          <h2 className="text-2xl font-semibold mb-4 text-blue-900">
            Active Game
          </h2>
          <p className="text-blue-800 mb-4">
            You have an active game session. You can rejoin your game or start a
            new one.
          </p>
          <div className="flex gap-3">
            <button
              className="btn-primary flex-1"
              onClick={handleRejoinGame}
              disabled={!isWSAuthenticated || isCheckingActiveGame}
            >
              {isCheckingActiveGame ? "Checking..." : "Rejoin Game"}
            </button>
            <button
              className="btn-secondary"
              onClick={handleClearActiveGame}
              disabled={isCheckingActiveGame}
            >
              Clear
            </button>
          </div>
          <p className="text-xs text-blue-600 mt-2">Game ID: {gameId}</p>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Queue Section */}
        <div className="card">
          <h2 className="text-2xl font-semibold mb-4">Join Game</h2>
          <p className="text-gray-600 mb-4">
            Queue up to play with 2-16 other players in a collaborative Wordle
            match.
          </p>
          {!isAuthenticated ? (
            <div className="text-center p-4 bg-yellow-50 rounded-lg">
              <p className="text-yellow-800 mb-2">
                Please sign in to join a game
              </p>
            </div>
          ) : !isConnected || !isWSAuthenticated ? (
            <div className="text-center p-4 bg-red-50 rounded-lg">
              <p className="text-red-800 mb-2">Connecting to game server...</p>
            </div>
          ) : (
            <>
              <button
                className="btn-primary w-full"
                onClick={handleJoinQueue}
                disabled={!isWSAuthenticated}
              >
                {!isWSAuthenticated
                  ? "Connecting to server..."
                  : isInQueue
                    ? "Leave Queue"
                    : "Join Queue"}
              </button>
              {queuePosition && (
                <p className="text-sm text-gray-500 mt-2">
                  Position in queue: {queuePosition}
                </p>
              )}

              {/* Countdown and Vote UI */}
              {isInQueue && countdownInfo && (
                <div className="mt-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
                  <div className="flex items-center justify-between mb-3">
                    <div>
                      <h3 className="text-lg font-semibold text-blue-900">
                        Game Starting Soon!
                      </h3>
                      <p className="text-blue-700 text-sm">
                        {localCountdown > 0
                          ? `Starting in ${localCountdown} seconds`
                          : "Starting now..."}
                      </p>
                    </div>
                    <div className="text-right">
                      <p className="text-blue-800 font-medium">
                        {countdownInfo.playersReady}/
                        {countdownInfo.totalPlayers} ready
                      </p>
                      <p className="text-blue-600 text-xs">
                        Need 60% to start early
                      </p>
                    </div>
                  </div>

                  <button
                    className="btn-secondary w-full bg-blue-600 hover:bg-blue-700 text-white"
                    onClick={handleVoteToStart}
                  >
                    Vote to Start Now
                  </button>
                </div>
              )}

              {/* Waiting message when no countdown active */}
              {isInQueue && !countdownInfo && (
                <div className="mt-4 p-3 bg-gray-50 rounded-lg">
                  <p className="text-gray-600 text-center">
                    Waiting for players... Game will start when enough players
                    join
                  </p>
                </div>
              )}
            </>
          )}
        </div>

        {/* Leaderboard Section */}
        <div className="card">
          <h2 className="text-2xl font-semibold mb-4">Leaderboard</h2>
          <div className="space-y-2">
            {mockLeaderboard.map((user, index) => (
              <div
                key={user.id}
                className="flex items-center justify-between p-2 bg-gray-50 rounded"
              >
                <div className="flex items-center space-x-3">
                  <span className="font-bold text-lg">#{index + 1}</span>
                  <span className="font-medium">{user.display_name}</span>
                </div>
                <div className="text-right">
                  <div className="font-semibold">{user.total_points} pts</div>
                  <div className="text-sm text-gray-500">
                    {user.total_wins} wins
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Game Rules */}
      <div className="card mt-8">
        <h2 className="text-2xl font-semibold mb-4">How to Play</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 text-sm">
          <div>
            <h3 className="font-semibold text-blue-600 mb-2">Gameplay</h3>
            <ul className="space-y-1 text-gray-600">
              <li>• Players collaborate to solve Wordle puzzles</li>
              <li>• Submit guesses simultaneously during countdown</li>
              <li>• Best guess wins the round and appears on the board</li>
              <li>• Winning player gets individual guess, then repeat</li>
            </ul>
          </div>
          <div>
            <h3 className="font-semibold text-blue-600 mb-2">Scoring</h3>
            <ul className="space-y-1 text-gray-600">
              <li>
                •{" "}
                <span className="text-present font-medium">Orange letters</span>
                : 1 point
              </li>
              <li>
                • <span className="text-correct font-medium">Blue letters</span>
                : 2 points
              </li>
              <li>• Solving the word: 5 points</li>
              <li>• First to 25 points wins!</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Lobby;
