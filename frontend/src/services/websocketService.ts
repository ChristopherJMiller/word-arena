import { ClientMessage, ServerMessage } from "../types/generated";

type MessageHandler = (message: ServerMessage) => void;

export class WebSocketService {
  private ws: WebSocket | null = null;
  private messageHandlers: Set<MessageHandler> = new Set();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectInterval = 1000;
  private isAuthenticated = false;
  private authToken: string | null = null;

  constructor(private url: string) {}

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log("WebSocket connected");
          this.reconnectAttempts = 0;
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const message: ServerMessage = JSON.parse(event.data);
            this.handleMessage(message);
          } catch (error) {
            console.error("Failed to parse WebSocket message:", error);
          }
        };

        this.ws.onclose = (event) => {
          console.log("WebSocket disconnected:", event.code, event.reason);
          this.isAuthenticated = false;
          this.handleReconnection();
        };

        this.ws.onerror = (error) => {
          console.error("WebSocket error:", error);
          reject(error);
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.isAuthenticated = false;
    this.authToken = null;
  }

  async authenticate(token: string): Promise<boolean> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket not connected");
    }

    return new Promise((resolve) => {
      this.authToken = token;

      // Set up one-time handler for auth response
      const authHandler = (message: ServerMessage) => {
        if (typeof message === "object" && message !== null) {
          if ("AuthenticationSuccess" in message) {
            this.isAuthenticated = true;
            this.removeMessageHandler(authHandler);
            resolve(true);
          } else if ("AuthenticationFailed" in message) {
            this.isAuthenticated = false;
            this.removeMessageHandler(authHandler);
            console.error(
              "Authentication failed:",
              message.AuthenticationFailed.reason,
            );
            resolve(false);
          }
        }
      };

      this.addMessageHandler(authHandler);
      this.sendMessage({ Authenticate: { token } });

      // Timeout after 10 seconds
      setTimeout(() => {
        this.removeMessageHandler(authHandler);
        resolve(false);
      }, 10000);
    });
  }

  sendMessage(message: ClientMessage) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket not connected");
    }

    console.log("[WebSocket] Sending client message:", JSON.stringify(message, null, 2));
    this.ws.send(JSON.stringify(message));
  }

  rejoinGame(gameId: string) {
    this.sendMessage({ RejoinGame: { game_id: gameId } });
  }

  addMessageHandler(handler: MessageHandler) {
    this.messageHandlers.add(handler);
  }

  removeMessageHandler(handler: MessageHandler) {
    this.messageHandlers.delete(handler);
  }

  private handleMessage(message: ServerMessage) {
    // Log incoming server messages for easier debugging
    console.log("[WebSocket] Received server message:", JSON.stringify(message, null, 2));
    
    this.messageHandlers.forEach((handler) => {
      try {
        handler(message);
      } catch (error) {
        console.error("Error in message handler:", error);
      }
    });
  }

  private async handleReconnection() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error("Max reconnection attempts reached");
      return;
    }

    this.reconnectAttempts++;
    console.log(
      `Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`,
    );

    setTimeout(async () => {
      try {
        await this.connect();

        // Re-authenticate if we had a token
        if (this.authToken) {
          await this.authenticate(this.authToken);
        }
      } catch (error) {
        console.error("Reconnection failed:", error);
        this.handleReconnection();
      }
    }, this.reconnectInterval * this.reconnectAttempts);
  }

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  get authenticated(): boolean {
    return this.isAuthenticated;
  }
}

// Singleton instance
let wsService: WebSocketService | null = null;

export function getWebSocketService(): WebSocketService {
  if (!wsService) {
    const wsUrl = import.meta.env.VITE_WS_URL || "ws://localhost:8080/ws";
    wsService = new WebSocketService(wsUrl);
  }
  return wsService;
}
