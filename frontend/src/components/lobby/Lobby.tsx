import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth/AuthProvider'
import { useWebSocket } from '../../hooks/useWebSocket'
import type { User } from '../../types/generated/User'
import type { ServerMessage } from '../../types/generated/ServerMessage'

const Lobby: React.FC = () => {
  const navigate = useNavigate()
  const { isAuthenticated } = useAuth()
  const { isConnected, isAuthenticated: isWSAuthenticated, sendMessage, addMessageHandler, removeMessageHandler } = useWebSocket()
  const [queuePosition, setQueuePosition] = useState<number | null>(null)
  const [isInQueue, setIsInQueue] = useState(false)

  useEffect(() => {
    // Handle WebSocket messages
    const messageHandler = (message: ServerMessage) => {
      if (typeof message === 'object' && message !== null) {
        if ('QueueJoined' in message) {
          setIsInQueue(true)
          setQueuePosition(message.QueueJoined.position)
        } else if ('QueueLeft' in message) {
          setIsInQueue(false)
          setQueuePosition(null)
        } else if ('MatchFound' in message) {
          setIsInQueue(false)
          setQueuePosition(null)
          // Navigate to the game page
          navigate(`/game/${message.MatchFound.game_id}`)
          console.log('Match found, navigating to game:', message.MatchFound.game_id)
        } else if ('Error' in message) {
          console.error('Server error:', message.Error.message)
          // Show error to user
        }
      }
    }

    if (isWSAuthenticated) {
      addMessageHandler(messageHandler)
    }

    return () => {
      if (isWSAuthenticated) {
        removeMessageHandler(messageHandler)
      }
    }
  }, [isWSAuthenticated, addMessageHandler, removeMessageHandler])

  const handleJoinQueue = () => {
    if (!isWSAuthenticated) return

    try {
      if (isInQueue) {
        sendMessage('LeaveQueue')
      } else {
        sendMessage('JoinQueue')
      }
    } catch (error) {
      console.error('Failed to send queue message:', error)
    }
  }

  // Mock data for testing
  const mockLeaderboard: User[] = [
    {
      id: "1",
      email: "player1@example.com",
      display_name: "Player One",
      total_points: 150,
      total_wins: 5,
      created_at: new Date().toISOString()
    },
    {
      id: "2", 
      email: "player2@example.com",
      display_name: "Player Two",
      total_points: 120,
      total_wins: 3,
      created_at: new Date().toISOString()
    }
  ]

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

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Queue Section */}
        <div className="card">
          <h2 className="text-2xl font-semibold mb-4">Join Game</h2>
          <p className="text-gray-600 mb-4">
            Queue up to play with 2-16 other players in a collaborative Wordle match.
          </p>
          {!isAuthenticated ? (
            <div className="text-center p-4 bg-yellow-50 rounded-lg">
              <p className="text-yellow-800 mb-2">Please sign in to join a game</p>
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
                disabled={isInQueue}
              >
                {isInQueue ? 'Leave Queue' : 'Join Queue'}
              </button>
              {queuePosition && (
                <p className="text-sm text-gray-500 mt-2">
                  Position in queue: {queuePosition}
                </p>
              )}
            </>
          )}
        </div>

        {/* Leaderboard Section */}
        <div className="card">
          <h2 className="text-2xl font-semibold mb-4">Leaderboard</h2>
          <div className="space-y-2">
            {mockLeaderboard.map((user, index) => (
              <div key={user.id} className="flex items-center justify-between p-2 bg-gray-50 rounded">
                <div className="flex items-center space-x-3">
                  <span className="font-bold text-lg">#{index + 1}</span>
                  <span className="font-medium">{user.display_name}</span>
                </div>
                <div className="text-right">
                  <div className="font-semibold">{user.total_points} pts</div>
                  <div className="text-sm text-gray-500">{user.total_wins} wins</div>
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
              <li>• <span className="text-present font-medium">Orange letters</span>: 1 point</li>
              <li>• <span className="text-correct font-medium">Blue letters</span>: 2 points</li>
              <li>• Solving the word: 5 points</li>
              <li>• First to 25 points wins!</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}

export default Lobby