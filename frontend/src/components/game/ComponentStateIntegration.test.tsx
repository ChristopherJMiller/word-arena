import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Game } from "./Game";
import { useGameStore } from "../../store/gameStore";
import { useWebSocket } from "../../hooks/useWebSocket";
import type { ServerMessage, GameState } from "../../types/generated";

// Mock dependencies
vi.mock("../../hooks/useWebSocket", () => ({
  useWebSocket: vi.fn(),
}));

vi.mock("../../store/gameStore", () => ({
  useGameStore: vi.fn(),
}));

vi.mock("./GameLayout", () => ({
  GameLayout: () => <div data-testid="game-layout">Game Layout Component</div>,
}));

vi.mock("./GameNotFound", () => ({
  GameNotFound: () => <div data-testid="game-not-found">Game Not Found</div>,
}));

vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual("react-router-dom");
  const mockNavigate = vi.fn();
  const mockUseParams = vi.fn(() => ({ gameId: "test-game-123" }));
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useParams: mockUseParams,
  };
});

const mockUseWebSocket = vi.mocked(useWebSocket);
const mockUseGameStore = vi.mocked(useGameStore);

// Create mock functions that can be accessed in tests
const mockNavigate = vi.fn();
const mockUseParams = vi.fn(() => ({ gameId: "test-game-123" }));

describe("Game Component State Integration", () => {
  let messageHandlers: Set<(message: ServerMessage) => void>;
  let mockGameStoreActions: any;
  let mockWebSocketActions: any;

  const mockGameState: GameState = {
    id: "test-game-123",
    word: "HELLO",
    word_length: 5,
    current_round: 1,
    status: "Active",
    current_phase: "Guessing",
    players: [
      {
        user_id: "player-1",
        display_name: "Player One",
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

  beforeEach(() => {
    messageHandlers = new Set();

    mockGameStoreActions = {
      setGameId: vi.fn(),
      setGameState: vi.fn(),
      setCountdownEndTime: vi.fn(),
      addPersonalGuess: vi.fn(),
      setLastError: vi.fn(),
      setCurrentGuess: vi.fn(),
      setPendingGuess: vi.fn(),
      reconnectToGame: vi.fn(),
      rejoinAfterDisconnect: vi.fn(),
    };

    mockWebSocketActions = {
      addMessageHandler: vi.fn((handler) => messageHandlers.add(handler)),
      removeMessageHandler: vi.fn((handler) => messageHandlers.delete(handler)),
    };

    mockUseGameStore.mockReturnValue({
      gameState: null,
      gameId: null,
      pendingGuess: null,
      reconnectToGame:
        mockGameStoreActions.reconnectToGame.mockResolvedValue(undefined),
      rejoinAfterDisconnect:
        mockGameStoreActions.rejoinAfterDisconnect.mockResolvedValue(undefined),
      ...mockGameStoreActions,
    });

    mockUseWebSocket.mockReturnValue({
      isAuthenticated: true,
      ...mockWebSocketActions,
    });

    vi.clearAllMocks();
  });

  afterEach(() => {
    messageHandlers.clear();
  });

  // Helper to simulate server message
  const simulateServerMessage = (message: ServerMessage) => {
    messageHandlers.forEach((handler) => handler(message));
  };

  describe("URL-based game loading", () => {
    it("should set gameId from URL parameter", () => {
      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockGameStoreActions.setGameId).toHaveBeenCalledWith(
        "test-game-123",
      );
    });

    it("should redirect to lobby when gameId is missing", () => {
      // Configure the mock to return undefined gameId
      mockUseParams.mockReturnValueOnce({} as any);

      render(
        <MemoryRouter initialEntries={["/game/"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockNavigate).toHaveBeenCalledWith("/");
    });

    it("should show loading state when game state is null", () => {
      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(screen.getByText("Reconnecting...")).toBeInTheDocument();
      expect(
        screen.getByText("Loading game state for test-game-123"),
      ).toBeInTheDocument();
    });

    it("should render GameLayout when game state is loaded", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: null,
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(screen.getByTestId("game-layout")).toBeInTheDocument();
    });
  });

  describe("Game state loading and reconnection", () => {
    it("should attempt reconnection when game state does not match gameId", () => {
      // Game state is null, should trigger reconnection
      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockGameStoreActions.reconnectToGame).toHaveBeenCalledWith(
        "test-game-123",
      );
    });

    it("should attempt reconnection when game state ID does not match URL", () => {
      const differentGameState = { ...mockGameState, id: "different-game" };
      mockUseGameStore.mockReturnValue({
        gameState: differentGameState,
        gameId: "test-game-123",
        pendingGuess: null,
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockGameStoreActions.reconnectToGame).toHaveBeenCalledWith(
        "test-game-123",
      );
    });

    it("should fall back to WebSocket rejoin when HTTP reconnect fails", async () => {
      mockGameStoreActions.reconnectToGame.mockRejectedValue(
        new Error("HTTP failed"),
      );
      mockGameStoreActions.rejoinAfterDisconnect.mockResolvedValue(undefined);

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      await waitFor(() => {
        expect(mockGameStoreActions.rejoinAfterDisconnect).toHaveBeenCalledWith(
          "test-game-123",
        );
      });
    });

    it("should redirect to lobby when both reconnection methods fail", async () => {
      mockGameStoreActions.reconnectToGame.mockRejectedValue(
        new Error("HTTP failed"),
      );
      mockGameStoreActions.rejoinAfterDisconnect.mockRejectedValue(
        new Error("WebSocket failed"),
      );

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith("/");
      });
    });
  });

  describe("Message handler registration and cleanup", () => {
    it("should register message handler when WebSocket is authenticated", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: null,
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockWebSocketActions.addMessageHandler).toHaveBeenCalled();
      expect(messageHandlers.size).toBe(1);
    });

    it("should not register message handler when WebSocket is not authenticated", () => {
      mockUseWebSocket.mockReturnValue({
        isAuthenticated: false,
        ...mockWebSocketActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockWebSocketActions.addMessageHandler).not.toHaveBeenCalled();
      expect(messageHandlers.size).toBe(0);
    });

    it("should clean up message handler on unmount", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: null,
        ...mockGameStoreActions,
      });

      const { unmount } = render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(messageHandlers.size).toBe(1);

      unmount();

      expect(mockWebSocketActions.removeMessageHandler).toHaveBeenCalled();
    });
  });

  describe("Global message handling integration", () => {
    beforeEach(() => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: null,
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );
    });

    it("should handle GameStateUpdate messages", () => {
      const newGameState = { ...mockGameState, current_round: 2 };

      simulateServerMessage({
        GameStateUpdate: { state: newGameState },
      });

      expect(mockGameStoreActions.setGameState).toHaveBeenCalledWith(
        newGameState,
      );
    });

    it("should handle CountdownStart messages", () => {
      const countdownSeconds = 30;
      const expectedEndTime = Date.now() + countdownSeconds * 1000;

      vi.setSystemTime(Date.now());

      simulateServerMessage({
        CountdownStart: { seconds: countdownSeconds },
      });

      expect(mockGameStoreActions.setCountdownEndTime).toHaveBeenCalledWith(
        expect.closeTo(expectedEndTime, 100),
      );
    });

    it("should handle RoundResult messages with personal guess", () => {
      const personalGuess = {
        word: "WORLD",
        points_earned: 3,
        was_winning_guess: false,
        timestamp: "2024-01-01T00:01:00Z",
      };

      simulateServerMessage({
        RoundResult: {
          winning_guess: {
            word: "WORLD",
            player_id: "player-1",
            letters: [],
            points_earned: 3,
            timestamp: "2024-01-01T00:01:00Z",
          },
          your_guess: personalGuess,
          next_phase: "IndividualGuess",
          is_word_completed: false,
        },
      });

      expect(mockGameStoreActions.addPersonalGuess).toHaveBeenCalledWith(
        personalGuess,
      );
    });

    it("should handle GameLeft messages by navigating to lobby", () => {
      simulateServerMessage("GameLeft");

      expect(mockNavigate).toHaveBeenCalledWith("/");
    });

    it("should handle Error messages by setting last error", () => {
      const errorMessage = "Something went wrong";

      simulateServerMessage({
        Error: { message: errorMessage },
      });

      expect(mockGameStoreActions.setLastError).toHaveBeenCalledWith(
        errorMessage,
      );
    });

    it("should handle invalid guess errors specially", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: "INVALID",
        ...mockGameStoreActions,
      });

      // Re-render with pending guess
      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      simulateServerMessage({
        Error: { message: "Invalid guess - not in word list" },
      });

      expect(mockGameStoreActions.setLastError).toHaveBeenCalledWith(
        "Invalid word - not in our word list",
      );
      expect(mockGameStoreActions.setCurrentGuess).toHaveBeenCalledWith(
        "INVALID",
      );
      expect(mockGameStoreActions.setPendingGuess).toHaveBeenCalledWith(null);
    });

    it('should ignore "No disconnected players to rejoin" errors', () => {
      simulateServerMessage({
        Error: { message: "No disconnected players to rejoin" },
      });

      expect(mockGameStoreActions.setLastError).not.toHaveBeenCalled();
    });
  });

  describe("Pending guess management during state transitions", () => {
    it("should clear pending guess when moving away from Guessing phase", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: "WORLD",
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      const newGameState = {
        ...mockGameState,
        current_phase: "IndividualGuess" as const,
      };

      simulateServerMessage({
        GameStateUpdate: { state: newGameState },
      });

      expect(mockGameStoreActions.setPendingGuess).toHaveBeenCalledWith(null);
      expect(mockGameStoreActions.setCurrentGuess).toHaveBeenCalledWith("");
    });

    it("should clear pending guess when moving to new round", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: "WORLD",
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      const newGameState = {
        ...mockGameState,
        current_round: 2,
        current_phase: "Guessing" as const,
      };

      simulateServerMessage({
        GameStateUpdate: { state: newGameState },
      });

      expect(mockGameStoreActions.setPendingGuess).toHaveBeenCalledWith(null);
      expect(mockGameStoreActions.setCurrentGuess).toHaveBeenCalledWith("");
    });

    it("should preserve pending guess in same round and phase", () => {
      mockUseGameStore.mockReturnValue({
        gameState: mockGameState,
        gameId: "test-game-123",
        pendingGuess: "WORLD",
        ...mockGameStoreActions,
      });

      render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      // Same round and phase, just different player points
      const newGameState = {
        ...mockGameState,
        players: [{ ...mockGameState.players[0], points: 15 }],
      };

      simulateServerMessage({
        GameStateUpdate: { state: newGameState },
      });

      expect(mockGameStoreActions.setPendingGuess).not.toHaveBeenCalledWith(
        null,
      );
      expect(mockGameStoreActions.setCurrentGuess).not.toHaveBeenCalledWith("");
    });
  });

  describe("Component re-rendering optimization", () => {
    it("should only depend on gameId for reconnection useEffect", () => {
      const { rerender } = render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      vi.clearAllMocks();

      // Change other store values but not gameId - should not trigger reconnection
      mockUseGameStore.mockReturnValue({
        gameState: { ...mockGameState, current_round: 2 },
        gameId: "test-game-123", // Same gameId
        pendingGuess: "CHANGED",
        ...mockGameStoreActions,
      });

      rerender(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      // Should not trigger reconnection since gameId didn't change
      expect(mockGameStoreActions.reconnectToGame).not.toHaveBeenCalled();
    });

    it("should trigger reconnection when gameId changes", () => {
      const { rerender } = render(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Game />
        </MemoryRouter>,
      );

      vi.clearAllMocks();

      // Change gameId - should trigger reconnection
      const mockUseParams = vi.fn(() => ({ gameId: "new-game-456" }));
      vi.doMock("react-router-dom", async () => {
        const actual = await vi.importActual("react-router-dom");
        return {
          ...actual,
          useParams: mockUseParams,
          useNavigate: () => mockNavigate,
        };
      });

      rerender(
        <MemoryRouter initialEntries={["/game/new-game-456"]}>
          <Game />
        </MemoryRouter>,
      );

      expect(mockGameStoreActions.setGameId).toHaveBeenCalledWith(
        "new-game-456",
      );
    });
  });
});
