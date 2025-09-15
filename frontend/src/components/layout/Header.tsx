import { Link } from 'react-router-dom'
import { LoginButton } from '../auth/LoginButton'

const Header: React.FC = () => {
  return (
    <header className="bg-white shadow-sm border-b border-gray-200">
      <div className="container mx-auto px-4 py-4">
        <div className="flex items-center justify-between">
          <Link to="/" className="text-2xl font-bold text-blue-600">
            Word Arena
          </Link>
          
          <nav className="flex items-center space-x-4">
            <Link to="/" className="text-gray-600 hover:text-gray-900 transition-colors">
              Lobby
            </Link>
            
            <LoginButton />
          </nav>
        </div>
      </div>
    </header>
  )
}

export default Header