import { useEffect, useRef, useState } from 'react'
import { useAuth } from '../components/auth/AuthProvider'
import { getWebSocketService } from '../services/websocketService'
import { ServerMessage } from '../types/generated'

export function useWebSocket() {
  const { isAuthenticated, getAccessToken } = useAuth()
  const [isConnected, setIsConnected] = useState(false)
  const [isWSAuthenticated, setIsWSAuthenticated] = useState(false)
  const wsService = useRef(getWebSocketService())

  useEffect(() => {
    const connectAndAuthenticate = async () => {
      try {
        // Connect to WebSocket
        if (!wsService.current.isConnected) {
          await wsService.current.connect()
          setIsConnected(true)
        }

        // Authenticate if user is logged in but WS is not authenticated
        if (isAuthenticated && !isWSAuthenticated) {
          const token = await getAccessToken()
          if (token) {
            const authSuccess = await wsService.current.authenticate(token)
            setIsWSAuthenticated(authSuccess)
            
            if (!authSuccess) {
              console.error('WebSocket authentication failed')
            }
          }
        }
      } catch (error) {
        console.error('Failed to connect to WebSocket:', error)
        setIsConnected(false)
      }
    }

    // Auto-connect when user is authenticated
    if (isAuthenticated) {
      connectAndAuthenticate()
    }

    // Disconnect when user logs out
    if (!isAuthenticated && wsService.current.isConnected) {
      wsService.current.disconnect()
      setIsConnected(false)
      setIsWSAuthenticated(false)
    }
  }, [isAuthenticated, getAccessToken, isWSAuthenticated])

  const addMessageHandler = (handler: (message: ServerMessage) => void) => {
    wsService.current.addMessageHandler(handler)
  }

  const removeMessageHandler = (handler: (message: ServerMessage) => void) => {
    wsService.current.removeMessageHandler(handler)
  }

  const sendMessage = (message: any) => {
    if (!isConnected || !isWSAuthenticated) {
      throw new Error('WebSocket not connected or not authenticated')
    }
    wsService.current.sendMessage(message)
  }

  return {
    isConnected,
    isAuthenticated: isWSAuthenticated,
    sendMessage,
    addMessageHandler,
    removeMessageHandler,
  }
}