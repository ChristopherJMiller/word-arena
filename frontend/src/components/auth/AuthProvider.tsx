import React, { createContext, useContext, useEffect, useState } from 'react'
import { PublicClientApplication, AccountInfo } from '@azure/msal-browser'
import { MsalProvider } from '@azure/msal-react'
import { User } from '../../types/generated/User'

// MSAL configuration
const msalConfig = {
  auth: {
    clientId: import.meta.env.VITE_AZURE_CLIENT_ID || 'your-client-id',
    authority: `https://login.microsoftonline.com/${import.meta.env.VITE_AZURE_TENANT_ID || 'common'}`,
    redirectUri: window.location.origin,
  },
  cache: {
    cacheLocation: 'sessionStorage',
    storeAuthStateInCookie: false,
  },
}

// Create MSAL instance
const msalInstance = new PublicClientApplication(msalConfig)

// Auth context types
interface AuthContextType {
  user: User | null
  accessToken: string | null
  isAuthenticated: boolean
  login: () => Promise<void>
  logout: () => Promise<void>
  getAccessToken: () => Promise<string | null>
  devLogin?: (displayName: string, email?: string) => void
  isDevMode: boolean
}

const AuthContext = createContext<AuthContextType | null>(null)

interface AuthProviderProps {
  children: React.ReactNode
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null)
  const [accessToken, setAccessToken] = useState<string | null>(null)
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const isDevMode = import.meta.env.VITE_AUTH_DEV_MODE === 'true'

  // Manual dev login function exposed to components
  const devLogin = (displayName: string, email?: string) => {
    const userId = `dev-user-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
    const devUser: User = {
      id: userId,
      email: email || `${displayName.toLowerCase().replace(/\s+/g, '.')}@dev.example.com`,
      display_name: displayName,
      total_points: Math.floor(Math.random() * 500),
      total_wins: Math.floor(Math.random() * 20),
      created_at: new Date().toISOString(),
    }
    setUser(devUser)
    setAccessToken(`dev-token-${userId}`)
    setIsAuthenticated(true)
    
    // Store in localStorage for session persistence
    localStorage.setItem('dev-user', JSON.stringify({
      user: devUser,
      token: `dev-token-${userId}`
    }))
    
    console.log('Development mode: manually authenticated as:', devUser.display_name)
  }

  useEffect(() => {
    // Development mode - check for stored dev user
    if (isDevMode) {
      const storedDevUser = localStorage.getItem('dev-user')
      if (storedDevUser) {
        try {
          const userData = JSON.parse(storedDevUser)
          setUser(userData.user)
          setAccessToken(userData.token)
          setIsAuthenticated(true)
          console.log('Development mode: restored session for:', userData.user.display_name)
        } catch (error) {
          console.error('Failed to restore dev user session:', error)
          localStorage.removeItem('dev-user')
        }
      }
      return
    }

    // Initialize MSAL and check for existing authentication
    const initializeMsal = async () => {
      try {
        await msalInstance.initialize()
        
        // Check if user is already authenticated
        const accounts = msalInstance.getAllAccounts()
        if (accounts.length > 0) {
          const account = accounts[0]
          await handleAuthenticationResult(account)
        }
      } catch (error) {
        console.error('Failed to initialize MSAL:', error)
      }
    }

    initializeMsal()
  }, [isDevMode])

  const handleAuthenticationResult = async (account: AccountInfo) => {
    try {
      // Get access token
      const tokenResponse = await msalInstance.acquireTokenSilent({
        scopes: ['openid', 'profile', 'email'],
        account,
      })

      const token = tokenResponse.accessToken
      setAccessToken(token)

      // Create user object from account info
      const userInfo: User = {
        id: account.homeAccountId,
        email: account.username,
        display_name: account.name || account.username,
        total_points: 0,
        total_wins: 0,
        created_at: new Date().toISOString(),
      }

      setUser(userInfo)
      setIsAuthenticated(true)
    } catch (error) {
      console.error('Failed to acquire token:', error)
      setIsAuthenticated(false)
    }
  }

  const login = async () => {
    // In dev mode, authentication is automatic
    if (isDevMode) {
      console.log('Development mode: already authenticated')
      return
    }

    try {
      const loginResponse = await msalInstance.loginPopup({
        scopes: ['openid', 'profile', 'email'],
      })

      if (loginResponse.account) {
        await handleAuthenticationResult(loginResponse.account)
      }
    } catch (error) {
      console.error('Login failed:', error)
    }
  }

  const logout = async () => {
    // In dev mode, clear localStorage and reset state
    if (isDevMode) {
      localStorage.removeItem('dev-user')
      setUser(null)
      setAccessToken(null)
      setIsAuthenticated(false)
      console.log('Development mode: logged out')
      return
    }

    try {
      await msalInstance.logoutPopup()
      setUser(null)
      setAccessToken(null)
      setIsAuthenticated(false)
    } catch (error) {
      console.error('Logout failed:', error)
    }
  }

  const getAccessToken = async (): Promise<string | null> => {
    if (!isAuthenticated) return null

    // In dev mode, return dev token
    if (isDevMode) {
      return accessToken
    }

    try {
      const accounts = msalInstance.getAllAccounts()
      if (accounts.length === 0) return null

      const tokenResponse = await msalInstance.acquireTokenSilent({
        scopes: ['openid', 'profile', 'email'],
        account: accounts[0],
      })

      return tokenResponse.accessToken
    } catch (error) {
      console.error('Failed to acquire token:', error)
      return null
    }
  }

  const contextValue: AuthContextType = {
    user,
    accessToken,
    isAuthenticated,
    login,
    logout,
    getAccessToken,
    devLogin: isDevMode ? devLogin : undefined,
    isDevMode,
  }

  return (
    <MsalProvider instance={msalInstance}>
      <AuthContext.Provider value={contextValue}>
        {children}
      </AuthContext.Provider>
    </MsalProvider>
  )
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}