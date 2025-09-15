import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { GameBoardContainer } from './GameBoard';
import { useGameStore } from '../../store/gameStore';
import type { GameState } from '../../types/generated';

// Mock the store
vi.mock('../../store/gameStore');

const mockUseGameStore = vi.mocked(useGameStore);

describe('GameBoardContainer Integration', () => {
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
    official_board: [
      {
        word: 'WORLD',
        player_id: 'player-1',
        letters: [
          { letter: 'W', status: 'Absent', position: 0 },
          { letter: 'O', status: 'Present', position: 1 },
          { letter: 'R', status: 'Absent', position: 2 },
          { letter: 'L', status: 'Correct', position: 3 },
          { letter: 'D', status: 'Absent', position: 4 },
        ],
        points_earned: 3,
        timestamp: '2024-01-01T00:00:00Z',
      }
    ],
    current_winner: null,
    created_at: '2024-01-01T00:00:00Z',
    point_threshold: 25,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('displays "waiting for game" message when no game state', () => {
    mockUseGameStore.mockReturnValue({
      gameState: null,
      currentGuess: '',
      isSubmitting: false,
      countdownEndTime: undefined,
      personalGuessHistory: [],
      setGameState: vi.fn(),
      setCurrentGuess: vi.fn(),
      setIsSubmitting: vi.fn(),
      setCountdownEndTime: vi.fn(),
      addPersonalGuess: vi.fn(),
      resetGame: vi.fn(),
    });

    render(<GameBoardContainer />);
    expect(screen.getByText('Waiting for game to start...')).toBeInTheDocument();
  });

  it('renders game board with state from store', () => {
    mockUseGameStore.mockReturnValue({
      gameState: mockGameState,
      currentGuess: 'TES',
      isSubmitting: false,
      countdownEndTime: undefined,
      personalGuessHistory: [],
      setGameState: vi.fn(),
      setCurrentGuess: vi.fn(),
      setIsSubmitting: vi.fn(),
      setCountdownEndTime: vi.fn(),
      addPersonalGuess: vi.fn(),
      resetGame: vi.fn(),
    });

    render(<GameBoardContainer />);
    
    // Should show the collaborative board title
    expect(screen.getByText('Collaborative Board')).toBeInTheDocument();
    
    // Should show the completed guess from official board
    const correctTiles = screen.getAllByTestId('letter-tile-Correct');
    expect(correctTiles).toHaveLength(1); // L is correct
    
    const presentTiles = screen.getAllByTestId('letter-tile-Present');
    expect(presentTiles).toHaveLength(1); // O is present
    
    const absentTiles = screen.getAllByTestId('letter-tile-Absent');
    expect(absentTiles).toHaveLength(3); // W, R, D are absent
    
    // Should show current guess as pending
    const pendingTiles = screen.getAllByTestId('letter-tile-pending');
    expect(pendingTiles).toHaveLength(3); // T, E, S as pending
  });

  it('updates display when store state changes', () => {
    // Initially no game
    mockUseGameStore.mockReturnValue({
      gameState: null,
      currentGuess: '',
      isSubmitting: false,
      countdownEndTime: undefined,
      personalGuessHistory: [],
      setGameState: vi.fn(),
      setCurrentGuess: vi.fn(),
      setIsSubmitting: vi.fn(),
      setCountdownEndTime: vi.fn(),
      addPersonalGuess: vi.fn(),
      resetGame: vi.fn(),
    });
    
    const { rerender } = render(<GameBoardContainer />);
    expect(screen.getByText('Waiting for game to start...')).toBeInTheDocument();
    
    // Game starts - update mock and rerender
    mockUseGameStore.mockReturnValue({
      gameState: mockGameState,
      currentGuess: '',
      isSubmitting: false,
      countdownEndTime: undefined,
      personalGuessHistory: [],
      setGameState: vi.fn(),
      setCurrentGuess: vi.fn(),
      setIsSubmitting: vi.fn(),
      setCountdownEndTime: vi.fn(),
      addPersonalGuess: vi.fn(),
      resetGame: vi.fn(),
    });
    
    rerender(<GameBoardContainer />);
    expect(screen.getByText('Collaborative Board')).toBeInTheDocument();
  });

  it('handles different word lengths correctly', () => {
    const gameWith7Letters = {
      ...mockGameState,
      word_length: 7,
      official_board: []
    };

    mockUseGameStore.mockReturnValue({
      gameState: gameWith7Letters,
      currentGuess: '',
      isSubmitting: false,
      countdownEndTime: undefined,
      personalGuessHistory: [],
      setGameState: vi.fn(),
      setCurrentGuess: vi.fn(),
      setIsSubmitting: vi.fn(),
      setCountdownEndTime: vi.fn(),
      addPersonalGuess: vi.fn(),
      resetGame: vi.fn(),
    });

    render(<GameBoardContainer />);
    
    // Check that first row has 7 tiles
    const firstRow = screen.getByTestId('game-row-0');
    const tiles = firstRow.querySelectorAll('[data-testid^="letter-tile-"]');
    expect(tiles).toHaveLength(7);
  });
});