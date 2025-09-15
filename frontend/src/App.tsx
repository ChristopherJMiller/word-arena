import { BrowserRouter as Router, Routes, Route } from 'react-router-dom'
import { AuthProvider } from './components/auth/AuthProvider'
import Layout from './components/layout/Layout'
import Lobby from './components/lobby/Lobby'
import { Game } from './components/game/Game'

function App() {
  // Test that our types are imported correctly
  console.log('Word Arena Frontend loaded successfully')

  return (
    <AuthProvider>
      <Router>
        <Layout>
          <Routes>
            <Route path="/" element={<Lobby />} />
            <Route path="/game/:gameId" element={<Game />} />
          </Routes>
        </Layout>
      </Router>
    </AuthProvider>
  )
}

export default App