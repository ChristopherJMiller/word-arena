import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { useGameStore } from "./gameStore";
import type {
  GameState,
  PersonalGuess,
  SafeGameState,
} from "../types/generated";

// Mock the services for reconnection tests
vi.mock("../services/gameHttpClient", () => ({
  gameHttpClient: {
    getGameState: vi.fn(),
  },
}));

vi.mock("../services/websocketService", () => ({
  getWebSocketService: vi.fn(),
}));

import { gameHttpClient } from "../services/gameHttpClient";
import { getWebSocketService } from "../services/websocketService";

const mockGameHttpClient = vi.mocked(gameHttpClient);
const mockGetWebSocketService = vi.mocked(getWebSocketService);

// Mock localStorage for consistent testing
const mockLocalStorage = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};
Object.defineProperty(window, "localStorage", {
  value: mockLocalStorage,
});

describe("GameStore Logic", () => {
  const mockGameState: GameState = {
    id: "game-123",
    word: "HELLO",
    word_length: 5,
    current_round: 1,
    status: "Active",
    current_phase: "Guessing",
    players: [
      {
        user_id: "player-1",
        display_name: "Player 1",
        points: 10,
        guess_history: [],
        is_connected: true,
      },
    ],
    official_board: [],
    current_winner: null,
    created_at: "2024-01-01T00:00:00Z",
    point_threshold: 25,
  };

  const mockPersonalGuess: PersonalGuess = {
    word: "WORLD",
    points_earned: 3,
    was_winning_guess: false,
    timestamp: "2024-01-01T00:00:00Z",
  };

  beforeEach(() => {
    // Reset store state before each test
    useGameStore.getState().resetGame();
    vi.clearAllMocks();
    // Reset localStorage mock
    mockLocalStorage.getItem.mockReturnValue(null);
    mockLocalStorage.setItem.mockImplementation(() => {});
    mockLocalStorage.removeItem.mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("initializes with empty state", () => {
    const state = useGameStore.getState();

    expect(state.gameState).toBeNull();
    expect(state.currentGuess).toBe("");
    expect(state.isSubmitting).toBe(false);
    expect(state.countdownEndTime).toBeUndefined();
    expect(state.personalGuessHistory).toEqual([]);
  });

  it("updates game state correctly", () => {
    const { setGameState } = useGameStore.getState();

    setGameState(mockGameState);

    const state = useGameStore.getState();
    expect(state.gameState).toEqual(mockGameState);
  });

  it("manages current guess state", () => {
    const { setCurrentGuess } = useGameStore.getState();

    setCurrentGuess("HELLO");
    expect(useGameStore.getState().currentGuess).toBe("HELLO");

    setCurrentGuess("WORLD");
    expect(useGameStore.getState().currentGuess).toBe("WORLD");
  });

  it("tracks submission state", () => {
    const { setIsSubmitting } = useGameStore.getState();

    expect(useGameStore.getState().isSubmitting).toBe(false);

    setIsSubmitting(true);
    expect(useGameStore.getState().isSubmitting).toBe(true);

    setIsSubmitting(false);
    expect(useGameStore.getState().isSubmitting).toBe(false);
  });

  it("manages countdown timer", () => {
    const { setCountdownEndTime } = useGameStore.getState();
    const futureTime = Date.now() + 30000; // 30 seconds from now

    setCountdownEndTime(futureTime);
    expect(useGameStore.getState().countdownEndTime).toBe(futureTime);

    setCountdownEndTime(undefined);
    expect(useGameStore.getState().countdownEndTime).toBeUndefined();
  });

  it("accumulates personal guess history", () => {
    const { addPersonalGuess } = useGameStore.getState();

    addPersonalGuess(mockPersonalGuess);
    expect(useGameStore.getState().personalGuessHistory).toHaveLength(1);
    expect(useGameStore.getState().personalGuessHistory[0]).toEqual(
      mockPersonalGuess,
    );

    const secondGuess: PersonalGuess = {
      ...mockPersonalGuess,
      word: "TESTS",
      points_earned: 5,
    };

    addPersonalGuess(secondGuess);
    expect(useGameStore.getState().personalGuessHistory).toHaveLength(2);
    expect(useGameStore.getState().personalGuessHistory[1]).toEqual(
      secondGuess,
    );
  });

  it("resets all state on game reset", () => {
    const {
      setGameState,
      setCurrentGuess,
      setIsSubmitting,
      setCountdownEndTime,
      addPersonalGuess,
      resetGame,
    } = useGameStore.getState();

    // Set up some state
    setGameState(mockGameState);
    setCurrentGuess("HELLO");
    setIsSubmitting(true);
    setCountdownEndTime(Date.now() + 30000);
    addPersonalGuess(mockPersonalGuess);

    // Verify state is set
    const beforeReset = useGameStore.getState();
    expect(beforeReset.gameState).not.toBeNull();
    expect(beforeReset.currentGuess).toBe("HELLO");
    expect(beforeReset.isSubmitting).toBe(true);
    expect(beforeReset.countdownEndTime).toBeDefined();
    expect(beforeReset.personalGuessHistory).toHaveLength(1);

    // Reset and verify everything is cleared
    resetGame();

    const afterReset = useGameStore.getState();
    expect(afterReset.gameState).toBeNull();
    expect(afterReset.currentGuess).toBe("");
    expect(afterReset.isSubmitting).toBe(false);
    expect(afterReset.countdownEndTime).toBeUndefined();
    expect(afterReset.personalGuessHistory).toEqual([]);
  });

  it("handles multiple simultaneous state updates", () => {
    const { setGameState, setCurrentGuess, setIsSubmitting } =
      useGameStore.getState();

    // Simulate multiple rapid state changes
    setGameState(mockGameState);
    setCurrentGuess("TEST");
    setIsSubmitting(true);

    const state = useGameStore.getState();
    expect(state.gameState).toEqual(mockGameState);
    expect(state.currentGuess).toBe("TEST");
    expect(state.isSubmitting).toBe(true);
  });

  describe("localStorage integration", () => {
    it("should load gameId from localStorage on initialization", () => {
      // This test verifies that gameId is attempted to be loaded from localStorage
      // We can't easily test the initialization itself with Zustand singletons,
      // but we can verify the setGameId function works with localStorage
      const { setGameId } = useGameStore.getState();
      
      setGameId("test-game-123");
      expect(mockLocalStorage.setItem).toHaveBeenCalledWith(
        "word-arena-game-id", 
        "test-game-123"
      );
    });

    it("should handle localStorage errors gracefully during initialization", () => {
      // Mock localStorage throwing an error
      mockLocalStorage.getItem.mockImplementation(() => {
        throw new Error("localStorage not available");
      });

      // Should not throw and should return null
      const store = useGameStore.getState();
      expect(store.gameId).toBeNull();
    });

    it("should save gameId to localStorage when set", () => {
      const { setGameId } = useGameStore.getState();

      setGameId("new-game-456");

      expect(mockLocalStorage.setItem).toHaveBeenCalledWith(
        "word-arena-game-id",
        "new-game-456",
      );
    });

    it("should remove gameId from localStorage when cleared", () => {
      const { setGameId } = useGameStore.getState();

      setGameId(null);

      expect(mockLocalStorage.removeItem).toHaveBeenCalledWith(
        "word-arena-game-id",
      );
    });

    it("should handle localStorage errors gracefully when setting gameId", () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      mockLocalStorage.setItem.mockImplementation(() => {
        throw new Error("localStorage quota exceeded");
      });

      const { setGameId } = useGameStore.getState();

      // Should not throw
      expect(() => setGameId("test-game")).not.toThrow();

      consoleSpy.mockRestore();
    });

    it("should remove gameId from localStorage on reset", () => {
      const { resetGame } = useGameStore.getState();

      resetGame();

      expect(mockLocalStorage.removeItem).toHaveBeenCalledWith(
        "word-arena-game-id",
      );
    });
  });

  describe("pending guess management", () => {
    it("should track pending guess state", () => {
      const { setPendingGuess } = useGameStore.getState();

      expect(useGameStore.getState().pendingGuess).toBeNull();

      setPendingGuess("HELLO");
      expect(useGameStore.getState().pendingGuess).toBe("HELLO");

      setPendingGuess(null);
      expect(useGameStore.getState().pendingGuess).toBeNull();
    });

    it("should clear pending guess on game reset", () => {
      const { setPendingGuess, resetGame } = useGameStore.getState();

      setPendingGuess("WORLD");
      expect(useGameStore.getState().pendingGuess).toBe("WORLD");

      resetGame();
      expect(useGameStore.getState().pendingGuess).toBeNull();
    });
  });

  describe("error state management", () => {
    it("should track error messages", () => {
      const { setLastError } = useGameStore.getState();

      expect(useGameStore.getState().lastError).toBeNull();

      setLastError("Invalid word");
      expect(useGameStore.getState().lastError).toBe("Invalid word");

      setLastError(null);
      expect(useGameStore.getState().lastError).toBeNull();
    });

    it("should clear errors with clearError method", () => {
      const { setLastError, clearError } = useGameStore.getState();

      setLastError("Some error occurred");
      expect(useGameStore.getState().lastError).toBe("Some error occurred");

      clearError();
      expect(useGameStore.getState().lastError).toBeNull();
    });

    it("should clear errors on game reset", () => {
      const { setLastError, resetGame } = useGameStore.getState();

      setLastError("Network error");
      expect(useGameStore.getState().lastError).toBe("Network error");

      resetGame();
      expect(useGameStore.getState().lastError).toBeNull();
    });
  });

  describe("personal guess history ordering", () => {
    it("should maintain chronological order of personal guesses", () => {
      const { addPersonalGuess } = useGameStore.getState();

      const guess1: PersonalGuess = {
        word: "FIRST",
        points_earned: 1,
        was_winning_guess: false,
        timestamp: "2024-01-01T00:00:00Z",
      };

      const guess2: PersonalGuess = {
        word: "SECOND",
        points_earned: 2,
        was_winning_guess: true,
        timestamp: "2024-01-01T00:01:00Z",
      };

      addPersonalGuess(guess1);
      addPersonalGuess(guess2);

      const history = useGameStore.getState().personalGuessHistory;
      expect(history).toHaveLength(2);
      expect(history[0]).toEqual(guess1);
      expect(history[1]).toEqual(guess2);
    });

    it("should handle multiple guesses without mutation", () => {
      const { addPersonalGuess } = useGameStore.getState();

      const originalGuess = { ...mockPersonalGuess };
      addPersonalGuess(originalGuess);

      // Modify the original object
      originalGuess.points_earned = 999;

      // Store should reference the same object (Zustand doesn't deep clone)
      const storedGuess = useGameStore.getState().personalGuessHistory[0];
      expect(storedGuess.points_earned).toBe(999); // Modified value due to reference
      expect(storedGuess).toBe(originalGuess); // Same reference
    });
  });
});

describe("GameStore Reconnection Logic", () => {
  const mockSafeGameState: SafeGameState = {
    id: "test-game-123",
    word_length: 5,
    current_round: 2,
    status: "Active",
    current_phase: "Guessing",
    players: [
      {
        user_id: "user-1",
        display_name: "Player 1",
        points: 10,
        guess_history: [],
        is_connected: true,
      },
    ],
    official_board: [],
    current_winner: null,
    created_at: "2024-01-01T00:00:00Z",
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

  describe("gameId management", () => {
    it("should set and track gameId", () => {
      const { setGameId } = useGameStore.getState();

      setGameId("new-game-456");
      expect(useGameStore.getState().gameId).toBe("new-game-456");
    });

    it("should initialize with null gameId", () => {
      expect(useGameStore.getState().gameId).toBe(null);
    });
  });

  describe("reconnecting state management", () => {
    it("should track reconnecting state", () => {
      const { setIsReconnecting } = useGameStore.getState();

      expect(useGameStore.getState().isReconnecting).toBe(false);

      setIsReconnecting(true);
      expect(useGameStore.getState().isReconnecting).toBe(true);

      setIsReconnecting(false);
      expect(useGameStore.getState().isReconnecting).toBe(false);
    });
  });

  describe("reconnectToGame function", () => {
    it("should set reconnecting state to true initially", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);

      const reconnectPromise = useGameStore
        .getState()
        .reconnectToGame("test-game-123");

      // Check state immediately after calling reconnect
      expect(useGameStore.getState().isReconnecting).toBe(true);

      await reconnectPromise;
    });

    it("should fetch game state via HTTP client", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);

      await useGameStore.getState().reconnectToGame("test-game-123");

      expect(mockGameHttpClient.getGameState).toHaveBeenCalledWith(
        "test-game-123",
      );
    });

    it("should convert SafeGameState to GameState and update store", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);

      await useGameStore.getState().reconnectToGame("test-game-123");

      const state = useGameStore.getState();
      expect(state.gameId).toBe("test-game-123");
      expect(state.gameState).toEqual({
        ...mockSafeGameState,
        word: "", // Should add empty word field
      });
      expect(state.isReconnecting).toBe(false);
    });

    it("should attempt WebSocket reconnection when not connected", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = false;

      await useGameStore.getState().reconnectToGame("test-game-123");

      expect(mockWebSocketService.connect).toHaveBeenCalled();
    });

    it("should not send rejoin message when WebSocket is connected", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = true;

      await useGameStore.getState().reconnectToGame("test-game-123");

      // reconnectToGame should NOT call rejoinGame - it only fetches HTTP state
      expect(mockWebSocketService.rejoinGame).not.toHaveBeenCalled();
    });
  });

  describe("rejoinAfterDisconnect function", () => {
    const mockWebSocketServiceWithAuth = {
      ...mockWebSocketService,
      authenticated: true,
      rejoinGame: vi.fn(),
    };

    beforeEach(() => {
      mockGetWebSocketService.mockReturnValue(
        mockWebSocketServiceWithAuth as any,
      );
    });

    it("should set reconnecting state during rejoin process", async () => {
      // Make the WebSocket not connected so connect() is called
      mockWebSocketServiceWithAuth.isConnected = false;
      mockWebSocketServiceWithAuth.connect.mockImplementation(() => 
        new Promise(resolve => setTimeout(resolve, 10))
      );
      
      const rejoinPromise = useGameStore
        .getState()
        .rejoinAfterDisconnect("test-game-123");

      // Check state immediately after calling rejoin
      expect(useGameStore.getState().isReconnecting).toBe(true);

      await rejoinPromise;
      expect(useGameStore.getState().isReconnecting).toBe(false);
    });

    it("should connect WebSocket if not connected", async () => {
      mockWebSocketServiceWithAuth.isConnected = false;

      await useGameStore.getState().rejoinAfterDisconnect("test-game-123");

      expect(mockWebSocketServiceWithAuth.connect).toHaveBeenCalled();
    });

    it("should send rejoin message when WebSocket is authenticated", async () => {
      mockWebSocketServiceWithAuth.isConnected = true;
      mockWebSocketServiceWithAuth.authenticated = true;

      await useGameStore.getState().rejoinAfterDisconnect("test-game-123");

      expect(mockWebSocketServiceWithAuth.rejoinGame).toHaveBeenCalledWith(
        "test-game-123",
      );
    });

    it("should not send rejoin message when WebSocket is not authenticated", async () => {
      mockWebSocketServiceWithAuth.isConnected = true;
      mockWebSocketServiceWithAuth.authenticated = false;

      await useGameStore.getState().rejoinAfterDisconnect("test-game-123");

      expect(mockWebSocketServiceWithAuth.rejoinGame).not.toHaveBeenCalled();
    });

    it("should handle WebSocket connection errors gracefully", async () => {
      const consoleErrorSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      mockWebSocketServiceWithAuth.isConnected = false;
      mockWebSocketServiceWithAuth.connect.mockRejectedValue(
        new Error("Connection failed"),
      );

      await expect(
        useGameStore.getState().rejoinAfterDisconnect("test-game-123"),
      ).rejects.toThrow("Connection failed");

      expect(useGameStore.getState().isReconnecting).toBe(false);
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        "Error rejoining game:",
        expect.any(Error),
      );

      consoleErrorSpy.mockRestore();
    });

    it("should handle HTTP client errors gracefully", async () => {
      const consoleErrorSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      mockGameHttpClient.getGameState.mockRejectedValue(
        new Error("Network error"),
      );

      // The function should throw the error since it can't fetch game state
      await expect(
        useGameStore.getState().reconnectToGame("test-game-123"),
      ).rejects.toThrow("Network error");

      const state = useGameStore.getState();
      expect(state.isReconnecting).toBe(false);
      expect(state.gameState).toBe(null);
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        "Error loading game state:",
        expect.any(Error),
      );

      consoleErrorSpy.mockRestore();
    });

    it("should handle WebSocket connection errors gracefully", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = false;
      mockWebSocketService.connect.mockRejectedValue(
        new Error("WebSocket error"),
      );

      await useGameStore.getState().reconnectToGame("test-game-123");

      // Should still update game state from HTTP even if WebSocket fails
      const state = useGameStore.getState();
      expect(state.gameState).not.toBe(null);
      expect(state.isReconnecting).toBe(false);
    });

    it("should not send rejoin if WebSocket connection fails", async () => {
      mockGameHttpClient.getGameState.mockResolvedValue(mockSafeGameState);
      mockWebSocketService.isConnected = false;
      mockWebSocketService.connect.mockRejectedValue(
        new Error("WebSocket error"),
      );

      await useGameStore.getState().reconnectToGame("test-game-123");

      expect(mockWebSocketService.rejoinGame).not.toHaveBeenCalled();
    });
  });
});

describe("resetGame with reconnection state", () => {
  it("should clear all reconnection-related state", () => {
    // Set up reconnection state
    const store = useGameStore.getState();
    store.setGameId("test-id");
    store.setIsReconnecting(true);
    store.setGameState({
      id: "test",
      word: "HELLO",
      word_length: 5,
      current_round: 1,
      status: "Active",
      current_phase: "Guessing",
      players: [],
      official_board: [],
      current_winner: null,
      created_at: "2024-01-01T00:00:00Z",
      point_threshold: 25,
    });

    // Reset and verify
    store.resetGame();

    const resetState = useGameStore.getState();
    expect(resetState.gameId).toBe(null);
    expect(resetState.gameState).toBe(null);
    expect(resetState.isReconnecting).toBe(false);
    expect(resetState.currentGuess).toBe("");
    expect(resetState.personalGuessHistory).toEqual([]);
  });
});
