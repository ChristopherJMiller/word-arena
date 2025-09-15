import type { SafeGameState } from '../types/generated';

export interface GameHttpClient {
  getGameState(gameId: string): Promise<SafeGameState>;
}

class GameHttpClientImpl implements GameHttpClient {
  private baseUrl: string;

  constructor(baseUrl: string = '/api') {
    this.baseUrl = baseUrl;
  }

  async getGameState(gameId: string): Promise<SafeGameState> {
    const response = await fetch(`${this.baseUrl}/game/${gameId}/state`);
    
    if (!response.ok) {
      throw new Error(`Failed to fetch game state: ${response.status} ${response.statusText}`);
    }
    
    return response.json();
  }
}

// Singleton instance
export const gameHttpClient = new GameHttpClientImpl();