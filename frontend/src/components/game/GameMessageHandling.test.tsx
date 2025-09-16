import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import { Game } from "./Game";
import { useGameStore } from "../../store/gameStore";
import { useWebSocket } from "../../hooks/useWebSocket";
import type { GameState, ServerMessage } from "../../types/generated";

// Mock the hooks
vi.mock("../../store/gameStore");
vi.mock("../../hooks/useWebSocket");
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual("react-router-dom");
  return {
    ...actual,
    useParams: () => ({ gameId: "test-game-id" }),
    useNavigate: () => vi.fn(),
  };
});

const mockUseGameStore = vi.mocked(useGameStore);
const mockUseWebSocket = vi.mocked(useWebSocket);

describe("Game Message Handling", () => {
  const mockGameState: GameState = {
    id: "test-game-id",
    word: "*****",
    word_length: 5,
    current_round: 1,
    status: "Active",
    current_phase: "Guessing",
    players: [],
    official_board: [],
    current_winner: null,
    created_at: "2024-01-01T00:00:00Z",
    point_threshold: 25,
  };

  const defaultStoreState = {
    gameState: mockGameState,
    gameId: "test-game-id",
    currentGuess: "",
    pendingGuess: null,
    countdownEndTime: null,
    lastError: null,
    personalGuessHistory: [],
    isReconnecting: false,
    setGameState: vi.fn(),
    setCurrentGuess: vi.fn(),
    setPendingGuess: vi.fn(),
    setLastError: vi.fn(),
    addPersonalGuess: vi.fn(),
    setCountdownEndTime: vi.fn(),
    setGameId: vi.fn(),
    reconnectToGame: vi.fn().mockResolvedValue(undefined),
    rejoinAfterDisconnect: vi.fn().mockResolvedValue(undefined),
    resetGame: vi.fn(),
  };

  let messageHandler: (message: ServerMessage) => void;
  const addMessageHandler = vi.fn((handler) => {
    messageHandler = handler;
  });
  const removeMessageHandler = vi.fn();

  const defaultWebSocketState = {
    isAuthenticated: true,
    addMessageHandler,
    removeMessageHandler,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseGameStore.mockReturnValue(defaultStoreState as any);
    mockUseWebSocket.mockReturnValue(defaultWebSocketState as any);
  });

  const renderComponent = () => {
    return render(
      <BrowserRouter>
        <Game />
      </BrowserRouter>,
    );
  };

  describe("GameStateUpdate message handling", () => {
    it("should clear pending guess when phase changes from Guessing to non-Guessing", () => {
      const setPendingGuess = vi.fn();
      const setCurrentGuess = vi.fn();
      const setGameState = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        gameState: { ...mockGameState, current_phase: "Guessing" },
        pendingGuess: "ABOUT",
        setPendingGuess,
        setCurrentGuess,
        setGameState,
      } as any);

      renderComponent();

      // Simulate phase change message
      const newState = {
        ...mockGameState,
        current_phase: "IndividualGuess" as const,
      };

      messageHandler({
        GameStateUpdate: { state: newState },
      });

      expect(setGameState).toHaveBeenCalledWith(newState);
      expect(setPendingGuess).toHaveBeenCalledWith(null);
      expect(setCurrentGuess).toHaveBeenCalledWith("");
    });

    it("should clear pending guess when round number changes", () => {
      const setPendingGuess = vi.fn();
      const setCurrentGuess = vi.fn();
      const setGameState = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        gameState: {
          ...mockGameState,
          current_round: 1,
          current_phase: "Guessing",
        },
        pendingGuess: "ABOUT",
        setPendingGuess,
        setCurrentGuess,
        setGameState,
      } as any);

      renderComponent();

      // Simulate new round message
      const newState = {
        ...mockGameState,
        current_round: 2,
        current_phase: "Guessing" as const,
      };

      messageHandler({
        GameStateUpdate: { state: newState },
      });

      expect(setGameState).toHaveBeenCalledWith(newState);
      expect(setPendingGuess).toHaveBeenCalledWith(null);
      expect(setCurrentGuess).toHaveBeenCalledWith("");
    });

    it("should not clear pending guess when staying in same phase and round", () => {
      const setPendingGuess = vi.fn();
      const setCurrentGuess = vi.fn();
      const setGameState = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        gameState: { ...mockGameState, current_phase: "Guessing" },
        pendingGuess: "ABOUT",
        setPendingGuess,
        setCurrentGuess,
        setGameState,
      } as any);

      renderComponent();

      // Simulate same phase/round update (e.g., player joined)
      const newState = {
        ...mockGameState,
        current_phase: "Guessing" as const,
        players: [
          {
            user_id: "test-user",
            display_name: "Test",
            points: 0,
            guess_history: [],
            is_connected: true,
          },
        ],
      };

      messageHandler({
        GameStateUpdate: { state: newState },
      });

      expect(setGameState).toHaveBeenCalledWith(newState);
      expect(setPendingGuess).not.toHaveBeenCalled();
      expect(setCurrentGuess).not.toHaveBeenCalled();
    });
  });

  describe("Error message handling", () => {
    it("should handle invalid guess error and restore pending guess to current", () => {
      const setLastError = vi.fn();
      const setCurrentGuess = vi.fn();
      const setPendingGuess = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        pendingGuess: "HELLO",
        setLastError,
        setCurrentGuess,
        setPendingGuess,
      } as any);

      renderComponent();

      messageHandler({
        Error: { message: "Invalid guess: word not found" },
      });

      expect(setLastError).toHaveBeenCalledWith(
        "Invalid word - not in our word list",
      );
      expect(setCurrentGuess).toHaveBeenCalledWith("HELLO");
      expect(setPendingGuess).toHaveBeenCalledWith(null);
    });

    it("should handle non-invalid-guess errors normally", () => {
      const setLastError = vi.fn();
      const setCurrentGuess = vi.fn();
      const setPendingGuess = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        pendingGuess: "ABOUT",
        setLastError,
        setCurrentGuess,
        setPendingGuess,
      } as any);

      renderComponent();

      messageHandler({
        Error: { message: "Some other error" },
      });

      expect(setLastError).toHaveBeenCalledWith("Some other error");
      expect(setCurrentGuess).not.toHaveBeenCalled();
      expect(setPendingGuess).not.toHaveBeenCalled();
    });

    it("should ignore rejoin errors gracefully", () => {
      const setLastError = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        setLastError,
      } as any);

      renderComponent();

      messageHandler({
        Error: { message: "No disconnected players to rejoin" },
      });

      expect(setLastError).not.toHaveBeenCalled();
    });
  });

  describe("RoundResult message handling", () => {
    it("should add personal guess to history without clearing pending", () => {
      const addPersonalGuess = vi.fn();
      const setPendingGuess = vi.fn();
      const setCurrentGuess = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        addPersonalGuess,
        setPendingGuess,
        setCurrentGuess,
      } as any);

      renderComponent();

      const personalGuess = {
        word: "ABOUT",
        points_earned: 5,
        was_winning_guess: false,
        timestamp: "2024-01-01T00:01:00Z",
      };

      const guessResult = {
        word: "ABOUT",
        player_id: "test-player",
        letters: [
          { letter: "A", status: "Present" as const, position: 0 },
          { letter: "B", status: "Absent" as const, position: 1 },
          { letter: "O", status: "Absent" as const, position: 2 },
          { letter: "U", status: "Absent" as const, position: 3 },
          { letter: "T", status: "Absent" as const, position: 4 },
        ],
        points_earned: 5,
        timestamp: "2024-01-01T00:01:00Z",
      };

      messageHandler({
        RoundResult: {
          winning_guess: guessResult,
          your_guess: personalGuess,
          next_phase: "IndividualGuess",
          is_word_completed: false,
        },
      });

      expect(addPersonalGuess).toHaveBeenCalledWith(personalGuess);
      // Should not clear pending guess - let GameStateUpdate handle it
      expect(setPendingGuess).not.toHaveBeenCalled();
      expect(setCurrentGuess).not.toHaveBeenCalled();
    });
  });

  describe("Component lifecycle", () => {
    it("should register and unregister message handler", () => {
      const { unmount } = renderComponent();

      expect(addMessageHandler).toHaveBeenCalled();

      unmount();

      expect(removeMessageHandler).toHaveBeenCalled();
    });

    it("should not register handler when not authenticated", () => {
      mockUseWebSocket.mockReturnValue({
        ...defaultWebSocketState,
        isAuthenticated: false,
      } as any);

      renderComponent();

      expect(addMessageHandler).not.toHaveBeenCalled();
    });
  });
});
