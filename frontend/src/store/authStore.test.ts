import { describe, it, expect, beforeEach } from "vitest";
import { useAuthStore } from "./authStore";
import type { User } from "../types/generated";

describe("AuthStore", () => {
  const mockUser: User = {
    id: "user-123",
    email: "test@example.com",
    display_name: "Test User",
    total_points: 150,
    total_wins: 5,
    total_games: 12,
    created_at: "2024-01-01T00:00:00Z",
  };

  beforeEach(() => {
    // Reset store state before each test
    const store = useAuthStore.getState();
    store.logout();
    store.setLoading(false);
  });

  describe("initial state", () => {
    it("should initialize with null user and not authenticated", () => {
      const state = useAuthStore.getState();

      expect(state.user).toBeNull();
      expect(state.isAuthenticated).toBe(false);
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setUser functionality", () => {
    it("should set user and mark as authenticated when user is provided", () => {
      const { setUser } = useAuthStore.getState();

      setUser(mockUser);

      const state = useAuthStore.getState();
      expect(state.user).toEqual(mockUser);
      expect(state.isAuthenticated).toBe(true);
    });

    it("should clear user and mark as not authenticated when null is provided", () => {
      const { setUser } = useAuthStore.getState();

      // First set a user
      setUser(mockUser);
      expect(useAuthStore.getState().isAuthenticated).toBe(true);

      // Then clear it
      setUser(null);

      const state = useAuthStore.getState();
      expect(state.user).toBeNull();
      expect(state.isAuthenticated).toBe(false);
    });

    it("should handle setting different user objects", () => {
      const { setUser } = useAuthStore.getState();
      const user1 = { ...mockUser };
      const user2 = {
        ...mockUser,
        total_points: 200,
        display_name: "Different User",
      };

      setUser(user1);
      expect(useAuthStore.getState().user?.total_points).toBe(150);

      setUser(user2);
      expect(useAuthStore.getState().user?.total_points).toBe(200);
      expect(useAuthStore.getState().user?.display_name).toBe("Different User");
    });

    it("should properly handle truthy/falsy user values", () => {
      const { setUser } = useAuthStore.getState();

      // Test with valid user
      setUser(mockUser);
      expect(useAuthStore.getState().isAuthenticated).toBe(true);

      // Test with null
      setUser(null);
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
    });
  });

  describe("loading state management", () => {
    it("should track loading state independently", () => {
      const { setLoading } = useAuthStore.getState();

      expect(useAuthStore.getState().isLoading).toBe(false);

      setLoading(true);
      expect(useAuthStore.getState().isLoading).toBe(true);

      setLoading(false);
      expect(useAuthStore.getState().isLoading).toBe(false);
    });

    it("should not affect authentication state when loading changes", () => {
      const { setUser, setLoading } = useAuthStore.getState();

      setUser(mockUser);
      expect(useAuthStore.getState().isAuthenticated).toBe(true);

      setLoading(true);
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().user).toEqual(mockUser);

      setLoading(false);
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().user).toEqual(mockUser);
    });
  });

  describe("logout functionality", () => {
    it("should clear user data and authentication state", () => {
      const { setUser, logout } = useAuthStore.getState();

      // Set up authenticated state
      setUser(mockUser);
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().user).toEqual(mockUser);

      // Logout
      logout();

      const state = useAuthStore.getState();
      expect(state.user).toBeNull();
      expect(state.isAuthenticated).toBe(false);
    });

    it("should not affect loading state on logout", () => {
      const { setUser, setLoading, logout } = useAuthStore.getState();

      setUser(mockUser);
      setLoading(true);

      logout();

      const state = useAuthStore.getState();
      expect(state.user).toBeNull();
      expect(state.isAuthenticated).toBe(false);
      expect(state.isLoading).toBe(true); // Should remain unchanged
    });

    it("should be idempotent when called multiple times", () => {
      const { setUser, logout } = useAuthStore.getState();

      setUser(mockUser);

      // Call logout multiple times
      logout();
      logout();
      logout();

      const state = useAuthStore.getState();
      expect(state.user).toBeNull();
      expect(state.isAuthenticated).toBe(false);
    });
  });

  describe("state consistency", () => {
    it("should maintain consistent isAuthenticated flag with user state", () => {
      const { setUser } = useAuthStore.getState();

      // Test various user scenarios
      const testUsers = [
        mockUser,
        { ...mockUser, id: "different-id" },
        { ...mockUser, email: "other@example.com" },
      ];

      testUsers.forEach((user) => {
        setUser(user);
        expect(useAuthStore.getState().isAuthenticated).toBe(true);
        expect(useAuthStore.getState().user).toEqual(user);
      });

      setUser(null);
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      expect(useAuthStore.getState().user).toBeNull();
    });

    it("should handle rapid state changes correctly", () => {
      const { setUser, setLoading, logout } = useAuthStore.getState();

      // Rapid state changes
      setLoading(true);
      setUser(mockUser);
      setLoading(false);
      logout();
      setUser(mockUser);

      const finalState = useAuthStore.getState();
      expect(finalState.user).toEqual(mockUser);
      expect(finalState.isAuthenticated).toBe(true);
      expect(finalState.isLoading).toBe(false);
    });
  });

  describe("edge cases", () => {
    it("should handle user objects with missing optional fields", () => {
      const { setUser } = useAuthStore.getState();
      const minimalUser: User = {
        id: "minimal-user",
        email: "minimal@example.com",
        display_name: "Minimal",
        total_points: 0,
        total_wins: 0,
        total_games: 0,
        created_at: "2024-01-01T00:00:00Z",
      };

      setUser(minimalUser);

      const state = useAuthStore.getState();
      expect(state.user).toEqual(minimalUser);
      expect(state.isAuthenticated).toBe(true);
    });

    it("should handle user with zero values correctly", () => {
      const { setUser } = useAuthStore.getState();
      const zeroUser: User = {
        ...mockUser,
        total_points: 0,
        total_wins: 0,
        total_games: 0,
      };

      setUser(zeroUser);

      const state = useAuthStore.getState();
      expect(state.user).toEqual(zeroUser);
      expect(state.isAuthenticated).toBe(true); // Should still be authenticated
    });

    it("should maintain state isolation between multiple calls", () => {
      const { setUser, setLoading } = useAuthStore.getState();

      // Simulate concurrent operations
      const operations = [];

      operations.push(() => setUser(mockUser));
      operations.push(() => setLoading(true));
      operations.push(() => setUser(null));
      operations.push(() => setLoading(false));
      operations.push(() => setUser(mockUser));

      // Execute all operations
      operations.forEach((op) => op());

      const finalState = useAuthStore.getState();
      expect(finalState.user).toEqual(mockUser);
      expect(finalState.isAuthenticated).toBe(true);
      expect(finalState.isLoading).toBe(false);
    });
  });

  describe("authentication workflows", () => {
    it("should support login workflow", () => {
      const { setLoading, setUser } = useAuthStore.getState();

      // Start login process
      setLoading(true);
      expect(useAuthStore.getState().isLoading).toBe(true);
      expect(useAuthStore.getState().isAuthenticated).toBe(false);

      // Complete login
      setUser(mockUser);
      setLoading(false);

      const state = useAuthStore.getState();
      expect(state.isLoading).toBe(false);
      expect(state.isAuthenticated).toBe(true);
      expect(state.user).toEqual(mockUser);
    });

    it("should support failed login workflow", () => {
      const { setLoading } = useAuthStore.getState();

      // Start login process
      setLoading(true);

      // Simulate login failure
      setLoading(false);
      // Don't set user (login failed)

      const state = useAuthStore.getState();
      expect(state.isLoading).toBe(false);
      expect(state.isAuthenticated).toBe(false);
      expect(state.user).toBeNull();
    });

    it("should support user profile update workflow", () => {
      const { setUser } = useAuthStore.getState();

      // Initial login
      setUser(mockUser);

      // Update user profile
      const updatedUser: User = {
        ...mockUser,
        display_name: "Updated Name",
        total_points: 200,
        total_wins: 7,
      };

      setUser(updatedUser);

      const state = useAuthStore.getState();
      expect(state.user).toEqual(updatedUser);
      expect(state.isAuthenticated).toBe(true);
    });
  });
});
