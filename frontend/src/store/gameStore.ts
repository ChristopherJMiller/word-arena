import { create } from 'zustand';
import type { GameState, PersonalGuess } from '../types/generated';
import { getWebSocketService } from '../services/websocketService';
import { gameHttpClient } from '../services/gameHttpClient';

interface GameStore {
  // Game state
  gameState: GameState | null;
  gameId: string | null;
  currentGuess: string;
  isSubmitting: boolean;
  countdownEndTime: number | undefined;
  isReconnecting: boolean;
  
  // Personal state
  personalGuessHistory: PersonalGuess[];
  
  // UI state
  lastError: string | null;
  pendingGuess: string | null;
  
  // Actions
  setGameState: (state: GameState) => void;
  setGameId: (id: string) => void;
  setCurrentGuess: (guess: string) => void;
  setIsSubmitting: (submitting: boolean) => void;
  setCountdownEndTime: (time: number | undefined) => void;
  setIsReconnecting: (reconnecting: boolean) => void;
  addPersonalGuess: (guess: PersonalGuess) => void;
  setLastError: (error: string | null) => void;
  clearError: () => void;
  setPendingGuess: (guess: string | null) => void;
  reconnectToGame: (gameId: string) => Promise<void>;
  rejoinAfterDisconnect: (gameId: string) => Promise<void>;
  resetGame: () => void;
}

export const useGameStore = create<GameStore>((set) => ({
  // Initial state
  gameState: null,
  gameId: (() => {
    try {
      return localStorage.getItem('word-arena-game-id') || null;
    } catch {
      return null;
    }
  })(),
  currentGuess: '',
  isSubmitting: false,
  countdownEndTime: undefined,
  isReconnecting: false,
  personalGuessHistory: [],
  lastError: null,
  pendingGuess: null,
  
  // Actions
  setGameState: (state) => set({ gameState: state }),
  setGameId: (id) => {
    set({ gameId: id });
    try {
      if (id) {
        localStorage.setItem('word-arena-game-id', id);
      } else {
        localStorage.removeItem('word-arena-game-id');
      }
    } catch {
      // Ignore localStorage errors
    }
  },
  setCurrentGuess: (guess) => set({ currentGuess: guess }),
  setIsSubmitting: (submitting) => set({ isSubmitting: submitting }),
  setCountdownEndTime: (time) => set({ countdownEndTime: time }),
  setIsReconnecting: (reconnecting) => set({ isReconnecting: reconnecting }),
  addPersonalGuess: (guess) => set((state) => ({
    personalGuessHistory: [...state.personalGuessHistory, guess]
  })),
  setLastError: (error) => set({ lastError: error }),
  clearError: () => set({ lastError: null }),
  setPendingGuess: (guess) => set({ pendingGuess: guess }),
  
  reconnectToGame: async (gameId: string) => {
    set({ isReconnecting: true });
    
    try {
      // First try to fetch game state via HTTP
      const safeGameState = await gameHttpClient.getGameState(gameId);
      
      // Convert SafeGameState to GameState format (we'll need to add the missing word field)
      const gameState: GameState = {
        ...safeGameState,
        word: '', // We don't get the word from safe state - it will be updated via WebSocket
      };
      
      set({ 
        gameState,
        gameId,
        isReconnecting: false 
      });
      
      // Ensure WebSocket is connected but don't send rejoin message
      // The game state fetch already confirms we have access to this game
      try {
        const wsService = getWebSocketService();
        if (!wsService.isConnected) {
          await wsService.connect();
        }
      } catch (wsError) {
        console.warn('WebSocket connection failed during reconnect, continuing without WS:', wsError);
      }
      
      console.log('Successfully loaded game state for:', gameId);
    } catch (error) {
      console.error('Error loading game state:', error);
      set({ isReconnecting: false });
      // If we can't fetch the game state, we're not in this game
      throw error;
    }
  },

  rejoinAfterDisconnect: async (gameId: string) => {
    set({ isReconnecting: true });
    
    try {
      // For actual disconnection scenarios, use WebSocket rejoin
      const wsService = getWebSocketService();
      if (!wsService.isConnected) {
        await wsService.connect();
      }
      
      // Wait for authentication before sending rejoin
      if (wsService.isConnected && wsService.authenticated) {
        wsService.rejoinGame(gameId);
        console.log('Sent rejoin message for game:', gameId);
      } else {
        console.log('WebSocket not authenticated yet, skipping rejoin');
      }
      
      // The game state will be updated via WebSocket GameStateUpdate message
      set({ isReconnecting: false });
    } catch (error) {
      console.error('Error rejoining game:', error);
      set({ isReconnecting: false });
      throw error;
    }
  },
  
  resetGame: () => {
    set({
      gameState: null,
      gameId: null,
      currentGuess: '',
      isSubmitting: false,
      countdownEndTime: undefined,
      isReconnecting: false,
      personalGuessHistory: [],
      lastError: null,
      pendingGuess: null
    });
    try {
      localStorage.removeItem('word-arena-game-id');
    } catch {
      // Ignore localStorage errors
    }
  },
}));