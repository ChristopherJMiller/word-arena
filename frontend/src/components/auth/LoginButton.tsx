import { useAuth } from './AuthProvider'
import { DevLoginForm } from './DevLoginForm'

export function LoginButton() {
  const { isAuthenticated, user, login, logout, isDevMode } = useAuth()

  if (isAuthenticated && user) {
    return (
      <div className="flex items-center space-x-4">
        <span className="text-sm text-gray-600">
          Welcome, {user.display_name}
          {isDevMode && <span className="ml-1 text-xs text-yellow-600">(dev)</span>}
        </span>
        <button
          onClick={logout}
          className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
        >
          Sign Out
        </button>
      </div>
    )
  }

  // In dev mode, show the dev login form instead of Microsoft login
  if (isDevMode) {
    return <DevLoginForm />
  }

  return (
    <button
      onClick={login}
      className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
    >
      Sign in with Microsoft
    </button>
  )
}