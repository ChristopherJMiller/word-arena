import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, act } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Game } from "./Game";
import { useGameStore } from "../../store/gameStore";
import { getWebSocketService } from "../../services/websocketService";
import type {
  ServerMessage,
  GameState,
  PersonalGuess,
  Player,
  GuessResult,
  LetterResult,
} from "../../types/generated";

// Mock WebSocket service
vi.mock("../../services/websocketService", () => ({
  getWebSocketService: vi.fn(),
}));

// Mock Game components
vi.mock("./GameLayout", () => ({
  GameLayout: () => <div data-testid="game-layout">Game Layout</div>,
}));

vi.mock("./GameNotFound", () => ({
  GameNotFound: () => <div data-testid="game-not-found">Game Not Found</div>,
}));

// Mock AuthProvider
vi.mock("../auth/AuthProvider", () => ({
  useAuth: () => ({
    isAuthenticated: true,
    getAccessToken: vi.fn().mockResolvedValue("mock-token"),
  }),
}));

// Mock HTTP client to avoid network calls
vi.mock("../../services/gameHttpClient", () => ({
  gameHttpClient: {
    getGameState: vi.fn().mockRejectedValue(new Error("Not found")),
  },
}));

// Mock react-router
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual("react-router-dom");
  return {
    ...actual,
    useParams: vi.fn(() => ({ gameId: "test-game-123" })),
    useNavigate: vi.fn(),
  };
});

const mockGetWebSocketService = vi.mocked(getWebSocketService);

// Mock console methods to reduce test noise
const mockConsole = {
  log: vi.spyOn(console, "log").mockImplementation(() => {}),
  error: vi.spyOn(console, "error").mockImplementation(() => {}),
  warn: vi.spyOn(console, "warn").mockImplementation(() => {}),
};

describe("Game State Synchronization", () => {
  let messageHandlers: Set<(message: ServerMessage) => void>;
  let mockWebSocketService: any;

  const mockPlayer: Player = {
    user_id: "player-1",
    display_name: "Player One",
    points: 10,
    guess_history: [],
    is_connected: true,
  };

  const mockGameState: GameState = {
    id: "game-123",
    word: "HELLO",
    word_length: 5,
    current_round: 1,
    status: "Active",
    current_phase: "Guessing",
    players: [mockPlayer],
    official_board: [],
    current_winner: null,
    created_at: "2024-01-01T00:00:00Z",
    point_threshold: 25,
  };

  const mockGuessResult: GuessResult = {
    word: "WORLD",
    player_id: "player-1",
    letters: [
      { letter: "W", status: "Absent", position: 0 },
      { letter: "O", status: "Present", position: 1 },
      { letter: "R", status: "Absent", position: 2 },
      { letter: "L", status: "Correct", position: 3 },
      { letter: "D", status: "Absent", position: 4 },
    ] as LetterResult[],
    points_earned: 3,
    timestamp: "2024-01-01T00:01:00Z",
  };

  const mockPersonalGuess: PersonalGuess = {
    word: "WORLD",
    points_earned: 3,
    was_winning_guess: false,
    timestamp: "2024-01-01T00:01:00Z",
  };

  beforeEach(() => {
    messageHandlers = new Set();
    mockWebSocketService = {
      addMessageHandler: vi.fn((handler) => messageHandlers.add(handler)),
      removeMessageHandler: vi.fn((handler) => messageHandlers.delete(handler)),
      sendMessage: vi.fn(),
      connect: vi.fn().mockResolvedValue(undefined),
      disconnect: vi.fn(),
      authenticate: vi.fn().mockResolvedValue(true),
      rejoinGame: vi.fn(),
      isConnected: true,
      authenticated: true,
    };

    mockGetWebSocketService.mockReturnValue(mockWebSocketService);

    // Reset game store
    useGameStore.getState().resetGame();

    vi.clearAllMocks();

    // Render the Game component to register message handlers
    act(() => {
      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>
      );
    });
  });

  afterEach(() => {
    Object.values(mockConsole).forEach((mock) => mock.mockClear());
  });

  // Helper function to simulate server message
  const simulateServerMessage = (message: ServerMessage) => {
    act(() => {
      messageHandlers.forEach((handler) => {
        try {
          handler(message);
        } catch (error) {
          console.error("Handler error:", error);
        }
      });
    });
  };

  describe("GameStateUpdate message handling", () => {
    it("should update game state when GameStateUpdate is received", () => {
      // Simulate GameStateUpdate message
      simulateServerMessage({
        GameStateUpdate: { state: mockGameState },
      });

      const updatedState = useGameStore.getState();
      expect(updatedState.gameState).toEqual(mockGameState);
    });

    it("should override local state completely with server state", () => {
      const store = useGameStore.getState();

      // Set initial local state
      const localGameState: GameState = {
        ...mockGameState,
        current_round: 2,
        current_phase: "Countdown",
        players: [{ ...mockPlayer, points: 20 }],
      };

      store.setGameState(localGameState);
      expect(useGameStore.getState().gameState?.current_round).toBe(2);

      // Server sends update that should override local state
      simulateServerMessage({
        GameStateUpdate: { state: mockGameState },
      });

      const updatedState = useGameStore.getState();
      expect(updatedState.gameState?.current_round).toBe(1); // Server value wins
      expect(updatedState.gameState?.current_phase).toBe("Guessing"); // Server value wins
      expect(updatedState.gameState?.players[0].points).toBe(10); // Server value wins
    });

    it("should handle game state updates with different player configurations", () => {
      const multiPlayerState: GameState = {
        ...mockGameState,
        players: [
          mockPlayer,
          {
            ...mockPlayer,
            user_id: "player-2",
            display_name: "Player Two",
            points: 15,
          },
          {
            ...mockPlayer,
            user_id: "player-3",
            display_name: "Player Three",
            points: 5,
          },
        ],
      };

      simulateServerMessage({
        GameStateUpdate: { state: multiPlayerState },
      });

      const state = useGameStore.getState();
      expect(state.gameState?.players).toHaveLength(3);
      expect(state.gameState?.players.map((p) => p.user_id)).toEqual([
        "player-1",
        "player-2",
        "player-3",
      ]);
    });

    it("should handle connection status changes in player list", () => {
      const stateWithDisconnectedPlayer: GameState = {
        ...mockGameState,
        players: [{ ...mockPlayer, is_connected: false }],
      };

      simulateServerMessage({
        GameStateUpdate: { state: stateWithDisconnectedPlayer },
      });

      const state = useGameStore.getState();
      expect(state.gameState?.players[0].is_connected).toBe(false);
    });
  });

  describe("Game phase transition handling", () => {
    beforeEach(() => {
      const store = useGameStore.getState();
      store.setGameState(mockGameState);
    });

    it("should clear pending guess when transitioning away from Guessing phase", () => {
      const store = useGameStore.getState();

      // Set pending guess during guessing phase
      store.setPendingGuess("WORLD");
      expect(useGameStore.getState().pendingGuess).toBe("WORLD");

      // Re-render Game component to pick up new pending guess state
      act(() => {
        render(
          <MemoryRouter initialEntries={["/game/test-game-123"]}>
            <Game />
          </MemoryRouter>
        );
      });

      // Transition to different phase
      const newState: GameState = {
        ...mockGameState,
        current_phase: "IndividualGuess",
      };

      simulateServerMessage({
        GameStateUpdate: { state: newState },
      });

      expect(useGameStore.getState().pendingGuess).toBeNull();
      expect(useGameStore.getState().currentGuess).toBe("");
    });

    it("should clear pending guess when moving to new round in Guessing phase", () => {
      const store = useGameStore.getState();

      // Set pending guess in round 1
      store.setPendingGuess("WORLD");

      // Re-render Game component to pick up new pending guess state
      act(() => {
        render(
          <MemoryRouter initialEntries={["/game/test-game-123"]}>
            <Game />
          </MemoryRouter>
        );
      });

      // Move to round 2 (still in guessing phase)
      const newState: GameState = {
        ...mockGameState,
        current_round: 2,
        current_phase: "Guessing",
      };

      simulateServerMessage({
        GameStateUpdate: { state: newState },
      });

      expect(useGameStore.getState().pendingGuess).toBeNull();
      expect(useGameStore.getState().currentGuess).toBe("");
    });

    it("should preserve pending guess when staying in same round and phase", () => {
      const store = useGameStore.getState();

      // Set pending guess
      store.setPendingGuess("WORLD");

      // Update state but keep same round and phase
      const newState: GameState = {
        ...mockGameState,
        players: [{ ...mockPlayer, points: 15 }], // Only player points changed
      };

      simulateServerMessage({
        GameStateUpdate: { state: newState },
      });

      // Pending guess should be preserved
      expect(useGameStore.getState().pendingGuess).toBe("WORLD");
    });

    it("should handle phase transitions correctly", () => {
      const phases: Array<GameState["current_phase"]> = [
        "Waiting",
        "Countdown",
        "Guessing",
        "IndividualGuess",
        "GameOver",
      ];

      phases.forEach((phase, index) => {
        const newState: GameState = {
          ...mockGameState,
          current_phase: phase,
          current_round: index + 1,
        };

        simulateServerMessage({
          GameStateUpdate: { state: newState },
        });

        const state = useGameStore.getState();
        expect(state.gameState?.current_phase).toBe(phase);
        expect(state.gameState?.current_round).toBe(index + 1);
      });
    });
  });

  describe("CountdownStart message handling", () => {
    it("should set countdown end time correctly", () => {
      const startTime = Date.now();
      const countdownSeconds = 30;

      vi.setSystemTime(startTime);

      simulateServerMessage({
        CountdownStart: { seconds: countdownSeconds },
      });

      const state = useGameStore.getState();
      const expectedEndTime = startTime + countdownSeconds * 1000;

      expect(state.countdownEndTime).toBe(expectedEndTime);
      expect(mockConsole.log).toHaveBeenCalledWith(
        "Countdown started:",
        countdownSeconds,
        "seconds",
      );
    });

    it("should update countdown end time when receiving multiple CountdownStart messages", () => {
      const firstTime = Date.now();
      vi.setSystemTime(firstTime);

      simulateServerMessage({
        CountdownStart: { seconds: 30 },
      });

      const firstEndTime = useGameStore.getState().countdownEndTime;

      // Advance time and send another countdown
      const secondTime = firstTime + 5000;
      vi.setSystemTime(secondTime);

      simulateServerMessage({
        CountdownStart: { seconds: 25 },
      });

      const secondEndTime = useGameStore.getState().countdownEndTime;

      expect(secondEndTime).toBeGreaterThan(firstEndTime!);
      expect(secondEndTime).toBe(secondTime + 25000);
    });
  });

  describe("RoundResult message handling", () => {
    it("should add personal guess to history when provided", () => {
      simulateServerMessage({
        RoundResult: {
          winning_guess: mockGuessResult,
          your_guess: mockPersonalGuess,
          next_phase: "IndividualGuess",
          is_word_completed: false,
        },
      });

      const state = useGameStore.getState();
      expect(state.personalGuessHistory).toHaveLength(1);
      expect(state.personalGuessHistory[0]).toEqual(mockPersonalGuess);
    });

    it("should not add to personal history when your_guess is null", () => {
      simulateServerMessage({
        RoundResult: {
          winning_guess: mockGuessResult,
          your_guess: null,
          next_phase: "IndividualGuess",
          is_word_completed: false,
        },
      });

      const state = useGameStore.getState();
      expect(state.personalGuessHistory).toHaveLength(0);
    });

    it("should handle multiple round results accumulating personal history", () => {
      const guess1: PersonalGuess = {
        ...mockPersonalGuess,
        word: "FIRST",
        points_earned: 1,
      };

      const guess2: PersonalGuess = {
        ...mockPersonalGuess,
        word: "SECOND",
        points_earned: 2,
      };

      simulateServerMessage({
        RoundResult: {
          winning_guess: mockGuessResult,
          your_guess: guess1,
          next_phase: "IndividualGuess",
          is_word_completed: false,
        },
      });

      simulateServerMessage({
        RoundResult: {
          winning_guess: { ...mockGuessResult, word: "OTHER" },
          your_guess: guess2,
          next_phase: "Countdown",
          is_word_completed: false,
        },
      });

      const state = useGameStore.getState();
      expect(state.personalGuessHistory).toHaveLength(2);
      expect(state.personalGuessHistory[0]).toEqual(guess1);
      expect(state.personalGuessHistory[1]).toEqual(guess2);
    });

    it("should log round results", () => {
      const roundResult = {
        winning_guess: mockGuessResult,
        your_guess: mockPersonalGuess,
        next_phase: "IndividualGuess" as const,
        is_word_completed: false,
      };

      simulateServerMessage({
        RoundResult: roundResult,
      });

      expect(mockConsole.log).toHaveBeenCalledWith(
        "Round result:",
        roundResult,
      );
    });
  });

  describe("GameOver message handling", () => {
    it("should log game over message", () => {
      const gameOverMessage = {
        winner: mockPlayer,
        final_scores: [mockPlayer],
      };

      simulateServerMessage({
        GameOver: gameOverMessage,
      });

      expect(mockConsole.log).toHaveBeenCalledWith(
        "Game over:",
        gameOverMessage,
      );
    });
  });

  describe("Player connection status handling", () => {
    it("should log player disconnection", () => {
      const playerId = "player-123";

      simulateServerMessage({
        PlayerDisconnected: { player_id: playerId },
      });

      expect(mockConsole.log).toHaveBeenCalledWith(
        "Player disconnected:",
        playerId,
      );
    });

    it("should log player reconnection", () => {
      const playerId = "player-123";

      simulateServerMessage({
        PlayerReconnected: { player_id: playerId },
      });

      expect(mockConsole.log).toHaveBeenCalledWith(
        "Player reconnected:",
        playerId,
      );
    });
  });

  describe("Error message handling", () => {
    it("should handle generic errors by setting last error", () => {
      const errorMessage = "Something went wrong";

      simulateServerMessage({
        Error: { message: errorMessage },
      });

      const state = useGameStore.getState();
      expect(state.lastError).toBe(errorMessage);
      expect(mockConsole.error).toHaveBeenCalledWith(
        "Game error:",
        errorMessage,
      );
    });

    it("should handle invalid guess errors specially", () => {
      // Set pending guess in store and re-render to ensure the component sees it
      act(() => {
        useGameStore.getState().setPendingGuess("INVALID");
      });

      // Re-render the component to pick up the pending guess change
      act(() => {
        render(
          <MemoryRouter initialEntries={["/game/test-game-123"]}>
            <Game />
          </MemoryRouter>
        );
      });

      simulateServerMessage({
        Error: { message: "Invalid guess - word not found" },
      });

      const state = useGameStore.getState();
      expect(state.lastError).toBe("Invalid word - not in our word list");
      expect(state.currentGuess).toBe("INVALID"); // Should restore pending guess
      expect(state.pendingGuess).toBeNull(); // Should clear pending guess
    });

    it('should ignore "No disconnected players to rejoin" errors', () => {
      simulateServerMessage({
        Error: {
          message: "No disconnected players to rejoin for game game-123",
        },
      });

      const state = useGameStore.getState();
      expect(state.lastError).toBeNull(); // Should not set error
      expect(mockConsole.log).toHaveBeenCalledWith(
        "Already in game, no need to rejoin",
      );
      expect(mockConsole.error).not.toHaveBeenCalled();
    });

    it("should clear pending guess and restore current guess on invalid word error", () => {
      // Set pending guess and current guess in store, then re-render
      act(() => {
        useGameStore.getState().setPendingGuess("BADWORD");
        useGameStore.getState().setCurrentGuess("");
      });

      // Re-render the component to pick up the state changes
      act(() => {
        render(
          <MemoryRouter initialEntries={["/game/test-game-123"]}>
            <Game />
          </MemoryRouter>
        );
      });

      simulateServerMessage({
        Error: { message: "Invalid guess - not in word list" },
      });

      const state = useGameStore.getState();
      expect(state.currentGuess).toBe("BADWORD");
      expect(state.pendingGuess).toBeNull();
      expect(state.lastError).toBe("Invalid word - not in our word list");
    });
  });

  describe("Message handler lifecycle", () => {
    it("should properly register and unregister message handlers", () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      // Add handlers
      mockWebSocketService.addMessageHandler(handler1);
      mockWebSocketService.addMessageHandler(handler2);

      expect(messageHandlers.size).toBe(2);
      expect(messageHandlers.has(handler1)).toBe(true);
      expect(messageHandlers.has(handler2)).toBe(true);

      // Remove one handler
      mockWebSocketService.removeMessageHandler(handler1);

      expect(messageHandlers.size).toBe(1);
      expect(messageHandlers.has(handler1)).toBe(false);
      expect(messageHandlers.has(handler2)).toBe(true);

      // Test that only remaining handler is called
      simulateServerMessage({
        QueueJoined: { position: 1 },
      });

      expect(handler1).not.toHaveBeenCalled();
      expect(handler2).toHaveBeenCalled();
    });

    it("should handle handler errors without affecting other handlers", () => {
      const errorHandler = vi.fn().mockImplementation(() => {
        throw new Error("Handler error");
      });
      const goodHandler = vi.fn();

      mockWebSocketService.addMessageHandler(errorHandler);
      mockWebSocketService.addMessageHandler(goodHandler);

      simulateServerMessage({
        QueueJoined: { position: 1 },
      });

      expect(errorHandler).toHaveBeenCalled();
      expect(goodHandler).toHaveBeenCalled();
      expect(mockConsole.error).toHaveBeenCalledWith(
        "Handler error:",
        expect.any(Error),
      );
    });
  });

  describe("State consistency validation", () => {
    it("should maintain state consistency across rapid message updates", () => {
      const messages: ServerMessage[] = [
        { GameStateUpdate: { state: { ...mockGameState, current_round: 1 } } },
        { CountdownStart: { seconds: 30 } },
        { GameStateUpdate: { state: { ...mockGameState, current_round: 2 } } },
        {
          RoundResult: {
            winning_guess: mockGuessResult,
            your_guess: mockPersonalGuess,
            next_phase: "IndividualGuess",
            is_word_completed: false,
          },
        },
        {
          GameStateUpdate: {
            state: {
              ...mockGameState,
              current_round: 3,
              current_phase: "GameOver",
            },
          },
        },
      ];

      messages.forEach((message) => simulateServerMessage(message));

      const finalState = useGameStore.getState();
      expect(finalState.gameState?.current_round).toBe(3);
      expect(finalState.gameState?.current_phase).toBe("GameOver");
      expect(finalState.personalGuessHistory).toHaveLength(1);
      expect(finalState.countdownEndTime).toBeDefined();
    });

    it("should handle out-of-order messages gracefully", () => {
      // Simulate messages arriving in unexpected order
      simulateServerMessage({
        RoundResult: {
          winning_guess: mockGuessResult,
          your_guess: mockPersonalGuess,
          next_phase: "IndividualGuess",
          is_word_completed: false,
        },
      });

      simulateServerMessage({
        CountdownStart: { seconds: 30 },
      });

      simulateServerMessage({
        GameStateUpdate: { state: mockGameState },
      });

      const state = useGameStore.getState();
      expect(state.gameState).toEqual(mockGameState);
      expect(state.personalGuessHistory).toHaveLength(1);
      expect(state.countdownEndTime).toBeDefined();
    });
  });
});
