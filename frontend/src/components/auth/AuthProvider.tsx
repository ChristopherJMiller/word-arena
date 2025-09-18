import React, { createContext, useContext, useEffect, useState } from "react";
import { PublicClientApplication, AccountInfo } from "@azure/msal-browser";
import { MsalProvider } from "@azure/msal-react";
import { User } from "../../types/generated/User";

// MSAL configuration
const msalConfig = {
  auth: {
    clientId: import.meta.env.VITE_AZURE_CLIENT_ID || "your-client-id",
    authority: `https://login.microsoftonline.com/${import.meta.env.VITE_AZURE_TENANT_ID || "common"}`,
    redirectUri: window.location.origin,
  },
  cache: {
    cacheLocation: "sessionStorage",
    storeAuthStateInCookie: false,
  },
};

// Create MSAL instance
const msalInstance = new PublicClientApplication(msalConfig);

// Auth context types
interface AuthContextType {
  user: User | null;
  accessToken: string | null;
  isAuthenticated: boolean;
  login: () => Promise<void>;
  logout: () => Promise<void>;
  getAccessToken: () => Promise<string | null>;
  devLogin?: (displayName: string, email?: string) => void;
  isDevMode: boolean;
}

const AuthContext = createContext<AuthContextType | null>(null);

interface AuthProviderProps {
  children: React.ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null);
  const [accessToken, setAccessToken] = useState<string | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const isDevMode = import.meta.env.VITE_AUTH_DEV_MODE === "true";

  // Create a mock JWT token for development
  const createMockJWT = (
    userId: string,
    email: string,
    displayName: string,
  ): string => {
    // JWT header (base64 encoded)
    const header = {
      alg: "RS256",
      typ: "JWT",
      kid: "dev-key-id",
    };

    // JWT payload/claims
    const payload = {
      aud: "dev-client-id",
      iss: "https://login.microsoftonline.com/dev/v2.0",
      iat: Math.floor(Date.now() / 1000),
      exp: Math.floor(Date.now() / 1000) + 3600, // 1 hour from now
      sub: userId,
      email: email,
      name: displayName,
      preferred_username: email,
    };

    // Base64 encode header and payload
    const encodedHeader = btoa(JSON.stringify(header))
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=/g, "");
    const encodedPayload = btoa(JSON.stringify(payload))
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=/g, "");

    // Create mock signature (just base64 encoded "dev-signature")
    const mockSignature = btoa("dev-signature")
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=/g, "");

    // Return complete JWT
    return `${encodedHeader}.${encodedPayload}.${mockSignature}`;
  };

  // Generate a UUID v4-like string for development
  const generateUUID = (): string => {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0;
      const v = c === 'x' ? r : (r & 0x3) | 0x8;
      return v.toString(16);
    });
  };

  // Manual dev login function exposed to components
  const devLogin = (displayName: string, email?: string) => {
    const userId = generateUUID();
    const userEmail =
      email ||
      `${displayName.toLowerCase().replace(/\s+/g, ".")}@dev.example.com`;
    const devUser: User = {
      id: userId,
      email: userEmail,
      display_name: displayName,
      total_points: Math.floor(Math.random() * 500),
      total_wins: Math.floor(Math.random() * 20),
      total_games: Math.floor(Math.random() * 50),
      created_at: new Date().toISOString(),
    };

    // Create mock JWT token
    const devToken = createMockJWT(userId, userEmail, displayName);

    setUser(devUser);
    setAccessToken(devToken);
    setIsAuthenticated(true);

    // Store in localStorage for session persistence
    localStorage.setItem(
      "dev-user",
      JSON.stringify({
        user: devUser,
        token: devToken,
      }),
    );

    console.log(
      "Development mode: manually authenticated as:",
      devUser.display_name,
    );
  };

  useEffect(() => {
    // Development mode - check for stored dev user
    if (isDevMode) {
      const storedDevUser = localStorage.getItem("dev-user");
      if (storedDevUser) {
        try {
          const userData = JSON.parse(storedDevUser);
          
          // Check if user ID is in old format (starts with "dev-user-")
          if (userData.user?.id?.startsWith("dev-user-")) {
            console.log("Migrating old dev user ID format to UUID");
            localStorage.removeItem("dev-user");
            // Will force re-authentication with new UUID format
            return;
          }
          
          setUser(userData.user);
          setAccessToken(userData.token);
          setIsAuthenticated(true);
          console.log(
            "Development mode: restored session for:",
            userData.user.display_name,
          );
        } catch (error) {
          console.error("Failed to restore dev user session:", error);
          localStorage.removeItem("dev-user");
        }
      }
      return;
    }

    // Initialize MSAL and check for existing authentication
    const initializeMsal = async () => {
      try {
        await msalInstance.initialize();

        // Check if user is already authenticated
        const accounts = msalInstance.getAllAccounts();
        if (accounts.length > 0) {
          const account = accounts[0];
          await handleAuthenticationResult(account);
        }
      } catch (error) {
        console.error("Failed to initialize MSAL:", error);
      }
    };

    initializeMsal();
  }, [isDevMode]);

  const handleAuthenticationResult = async (account: AccountInfo) => {
    try {
      // Get access token
      const tokenResponse = await msalInstance.acquireTokenSilent({
        scopes: ["api://72da9d9f-22a6-45c3-82ec-7b214eca7590/user_impersonation"],
        account,
      });

      const token = tokenResponse.accessToken;
      setAccessToken(token);

      // Create user object from account info
      const userInfo: User = {
        id: account.homeAccountId,
        email: account.username,
        display_name: account.name || account.username,
        total_points: 0,
        total_wins: 0,
        total_games: 0,
        created_at: new Date().toISOString(),
      };

      setUser(userInfo);
      setIsAuthenticated(true);
    } catch (error) {
      console.error("Failed to acquire token:", error);
      setIsAuthenticated(false);
    }
  };

  const login = async () => {
    // In dev mode, authentication is automatic
    if (isDevMode) {
      console.log("Development mode: already authenticated");
      return;
    }

    try {
      const loginResponse = await msalInstance.loginPopup({
        scopes: ["api://72da9d9f-22a6-45c3-82ec-7b214eca7590/user_impersonation"],
      });

      if (loginResponse.account) {
        await handleAuthenticationResult(loginResponse.account);
      }
    } catch (error) {
      console.error("Login failed:", error);
    }
  };

  const logout = async () => {
    // In dev mode, clear localStorage and reset state
    if (isDevMode) {
      localStorage.removeItem("dev-user");
      setUser(null);
      setAccessToken(null);
      setIsAuthenticated(false);
      console.log("Development mode: logged out");
      return;
    }

    try {
      await msalInstance.logoutPopup();
      setUser(null);
      setAccessToken(null);
      setIsAuthenticated(false);
    } catch (error) {
      console.error("Logout failed:", error);
    }
  };

  const getAccessToken = async (): Promise<string | null> => {
    if (!isAuthenticated) return null;

    // In dev mode, return dev token
    if (isDevMode) {
      return accessToken;
    }

    try {
      const accounts = msalInstance.getAllAccounts();
      if (accounts.length === 0) return null;

      const tokenResponse = await msalInstance.acquireTokenSilent({
        scopes: ["api://72da9d9f-22a6-45c3-82ec-7b214eca7590/user_impersonation"],
        account: accounts[0],
      });

      return tokenResponse.accessToken;
    } catch (error) {
      console.error("Failed to acquire token:", error);
      return null;
    }
  };

  const contextValue: AuthContextType = {
    user,
    accessToken,
    isAuthenticated,
    login,
    logout,
    getAccessToken,
    devLogin: isDevMode ? devLogin : undefined,
    isDevMode,
  };

  return (
    <MsalProvider instance={msalInstance}>
      <AuthContext.Provider value={contextValue}>
        {children}
      </AuthContext.Provider>
    </MsalProvider>
  );
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}
