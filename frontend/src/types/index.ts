// Re-export generated types
export * from './generated';

// Additional frontend-specific types
export interface WebSocketState {
  isConnected: boolean;
  isConnecting: boolean;
  error: string | null;
  reconnectAttempts: number;
}

export interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  accessToken: string | null;
  isLoading: boolean;
  error: string | null;
}

// Import User type for reference
import type { User } from './generated';