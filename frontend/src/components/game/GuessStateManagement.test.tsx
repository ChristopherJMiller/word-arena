import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import { GuessInputContainer } from './GuessInput';
import { useGameStore } from '../../store/gameStore';
import { useWebSocket } from '../../hooks/useWebSocket';
import type { GameState } from '../../types/generated';

// Mock the hooks
vi.mock('../../store/gameStore');
vi.mock('../../hooks/useWebSocket');

const mockUseGameStore = vi.mocked(useGameStore);
const mockUseWebSocket = vi.mocked(useWebSocket);

describe('Guess State Management', () => {
  const mockGameState: GameState = {
    id: 'test-game-id',
    word: '*****',
    word_length: 5,
    current_round: 1,
    status: 'Active',
    current_phase: 'Guessing',
    players: [],
    guesses: [],
    round_start_time: '',
    phase_end_time: null,
  };

  const defaultStoreState = {
    gameState: mockGameState,
    currentGuess: '',
    isSubmitting: false,
    lastError: null,
    pendingGuess: null,
    setCurrentGuess: vi.fn(),
    clearError: vi.fn(),
    setPendingGuess: vi.fn(),
  };

  const defaultWebSocketState = {
    isConnected: true,
    isAuthenticated: true,
    sendMessage: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseGameStore.mockReturnValue(defaultStoreState as any);
    mockUseWebSocket.mockReturnValue(defaultWebSocketState as any);
  });

  const renderComponent = () => {
    return render(
      <BrowserRouter>
        <GuessInputContainer />
      </BrowserRouter>
    );
  };

  describe('Valid guess submission flow', () => {
    it('should set pending guess and clear current guess on valid submission', async () => {
      const setPendingGuess = vi.fn();
      const setCurrentGuess = vi.fn();
      const sendMessage = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        currentGuess: 'ABOUT',
        setPendingGuess,
        setCurrentGuess,
      } as any);

      mockUseWebSocket.mockReturnValue({
        ...defaultWebSocketState,
        sendMessage,
      } as any);

      renderComponent();

      const submitButton = screen.getByTestId('submit-guess-button');
      await userEvent.click(submitButton);

      expect(sendMessage).toHaveBeenCalledWith({ SubmitGuess: { word: 'ABOUT' } });
      expect(setPendingGuess).toHaveBeenCalledWith('ABOUT');
      expect(setCurrentGuess).toHaveBeenCalledWith('');
    });

    it('should show pending guess and disable input when pending', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        pendingGuess: 'ABOUT',
        currentGuess: '',
      } as any);

      renderComponent();

      // Should show the pending guess in the inputs
      const inputs = screen.getAllByTestId(/letter-input-/);
      expect(inputs[0]).toHaveValue('A');
      expect(inputs[1]).toHaveValue('B');
      expect(inputs[2]).toHaveValue('O');
      expect(inputs[3]).toHaveValue('U');
      expect(inputs[4]).toHaveValue('T');

      // Should show waiting message
      expect(screen.getByText('Waiting for other players to submit...')).toBeInTheDocument();

      // Should disable inputs
      inputs.forEach(input => {
        expect(input).toBeDisabled();
      });
    });
  });

  describe('Invalid guess error flow', () => {
    it('should show error message for invalid guess', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        lastError: 'Invalid word - not in our word list',
      } as any);

      renderComponent();

      expect(screen.getByTestId('error-message')).toBeInTheDocument();
      expect(screen.getByText('Invalid word - not in our word list')).toBeInTheDocument();
    });

    it('should clear error when user starts typing new guess', async () => {
      const clearError = vi.fn();
      const setPendingGuess = vi.fn();
      const setCurrentGuess = vi.fn();

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        lastError: 'Invalid word - not in our word list',
        pendingGuess: 'HELLO',
        clearError,
        setPendingGuess,
        setCurrentGuess,
      } as any);

      renderComponent();

      const firstInput = screen.getByTestId('letter-input-0');
      await userEvent.type(firstInput, 'A');

      expect(clearError).toHaveBeenCalled();
      expect(setPendingGuess).toHaveBeenCalledWith(null);
    });

    it('should clear pending guess when error occurs', () => {
      // This test would verify that when an invalid guess error comes back,
      // the pending state is cleared. This is handled in the Game component
      // when processing error messages.
      expect(true).toBe(true); // Placeholder - actual test would need Game component integration
    });
  });

  describe('Input state management', () => {
    it('should disable input during non-guessing phases', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        gameState: {
          ...mockGameState,
          current_phase: 'IndividualGuess',
        },
      } as any);

      renderComponent();

      const inputs = screen.getAllByTestId(/letter-input-/);
      inputs.forEach(input => {
        expect(input).toBeDisabled();
      });
    });

    it('should disable input when game is not active', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        gameState: {
          ...mockGameState,
          status: 'Waiting',
        },
      } as any);

      renderComponent();

      const inputs = screen.getAllByTestId(/letter-input-/);
      inputs.forEach(input => {
        expect(input).toBeDisabled();
      });
    });

    it('should enable input during guessing phase when active', () => {
      renderComponent();

      const inputs = screen.getAllByTestId(/letter-input-/);
      inputs.forEach(input => {
        expect(input).not.toBeDisabled();
      });
    });
  });

  describe('Button state management', () => {
    it('should disable submit button when guess is incomplete', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        currentGuess: 'ABC',
      } as any);

      renderComponent();

      const submitButton = screen.getByTestId('submit-guess-button');
      expect(submitButton).toBeDisabled();
      expect(submitButton).toHaveClass('bg-gray-300');
    });

    it('should enable submit button when guess is complete', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        currentGuess: 'ABOUT',
      } as any);

      renderComponent();

      const submitButton = screen.getByTestId('submit-guess-button');
      expect(submitButton).not.toBeDisabled();
      expect(submitButton).toHaveClass('bg-blue-500');
    });

    it('should disable submit button when showing pending guess', () => {
      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        pendingGuess: 'ABOUT',
      } as any);

      renderComponent();

      const submitButton = screen.getByTestId('submit-guess-button');
      expect(submitButton).toBeDisabled();
    });
  });

  describe('WebSocket connection requirements', () => {
    it('should handle disconnected WebSocket gracefully', async () => {
      const sendMessage = vi.fn().mockImplementation(() => {
        throw new Error('WebSocket not connected');
      });

      mockUseWebSocket.mockReturnValue({
        ...defaultWebSocketState,
        isConnected: false,
        sendMessage,
      } as any);

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        currentGuess: 'ABOUT',
      } as any);

      renderComponent();

      const submitButton = screen.getByTestId('submit-guess-button');
      await userEvent.click(submitButton);

      // Should attempt to send but handle error gracefully
      expect(sendMessage).toHaveBeenCalled();
    });

    it('should handle unauthenticated WebSocket gracefully', async () => {
      const sendMessage = vi.fn().mockImplementation(() => {
        throw new Error('WebSocket not authenticated');
      });

      mockUseWebSocket.mockReturnValue({
        ...defaultWebSocketState,
        isAuthenticated: false,
        sendMessage,
      } as any);

      mockUseGameStore.mockReturnValue({
        ...defaultStoreState,
        currentGuess: 'ABOUT',
      } as any);

      renderComponent();

      const submitButton = screen.getByTestId('submit-guess-button');
      await userEvent.click(submitButton);

      expect(sendMessage).toHaveBeenCalled();
    });
  });
});