import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { gameHttpClient } from './gameHttpClient';
import type { SafeGameState } from '../types/generated';

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('GameHttpClient', () => {
  const mockSafeGameState: SafeGameState = {
    id: 'test-game-123',
    word_length: 5,
    current_round: 2,
    status: 'Active',
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

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('getGameState', () => {
    it('should fetch game state successfully', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockSafeGameState),
      });

      const result = await gameHttpClient.getGameState('test-game-123');

      expect(mockFetch).toHaveBeenCalledWith('/api/game/test-game-123/state');
      expect(result).toEqual(mockSafeGameState);
    });

    it('should use correct API endpoint format', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockSafeGameState),
      });

      await gameHttpClient.getGameState('abc-456-def');

      expect(mockFetch).toHaveBeenCalledWith('/api/game/abc-456-def/state');
    });

    it('should throw error for non-ok response', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        statusText: 'Not Found',
      });

      await expect(gameHttpClient.getGameState('nonexistent-game')).rejects.toThrow(
        'Failed to fetch game state: 404 Not Found'
      );
    });

    it('should handle different HTTP error codes', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      });

      await expect(gameHttpClient.getGameState('test-game')).rejects.toThrow(
        'Failed to fetch game state: 500 Internal Server Error'
      );
    });

    it('should handle fetch network errors', async () => {
      mockFetch.mockRejectedValue(new Error('Network error'));

      await expect(gameHttpClient.getGameState('test-game')).rejects.toThrow('Network error');
    });

    it('should handle JSON parsing errors', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.reject(new Error('Invalid JSON')),
      });

      await expect(gameHttpClient.getGameState('test-game')).rejects.toThrow('Invalid JSON');
    });

    it('should handle empty gameId', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockSafeGameState),
      });

      await gameHttpClient.getGameState('');

      expect(mockFetch).toHaveBeenCalledWith('/api/game//state');
    });

    it('should handle special characters in gameId', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockSafeGameState),
      });

      const gameIdWithSpecialChars = 'game-123_abc!@#';
      await gameHttpClient.getGameState(gameIdWithSpecialChars);

      expect(mockFetch).toHaveBeenCalledWith(`/api/game/${gameIdWithSpecialChars}/state`);
    });

    it('should preserve response data structure', async () => {
      const customGameState = {
        ...mockSafeGameState,
        id: 'custom-game',
        current_round: 5,
        players: [
          {
            user_id: 'user-1',
            display_name: 'Alice',
            points: 15,
            guess_history: [],
            is_connected: true,
          },
          {
            user_id: 'user-2', 
            display_name: 'Bob',
            points: 8,
            guess_history: [],
            is_connected: false,
          },
        ],
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(customGameState),
      });

      const result = await gameHttpClient.getGameState('custom-game');

      expect(result).toEqual(customGameState);
      expect(result.players).toHaveLength(2);
      expect(result.players[0].display_name).toBe('Alice');
      expect(result.players[1].is_connected).toBe(false);
    });
  });

  describe('HTTP client configuration', () => {
    it('should use default base URL', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockSafeGameState),
      });

      await gameHttpClient.getGameState('test');

      // Should call with /api prefix (default baseUrl)
      expect(mockFetch).toHaveBeenCalledWith('/api/game/test/state');
    });
  });

  describe('error message formatting', () => {
    it('should include status code and status text in error message', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
      });

      try {
        await gameHttpClient.getGameState('forbidden-game');
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect(error).toBeInstanceOf(Error);
        expect((error as Error).message).toBe('Failed to fetch game state: 403 Forbidden');
      }
    });

    it('should handle missing status text', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 418,
        statusText: '',
      });

      try {
        await gameHttpClient.getGameState('teapot-game');
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect((error as Error).message).toBe('Failed to fetch game state: 418 ');
      }
    });
  });
});