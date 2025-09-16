import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { act } from "@testing-library/react";
import { WebSocketService } from "./websocketService";
import type { ServerMessage, ClientMessage } from "../types/generated";

// Mock WebSocket globally
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState = MockWebSocket.CONNECTING;
  url: string;
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    // Simulate async connection
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      if (this.onopen) {
        this.onopen(new Event("open"));
      }
    }, 0);
  }

  send = vi.fn();
  close = vi.fn(() => {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(
        new CloseEvent("close", { code: 1000, reason: "Normal closure" }),
      );
    }
  });

  // Test helper methods
  simulateMessage(data: string) {
    if (this.onmessage) {
      this.onmessage(new MessageEvent("message", { data }));
    }
  }

  simulateError() {
    if (this.onerror) {
      this.onerror(new Event("error"));
    }
  }

  simulateClose(code = 1000, reason = "Normal closure") {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent("close", { code, reason }));
    }
  }
}

// @ts-ignore
global.WebSocket = MockWebSocket;

describe("WebSocketService", () => {
  let service: WebSocketService;
  let mockConsoleLog: any;
  let mockConsoleError: any;

  beforeEach(() => {
    service = new WebSocketService("ws://localhost:8080/ws");
    mockConsoleLog = vi.spyOn(console, "log").mockImplementation(() => {});
    mockConsoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.clearAllTimers();
    vi.useFakeTimers();
  });

  afterEach(() => {
    service.disconnect();
    mockConsoleLog.mockRestore();
    mockConsoleError.mockRestore();
    vi.useRealTimers();
  });

  describe("connection lifecycle", () => {
    it("should connect successfully", async () => {
      const connectPromise = service.connect();

      // Fast-forward to allow connection to complete
      await vi.runOnlyPendingTimersAsync();
      await connectPromise;

      expect(service.isConnected).toBe(true);
      expect(mockConsoleLog).toHaveBeenCalledWith("WebSocket connected");
    });

    it("should handle connection errors", async () => {
      const connectPromise = service.connect();

      // Simulate connection error before it opens
      const mockWs = (service as any).ws as MockWebSocket;
      act(() => {
        mockWs.simulateError();
      });

      await expect(connectPromise).rejects.toThrow();
    });

    it("should disconnect properly", async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();

      service.disconnect();
      // Run any pending timers for disconnect operations
      await vi.runOnlyPendingTimersAsync();

      expect(service.isConnected).toBe(false);
      expect(service.authenticated).toBe(false);
    }, 10000);

    it("should reset authentication on disconnect", async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();

      // Simulate authentication
      const authPromise = service.authenticate("test-token");
      const mockWs = (service as any).ws as MockWebSocket;

      setTimeout(() => {
        mockWs.simulateMessage(
          JSON.stringify({
            AuthenticationSuccess: {
              user: {
                id: "test",
                email: "test@example.com",
                display_name: "Test",
                total_points: 0,
                total_wins: 0,
                total_games: 0,
                created_at: "2024-01-01T00:00:00Z",
              },
            },
          }),
        );
      }, 0);

      await vi.runOnlyPendingTimersAsync();
      await authPromise;

      expect(service.authenticated).toBe(true);

      // Disconnect and verify auth is reset
      service.disconnect();
      expect(service.authenticated).toBe(false);
    });
  });

  describe("authentication flow", () => {
    beforeEach(async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();
    });

    it("should authenticate successfully", async () => {
      const authPromise = service.authenticate("valid-token");
      const mockWs = (service as any).ws as MockWebSocket;

      setTimeout(() => {
        mockWs.simulateMessage(
          JSON.stringify({
            AuthenticationSuccess: {
              user: {
                id: "user-123",
                email: "test@example.com",
                display_name: "Test User",
                total_points: 100,
                total_wins: 5,
                total_games: 10,
                created_at: "2024-01-01T00:00:00Z",
              },
            },
          }),
        );
      }, 0);

      await vi.runOnlyPendingTimersAsync();
      const result = await authPromise;

      expect(result).toBe(true);
      expect(service.authenticated).toBe(true);
      expect(mockWs.send).toHaveBeenCalledWith(
        JSON.stringify({ Authenticate: { token: "valid-token" } }),
      );
    });

    it("should handle authentication failure", async () => {
      const authPromise = service.authenticate("invalid-token");
      const mockWs = (service as any).ws as MockWebSocket;

      setTimeout(() => {
        mockWs.simulateMessage(
          JSON.stringify({
            AuthenticationFailed: { reason: "Invalid token" },
          }),
        );
      }, 0);

      await vi.runOnlyPendingTimersAsync();
      const result = await authPromise;

      expect(result).toBe(false);
      expect(service.authenticated).toBe(false);
      expect(mockConsoleError).toHaveBeenCalledWith(
        "Authentication failed:",
        "Invalid token",
      );
    });

    it("should timeout authentication after 10 seconds", async () => {
      const authPromise = service.authenticate("timeout-token");

      // Fast-forward 10 seconds
      vi.advanceTimersByTime(10000);
      await vi.runOnlyPendingTimersAsync();

      const result = await authPromise;
      expect(result).toBe(false);
    });

    it("should throw error when not connected", async () => {
      service.disconnect();

      await expect(service.authenticate("test-token")).rejects.toThrow(
        "WebSocket not connected",
      );
    });
  });

  describe("message handling", () => {
    beforeEach(async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();
    });

    it("should add and call message handlers", async () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      service.addMessageHandler(handler1);
      service.addMessageHandler(handler2);

      const testMessage: ServerMessage = {
        QueueJoined: { position: 1 },
      };

      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateMessage(JSON.stringify(testMessage));

      expect(handler1).toHaveBeenCalledWith(testMessage);
      expect(handler2).toHaveBeenCalledWith(testMessage);
    });

    it("should remove message handlers", async () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      service.addMessageHandler(handler1);
      service.addMessageHandler(handler2);
      service.removeMessageHandler(handler1);

      const testMessage: ServerMessage = {
        QueueJoined: { position: 1 },
      };

      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateMessage(JSON.stringify(testMessage));

      expect(handler1).not.toHaveBeenCalled();
      expect(handler2).toHaveBeenCalledWith(testMessage);
    });

    it("should handle malformed JSON gracefully", async () => {
      const handler = vi.fn();
      service.addMessageHandler(handler);

      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateMessage("invalid json");

      expect(handler).not.toHaveBeenCalled();
      expect(mockConsoleError).toHaveBeenCalledWith(
        "Failed to parse WebSocket message:",
        expect.any(Error),
      );
    });

    it("should handle handler errors without affecting other handlers", async () => {
      const errorHandler = vi.fn().mockImplementation(() => {
        throw new Error("Handler error");
      });
      const goodHandler = vi.fn();

      service.addMessageHandler(errorHandler);
      service.addMessageHandler(goodHandler);

      const testMessage: ServerMessage = {
        QueueJoined: { position: 1 },
      };

      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateMessage(JSON.stringify(testMessage));

      expect(errorHandler).toHaveBeenCalled();
      expect(goodHandler).toHaveBeenCalled();
      expect(mockConsoleError).toHaveBeenCalledWith(
        "Error in message handler:",
        expect.any(Error),
      );
    });
  });

  describe("message sending", () => {
    beforeEach(async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();
    });

    it("should send messages when connected", () => {
      const message: ClientMessage = "JoinQueue";

      service.sendMessage(message);

      const mockWs = (service as any).ws as MockWebSocket;
      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify(message));
    });

    it("should throw error when not connected", () => {
      service.disconnect();

      const message: ClientMessage = "JoinQueue";

      expect(() => service.sendMessage(message)).toThrow(
        "WebSocket not connected",
      );
    });

    it("should send rejoin game messages", async () => {
      service.rejoinGame("game-123");

      const mockWs = (service as any).ws as MockWebSocket;
      expect(mockWs.send).toHaveBeenCalledWith(
        JSON.stringify({ RejoinGame: { game_id: "game-123" } }),
      );
    });
  });

  describe("reconnection logic", () => {
    beforeEach(async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();
    });

    it("should attempt reconnection after disconnect", async () => {
      const connectSpy = vi.spyOn(service, "connect");

      // Simulate unexpected disconnect
      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateClose(1006, "Abnormal closure");

      expect(mockConsoleLog).toHaveBeenCalledWith(
        "WebSocket disconnected:",
        1006,
        "Abnormal closure",
      );
      expect(service.authenticated).toBe(false);

      // Fast-forward reconnection interval
      vi.advanceTimersByTime(1000);
      await vi.runOnlyPendingTimersAsync();

      expect(connectSpy).toHaveBeenCalled();
    });

    it("should re-authenticate after reconnection", async () => {
      // First authenticate
      const authPromise = service.authenticate("test-token");
      const mockWs = (service as any).ws as MockWebSocket;

      setTimeout(() => {
        mockWs.simulateMessage(
          JSON.stringify({
            AuthenticationSuccess: {
              user: {
                id: "test",
                email: "test@example.com",
                display_name: "Test",
                total_points: 0,
                total_wins: 0,
                total_games: 0,
                created_at: "2024-01-01T00:00:00Z",
              },
            },
          }),
        );
      }, 0);

      await vi.runOnlyPendingTimersAsync();
      await authPromise;

      expect(service.authenticated).toBe(true);

      const authenticateSpy = vi.spyOn(service, "authenticate");

      // Simulate disconnect and reconnection
      mockWs.simulateClose(1006);

      // Fast-forward through reconnection
      vi.advanceTimersByTime(1000);
      await vi.runOnlyPendingTimersAsync();

      expect(authenticateSpy).toHaveBeenCalledWith("test-token");
    });

    it("should use exponential backoff for reconnection attempts", async () => {
      const connectSpy = vi
        .spyOn(service, "connect")
        .mockRejectedValue(new Error("Connection failed"));

      // Simulate disconnect
      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateClose(1006);

      // First reconnect attempt (1000ms delay)
      vi.advanceTimersByTime(1000);
      await vi.runOnlyPendingTimersAsync();
      expect(connectSpy).toHaveBeenCalledTimes(1);

      // Second reconnect attempt (2000ms delay)
      vi.advanceTimersByTime(2000);
      await vi.runOnlyPendingTimersAsync();
      expect(connectSpy).toHaveBeenCalledTimes(2);

      // Third reconnect attempt (3000ms delay)
      vi.advanceTimersByTime(3000);
      await vi.runOnlyPendingTimersAsync();
      expect(connectSpy).toHaveBeenCalledTimes(3);
    });

    it("should stop reconnecting after max attempts", async () => {
      const connectSpy = vi
        .spyOn(service, "connect")
        .mockRejectedValue(new Error("Connection failed"));

      // Simulate disconnect
      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateClose(1006);

      // Fast-forward through all reconnection attempts
      for (let i = 1; i <= 5; i++) {
        vi.advanceTimersByTime(i * 1000);
        await vi.runOnlyPendingTimersAsync();
      }

      expect(connectSpy).toHaveBeenCalledTimes(5);
      expect(mockConsoleError).toHaveBeenCalledWith(
        "Max reconnection attempts reached",
      );

      // Should not attempt more reconnections
      vi.advanceTimersByTime(10000);
      await vi.runOnlyPendingTimersAsync();
      expect(connectSpy).toHaveBeenCalledTimes(5);
    });

    it("should reset reconnection attempts on successful connection", async () => {
      // Simulate one failed reconnection
      const connectSpy = vi
        .spyOn(service, "connect")
        .mockRejectedValueOnce(new Error("Connection failed"))
        .mockResolvedValue();

      const mockWs = (service as any).ws as MockWebSocket;
      mockWs.simulateClose(1006);

      // First failed attempt
      vi.advanceTimersByTime(1000);
      await vi.runOnlyPendingTimersAsync();

      // Second successful attempt
      vi.advanceTimersByTime(2000);
      await vi.runOnlyPendingTimersAsync();

      expect(connectSpy).toHaveBeenCalledTimes(2);

      // Verify reconnection attempts were reset by checking next disconnect uses 1000ms delay
      connectSpy.mockRejectedValue(new Error("Connection failed"));
      mockWs.simulateClose(1006);

      vi.advanceTimersByTime(1000); // Should be 1000ms, not 3000ms
      await vi.runOnlyPendingTimersAsync();

      expect(connectSpy).toHaveBeenCalledTimes(3);
    });
  });

  describe("edge cases and error handling", () => {
    it("should handle WebSocket constructor errors", async () => {
      // Mock WebSocket constructor to throw
      const originalWebSocket = global.WebSocket;
      global.WebSocket = Object.assign(
        vi.fn().mockImplementation(() => {
          throw new Error("WebSocket constructor failed");
        }),
        {
          CONNECTING: 0,
          OPEN: 1,
          CLOSING: 2,
          CLOSED: 3,
        }
      ) as any;

      const newService = new WebSocketService("ws://localhost:8080/ws");

      await expect(newService.connect()).rejects.toThrow(
        "WebSocket constructor failed",
      );

      // Restore original WebSocket
      global.WebSocket = originalWebSocket;
    });

    it("should handle multiple simultaneous connections", async () => {
      const service1 = new WebSocketService("ws://localhost:8080/ws");
      const service2 = new WebSocketService("ws://localhost:8081/ws");

      const [result1, result2] = await Promise.allSettled([
        service1.connect().then(() => vi.runOnlyPendingTimersAsync()),
        service2.connect().then(() => vi.runOnlyPendingTimersAsync()),
      ]);

      expect(result1.status).toBe("fulfilled");
      expect(result2.status).toBe("fulfilled");
      expect(service1.isConnected).toBe(true);
      expect(service2.isConnected).toBe(true);

      service1.disconnect();
      service2.disconnect();
    });

    it("should handle rapid connect/disconnect cycles", async () => {
      for (let i = 0; i < 3; i++) {
        await service.connect();
        await vi.runOnlyPendingTimersAsync();
        expect(service.isConnected).toBe(true);

        service.disconnect();
        expect(service.isConnected).toBe(false);
      }
    });

    it("should maintain state consistency during concurrent operations", async () => {
      await service.connect();
      await vi.runOnlyPendingTimersAsync();

      const handler1 = vi.fn();
      const handler2 = vi.fn();

      // Add handlers, send messages, and authenticate simultaneously
      service.addMessageHandler(handler1);
      service.addMessageHandler(handler2);

      const authPromise = service.authenticate("test-token");
      service.sendMessage("JoinQueue");
      service.removeMessageHandler(handler1);

      const mockWs = (service as any).ws as MockWebSocket;
      setTimeout(() => {
        mockWs.simulateMessage(
          JSON.stringify({
            AuthenticationSuccess: {
              user: {
                id: "test",
                email: "test@example.com",
                display_name: "Test",
                total_points: 0,
                total_wins: 0,
                total_games: 0,
                created_at: "2024-01-01T00:00:00Z",
              },
            },
          }),
        );
        mockWs.simulateMessage(
          JSON.stringify({ QueueJoined: { position: 1 } }),
        );
      }, 0);

      await vi.runOnlyPendingTimersAsync();
      await authPromise;

      expect(service.authenticated).toBe(true);
      expect(handler1).not.toHaveBeenCalled();
      expect(handler2).toHaveBeenCalled();
      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify("JoinQueue"));
    });
  });
});
