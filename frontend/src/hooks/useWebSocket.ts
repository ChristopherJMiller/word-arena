import { useEffect, useRef, useState } from "react";
import { useAuth } from "../components/auth/AuthProvider";
import { getWebSocketService } from "../services/websocketService";
import { ServerMessage } from "../types/generated";

export function useWebSocket() {
  const { hasMsalAuth, getAccessToken, handleSessionConflict } = useAuth();
  const [isConnected, setIsConnected] = useState(false);
  const [isWSAuthenticated, setIsWSAuthenticated] = useState(false);
  const wsService = useRef(getWebSocketService());

  // Sync local state with WebSocket service state
  useEffect(() => {
    const checkConnectionState = () => {
      const serviceConnected = wsService.current.isConnected;
      const serviceAuthenticated = wsService.current.authenticated;

      if (isConnected !== serviceConnected) {
        console.log("Syncing connection state:", serviceConnected);
        setIsConnected(serviceConnected);
      }
      if (isWSAuthenticated !== serviceAuthenticated) {
        console.log("Syncing authentication state:", serviceAuthenticated);
        setIsWSAuthenticated(serviceAuthenticated);
      }
    };

    // Check state every second to stay in sync
    const interval = setInterval(checkConnectionState, 1000);

    // Also check immediately
    checkConnectionState();

    return () => clearInterval(interval);
  }, [isConnected, isWSAuthenticated]);

  // Set up handler for session disconnection
  useEffect(() => {
    wsService.current.setSessionDisconnectedHandler(() => {
      setIsWSAuthenticated(false);
      setIsConnected(false);
      // Show an alert to the user
      alert("Your session has been taken over by another login. This window will now be disconnected.");
    });
  }, []);

  useEffect(() => {
    const connectAndAuthenticate = async () => {
      try {
        // Connect to WebSocket
        if (!wsService.current.isConnected) {
          await wsService.current.connect();
          setIsConnected(true);
        }

        // Authenticate if user has MSAL token but WS is not authenticated
        if (hasMsalAuth && !isWSAuthenticated) {
          const token = await getAccessToken();
          if (token) {
            const authResult = await wsService.current.authenticate(token, false);
            
            if (authResult === 'conflict') {
              // Handle session conflict
              handleSessionConflict(async () => {
                // Force authenticate on retry
                const forceAuthResult = await wsService.current.authenticate(token, true);
                setIsWSAuthenticated(forceAuthResult === true);
              }, "You already have an active session in another browser.");
            } else if (authResult === true) {
              setIsWSAuthenticated(true);
            } else {
              console.error("WebSocket authentication failed");
              setIsWSAuthenticated(false);
            }
          }
        }
      } catch (error) {
        console.error("Failed to connect to WebSocket:", error);
        setIsConnected(false);
      }
    };

    // Auto-connect when user has MSAL auth
    if (hasMsalAuth) {
      connectAndAuthenticate();
    }

    // Disconnect when user logs out
    if (!hasMsalAuth && wsService.current.isConnected) {
      wsService.current.disconnect();
      setIsConnected(false);
      setIsWSAuthenticated(false);
    }
  }, [hasMsalAuth, getAccessToken, isWSAuthenticated]);

  const addMessageHandler = (handler: (message: ServerMessage) => void) => {
    wsService.current.addMessageHandler(handler);
  };

  const removeMessageHandler = (handler: (message: ServerMessage) => void) => {
    wsService.current.removeMessageHandler(handler);
  };

  const sendMessage = (message: any) => {
    if (!isConnected || !isWSAuthenticated) {
      throw new Error("WebSocket not connected or not authenticated");
    }
    wsService.current.sendMessage(message);
  };

  return {
    isConnected,
    isAuthenticated: isWSAuthenticated,
    user: wsService.current.user,
    sendMessage,
    addMessageHandler,
    removeMessageHandler,
  };
}
