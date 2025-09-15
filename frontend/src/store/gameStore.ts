import { create } from 'zustand';
import type { GameState, PersonalGuess, SafeGameState } from '../types/generated';
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
  
  // Actions
  setGameState: (state: GameState) => void;
  setGameId: (id: string) => void;
  setCurrentGuess: (guess: string) => void;
  setIsSubmitting: (submitting: boolean) => void;
  setCountdownEndTime: (time: number | undefined) => void;
  setIsReconnecting: (reconnecting: boolean) => void;
  addPersonalGuess: (guess: PersonalGuess) => void;
  reconnectToGame: (gameId: string) => Promise<void>;
  resetGame: () => void;
}

export const useGameStore = create<GameStore>((set) => ({
  // Initial state
  gameState: null,
  gameId: null,
  currentGuess: '',
  isSubmitting: false,
  countdownEndTime: undefined,
  isReconnecting: false,
  personalGuessHistory: [],
  
  // Actions
  setGameState: (state) => set({ gameState: state }),
  setGameId: (id) => set({ gameId: id }),
  setCurrentGuess: (guess) => set({ currentGuess: guess }),
  setIsSubmitting: (submitting) => set({ isSubmitting: submitting }),
  setCountdownEndTime: (time) => set({ countdownEndTime: time }),
  setIsReconnecting: (reconnecting) => set({ isReconnecting: reconnecting }),
  addPersonalGuess: (guess) => set((state) => ({
    personalGuessHistory: [...state.personalGuessHistory, guess]
  })),
  
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
      
      // Reconnect WebSocket and send rejoin message
      const wsService = getWebSocketService();
      if (!wsService.isConnected) {
        await wsService.connect();
        // Note: Authentication should be handled separately by auth store
      }
      
      if (wsService.isConnected) {
        wsService.rejoinGame(gameId);
        console.log('Sent rejoin message for game:', gameId);
      }
      
      console.log('Reconnected to game:', gameId);
    } catch (error) {
      console.error('Error reconnecting to game:', error);
      set({ isReconnecting: false });
    }
  },
  
  resetGame: () => set({
    gameState: null,
    gameId: null,
    currentGuess: '',
    isSubmitting: false,
    countdownEndTime: undefined,
    isReconnecting: false,
    personalGuessHistory: []
  }),
}));