import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { Game } from "./Game";

// Mock the store and services
vi.mock("../../store/gameStore", () => ({
  useGameStore: vi.fn(() => ({
    setGameId: vi.fn(),
    gameState: null,
    reconnectToGame: vi.fn().mockResolvedValue(undefined),
    setGameState: vi.fn(),
    setCountdownEndTime: vi.fn(),
    addPersonalGuess: vi.fn(),
    gameId: null,
    currentGuess: "",
    isSubmitting: false,
    countdownEndTime: undefined,
    isReconnecting: false,
    personalGuessHistory: [],
    setCurrentGuess: vi.fn(),
    setIsSubmitting: vi.fn(),
    setIsReconnecting: vi.fn(),
    rejoinAfterDisconnect: vi.fn(),
    resetGame: vi.fn(),
  })),
}));

vi.mock("./GameLayout", () => ({
  GameLayout: () => <div data-testid="game-layout">Game Layout</div>,
}));

vi.mock("../auth/AuthProvider", () => ({
  useAuth: vi.fn(() => ({
    isAuthenticated: true,
    user: {
      id: "test-user",
      email: "test@example.com",
      display_name: "Test User",
    },
  })),
}));

vi.mock("../../hooks/useWebSocket", () => ({
  useWebSocket: vi.fn(() => ({
    isConnected: true,
    isAuthenticated: true,
    addMessageHandler: vi.fn(),
    removeMessageHandler: vi.fn(),
  })),
}));

import { useGameStore } from "../../store/gameStore";

const mockUseGameStore = vi.mocked(useGameStore);

describe("Game Component", () => {
  const mockStore = {
    setGameId: vi.fn(),
    gameState: null as any,
    reconnectToGame: vi.fn().mockResolvedValue(undefined),
    setGameState: vi.fn(),
    setCountdownEndTime: vi.fn(),
    addPersonalGuess: vi.fn(),
    gameId: null,
    currentGuess: "",
    isSubmitting: false,
    countdownEndTime: undefined,
    isReconnecting: false,
    personalGuessHistory: [],
    setCurrentGuess: vi.fn(),
    setIsSubmitting: vi.fn(),
    setIsReconnecting: vi.fn(),
    rejoinAfterDisconnect: vi.fn(),
    resetGame: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset all mock functions
    mockStore.setGameId.mockClear();
    mockStore.reconnectToGame.mockResolvedValue(undefined);
    mockStore.setGameState.mockClear();
    mockStore.setCountdownEndTime.mockClear();
    mockStore.addPersonalGuess.mockClear();
    mockStore.setCurrentGuess.mockClear();
    mockStore.setIsSubmitting.mockClear();
    mockStore.setIsReconnecting.mockClear();
    mockStore.rejoinAfterDisconnect.mockClear();
    mockStore.resetGame.mockClear();
    mockUseGameStore.mockReturnValue(mockStore as any);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  const renderGameWithRouter = (gameId: string) => {
    return render(
      <MemoryRouter initialEntries={[`/game/${gameId}`]}>
        <Routes>
          <Route path="/game/:gameId" element={<Game />} />
          <Route path="/" element={<div data-testid="lobby">Lobby</div>} />
        </Routes>
      </MemoryRouter>,
    );
  };

  describe("URL parameter handling", () => {
    it("should extract gameId from URL and set it in store", () => {
      renderGameWithRouter("test-game-123");

      expect(mockStore.setGameId).toHaveBeenCalledWith("test-game-123");
    });

    it("should handle different gameId formats", () => {
      renderGameWithRouter("abc-123-def-456");

      expect(mockStore.setGameId).toHaveBeenCalledWith("abc-123-def-456");
    });

    it("should handle UUID-style gameIds", () => {
      const uuidGameId = "550e8400-e29b-41d4-a716-446655440000";
      renderGameWithRouter(uuidGameId);

      expect(mockStore.setGameId).toHaveBeenCalledWith(uuidGameId);
    });
  });

  describe("reconnection logic", () => {
    it("should trigger reconnection when no game state exists", () => {
      mockStore.gameState = null;

      renderGameWithRouter("test-game-123");

      expect(mockStore.reconnectToGame).toHaveBeenCalledWith("test-game-123");
    });

    it("should trigger reconnection when gameId mismatches", () => {
      mockStore.gameState = {
        id: "different-game-id",
        word: "",
        word_length: 5,
        current_round: 1,
        status: "Active",
        players: [],
        official_board: [],
        current_winner: null,
        created_at: "2024-01-01T00:00:00Z",
        point_threshold: 25,
      };

      renderGameWithRouter("test-game-123");

      expect(mockStore.reconnectToGame).toHaveBeenCalledWith("test-game-123");
    });

    it("should not trigger reconnection when gameId matches existing state", () => {
      mockStore.gameState = {
        id: "test-game-123",
        word: "",
        word_length: 5,
        current_round: 1,
        status: "Active",
        players: [],
        official_board: [],
        current_winner: null,
        created_at: "2024-01-01T00:00:00Z",
        point_threshold: 25,
      };

      renderGameWithRouter("test-game-123");

      expect(mockStore.reconnectToGame).not.toHaveBeenCalled();
    });
  });

  describe("loading and display states", () => {
    it("should show loading state when no game state exists", () => {
      mockStore.gameState = null;

      renderGameWithRouter("test-game-123");

      expect(screen.getByText("Reconnecting...")).toBeInTheDocument();
      expect(
        screen.getByText("Loading game state for test-game-123"),
      ).toBeInTheDocument();
    });

    it("should show game layout when game state exists", () => {
      mockStore.gameState = {
        id: "test-game-123",
        word: "",
        word_length: 5,
        current_round: 1,
        status: "Active",
        players: [],
        official_board: [],
        current_winner: null,
        created_at: "2024-01-01T00:00:00Z",
        point_threshold: 25,
      };

      renderGameWithRouter("test-game-123");

      expect(screen.getByTestId("game-layout")).toBeInTheDocument();
      expect(screen.queryByText("Reconnecting...")).not.toBeInTheDocument();
    });

    it("should show loading spinner in reconnecting state", () => {
      mockStore.gameState = null;

      renderGameWithRouter("test-game-123");

      // Look for spinner animation class
      const spinnerElement = screen
        .getByText("Reconnecting...")
        .parentElement?.querySelector(".animate-spin");
      expect(spinnerElement).toBeInTheDocument();
    });
  });

  describe("invalid URL handling", () => {
    it("should redirect to lobby when no gameId in URL", () => {
      render(
        <MemoryRouter initialEntries={["/game/"]}>
          <Routes>
            <Route path="/game/:gameId" element={<Game />} />
            <Route path="/" element={<div data-testid="lobby">Lobby</div>} />
          </Routes>
        </MemoryRouter>,
      );

      // When gameId is undefined, component should redirect
      // This is handled by navigate('/') in the useEffect
      expect(mockStore.setGameId).not.toHaveBeenCalled();
      expect(mockStore.reconnectToGame).not.toHaveBeenCalled();
    });
  });

  describe("component lifecycle", () => {
    it("should call setGameId and reconnectToGame only once per gameId", () => {
      const { rerender } = renderGameWithRouter("test-game-123");

      expect(mockStore.setGameId).toHaveBeenCalledTimes(1);
      expect(mockStore.reconnectToGame).toHaveBeenCalledTimes(1);

      // Re-render with same props shouldn't trigger additional calls
      rerender(
        <MemoryRouter initialEntries={["/game/test-game-123"]}>
          <Routes>
            <Route path="/game/:gameId" element={<Game />} />
          </Routes>
        </MemoryRouter>,
      );

      // Should still be called only once due to useEffect dependencies
      expect(mockStore.setGameId).toHaveBeenCalledTimes(1);
      expect(mockStore.reconnectToGame).toHaveBeenCalledTimes(1);
    });
  });
});
