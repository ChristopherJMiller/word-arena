import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { useGameStore } from './gameStore';
import type { GameState, PersonalGuess, SafeGameState } from '../types/generated';

// Mock the services for reconnection tests
vi.mock('../services/gameHttpClient', () => ({
  gameHttpClient: {
    getGameState: vi.fn(),
  },
}));

vi.mock('../services/websocketService', () => ({
  getWebSocketService: vi.fn(),
}));

import { gameHttpClient } from '../services/gameHttpClient';
import { getWebSocketService } from '../services/websocketService';

const mockGameHttpClient = vi.mocked(gameHttpClient);
const mockGetWebSocketService = vi.mocked(getWebSocketService);

describe('GameStore Logic', () => {
  const mockGameState: GameState = {
    id: 'game-123',
    word: 'HELLO',
    word_length: 5,
    current_round: 1,
    status: 'Active',
    current_phase: 'Guessing',
    players: [
      {
        user_id: 'player-1',
        display_name: 'Player 1',
        points: 10,
        guess_history: [],
        is_connected: true,
      }
    ],
    official_board: [],
    current_winner: null,
    created_at: '2024-01-01T00:00:00Z',
    point_threshold: 25,
  };


  const mockPersonalGuess: PersonalGuess = {
    word: 'WORLD',
    points_earned: 3,
    was_winning_guess: false,
    timestamp: '2024-01-01T00:00:00Z',
  };

  beforeEach(() => {
    // Reset store state before each test
    useGameStore.getState().resetGame();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with empty state', () => {
    const state = useGameStore.getState();
    
    expect(state.gameState).toBeNull();
    expect(state.currentGuess).toBe('');
    expect(state.isSubmitting).toBe(false);
    expect(state.countdownEndTime).toBeUndefined();
    expect(state.personalGuessHistory).toEqual([]);
  });

  it('updates game state correctly', () => {
    const { setGameState } = useGameStore.getState();
    
    setGameState(mockGameState);
    
    const state = useGameStore.getState();
    expect(state.gameState).toEqual(mockGameState);
  });

  it('manages current guess state', () => {
    const { setCurrentGuess } = useGameStore.getState();
    
    setCurrentGuess('HELLO');
    expect(useGameStore.getState().currentGuess).toBe('HELLO');
    
    setCurrentGuess('WORLD');
    expect(useGameStore.getState().currentGuess).toBe('WORLD');
  });

  it('tracks submission state', () => {
    const { setIsSubmitting } = useGameStore.getState();
    
    expect(useGameStore.getState().isSubmitting).toBe(false);
    
    setIsSubmitting(true);
    expect(useGameStore.getState().isSubmitting).toBe(true);
    
    setIsSubmitting(false);
    expect(useGameStore.getState().isSubmitting).toBe(false);
  });

  it('manages countdown timer', () => {
    const { setCountdownEndTime } = useGameStore.getState();
    const futureTime = Date.now() + 30000; // 30 seconds from now
    
    setCountdownEndTime(futureTime);
    expect(useGameStore.getState().countdownEndTime).toBe(futureTime);
    
    setCountdownEndTime(undefined);
    expect(useGameStore.getState().countdownEndTime).toBeUndefined();
  });

  it('accumulates personal guess history', () => {
    const { addPersonalGuess } = useGameStore.getState();
    
    addPersonalGuess(mockPersonalGuess);
    expect(useGameStore.getState().personalGuessHistory).toHaveLength(1);
    expect(useGameStore.getState().personalGuessHistory[0]).toEqual(mockPersonalGuess);
    
    const secondGuess: PersonalGuess = {
      ...mockPersonalGuess,
      word: 'TESTS',
      points_earned: 5,
    };
    
    addPersonalGuess(secondGuess);
    expect(useGameStore.getState().personalGuessHistory).toHaveLength(2);
    expect(useGameStore.getState().personalGuessHistory[1]).toEqual(secondGuess);
  });

  it('resets all state on game reset', () => {
    const { setGameState, setCurrentGuess, setIsSubmitting, setCountdownEndTime, addPersonalGuess, resetGame } = useGameStore.getState();
    
    // Set up some state
    setGameState(mockGameState);
    setCurrentGuess('HELLO');
    setIsSubmitting(true);
    setCountdownEndTime(Date.now() + 30000);
    addPersonalGuess(mockPersonalGuess);
    
    // Verify state is set
    const beforeReset = useGameStore.getState();
    expect(beforeReset.gameState).not.toBeNull();
    expect(beforeReset.currentGuess).toBe('HELLO');
    expect(beforeReset.isSubmitting).toBe(true);
    expect(beforeReset.countdownEndTime).toBeDefined();
    expect(beforeReset.personalGuessHistory).toHaveLength(1);
    
    // Reset and verify everything is cleared
    resetGame();
    
    const afterReset = useGameStore.getState();
    expect(afterReset.gameState).toBeNull();
    expect(afterReset.currentGuess).toBe('');
    expect(afterReset.isSubmitting).toBe(false);
    expect(afterReset.countdownEndTime).toBeUndefined();
    expect(afterReset.personalGuessHistory).toEqual([]);
  });

  it('handles multiple simultaneous state updates', () => {
    const { setGameState, setCurrentGuess, setIsSubmitting } = useGameStore.getState();
    
    // Simulate multiple rapid state changes
    setGameState(mockGameState);
    setCurrentGuess('TEST');
    setIsSubmitting(true);
    
    const state = useGameStore.getState();
    expect(state.gameState).toEqual(mockGameState);
    expect(state.currentGuess).toBe('TEST');
    expect(state.isSubmitting).toBe(true);
  });
});

describe('GameStore Reconnection Logic', () => {
  const mockSafeGameState: SafeGameState = {
    id: 'test-game-123',
    word_length: 5,
    current_round: 2,
    status: 'Active',
    current_phase: 'Guessing',
    players: [
      {
        user_id: 'user-1',
        display_name: 'Player 1',
        points: 10,
        guess_history: [],
        is_connected: true,
      },
    ],
    official_board: [],
    current_winner: null,
    created_at: '2024-01-01T00:00:00Z',
    point_threshold: 25,
  };

  const mockWebSocketService = {
    isConnected: true,
    connect: vi.fn().mockResolvedValue(undefined),
    rejoinGame: vi.fn(),
  };

  beforeEach(() => {
    useGameStore.getState().resetGame();
    vi.clearAllMocks();
    mockGetWebSocketService.mockReturnValue(mockWebSocketService as any);
  });

  describe('gameId management', () => {
    it('should set and track gameId', () => {
      const { setGameId } = useGameStore.getState();
      
      setGameId('new-game-456');
      expect(useGameStore.getState().gameId).toBe('new-game-456');
    });

    it('should initialize with null gameId', () => {
      expect(useGameStore.getState().gameId).toBe(null);
    });
  });

  describe('reconnecting state management', () => {
    it('should track reconnecting state', () => {
      const { setIsReconnecting } = useGameStore.getState();
      
      expect(useGameStore.getState().isReconnecting).toBe(false);
      
      setIsReconnecting(true);
      expect(useGameStore.getState().isReconnecting).toBe(true);
      
      setIsReconnecting(false);
      expect(useGameStore.getState().isReconnecting).toBe(false);
    });
  });

  describe('reconnectToGame function', () => {
    it('should set reconnecting state to true initially', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);

      const reconnectPromise = useGameStore.getState().reconnectToGame('test-game-123');
      
      // Check state immediately after calling reconnect
      expect(useGameStore.getState().isReconnecting).toBe(true);
      
      await reconnectPromise;
    });

    it('should fetch game state via HTTP client', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);

      await useGameStore.getState().reconnectToGame('test-game-123');

      expect(mockGameHttpClient.getGameState).toHaveBeenCalledWith('test-game-123');
    });

    it('should convert SafeGameState to GameState and update store', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);

      await useGameStore.getState().reconnectToGame('test-game-123');

      const state = useGameStore.getState();
      expect(state.gameId).toBe('test-game-123');
      expect(state.gameState).toEqual({
        ...mockSafeGameState,
        word: '', // Should add empty word field
      });
      expect(state.isReconnecting).toBe(false);
    });

    it('should attempt WebSocket reconnection when not connected', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = false;

      await useGameStore.getState().reconnectToGame('test-game-123');

      expect(mockWebSocketService.connect).toHaveBeenCalled();
    });

    it('should not send rejoin message when WebSocket is connected', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = true;

      await useGameStore.getState().reconnectToGame('test-game-123');

      // reconnectToGame should NOT call rejoinGame - it only fetches HTTP state
      expect(mockWebSocketService.rejoinGame).not.toHaveBeenCalled();
    });

    it('should handle HTTP client errors gracefully', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockGameHttpClient.getGameState.mockRejectedValue(new Error('Network error'));

      // The function should throw the error since it can't fetch game state
      await expect(useGameStore.getState().reconnectToGame('test-game-123')).rejects.toThrow('Network error');

      const state = useGameStore.getState();
      expect(state.isReconnecting).toBe(false);
      expect(state.gameState).toBe(null);
      expect(consoleErrorSpy).toHaveBeenCalledWith('Error loading game state:', expect.any(Error));
      
      consoleErrorSpy.mockRestore();
    });

    it('should handle WebSocket connection errors gracefully', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = false;
      mockWebSocketService.connect.mockRejectedValue(new Error('WebSocket error'));

      await useGameStore.getState().reconnectToGame('test-game-123');

      // Should still update game state from HTTP even if WebSocket fails
      const state = useGameStore.getState();
      expect(state.gameState).not.toBe(null);
      expect(state.isReconnecting).toBe(false);
    });

    it('should not send rejoin if WebSocket connection fails', async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = false;
      mockWebSocketService.connect.mockRejectedValue(new Error('WebSocket error'));

      await useGameStore.getState().reconnectToGame('test-game-123');

      expect(mockWebSocketService.rejoinGame).not.toHaveBeenCalled();
    });
  });

  describe('resetGame with reconnection state', () => {
    it('should clear all reconnection-related state', () => {
      // Set up reconnection state
      const store = useGameStore.getState();
      store.setGameId('test-id');
      store.setIsReconnecting(true);
      store.setGameState({
        id: 'test',
        word: 'HELLO',
        word_length: 5,
        current_round: 1,
        status: 'Active',
        current_phase: 'Guessing',
        players: [],
        official_board: [],
        current_winner: null,
        created_at: '2024-01-01T00:00:00Z',
        point_threshold: 25,
      });

      // Reset and verify
      store.resetGame();
      
      const resetState = useGameStore.getState();
      expect(resetState.gameId).toBe(null);
      expect(resetState.gameState).toBe(null);
      expect(resetState.isReconnecting).toBe(false);
      expect(resetState.currentGuess).toBe('');
      expect(resetState.personalGuessHistory).toEqual([]);
    });
  });
});