import React, { useState } from 'react';
import { useAuth } from './AuthProvider';

interface DevLoginFormProps {
  onLogin?: () => void;
}

const presetUsers = [
  { name: 'Alice', email: 'alice@dev.example.com' },
  { name: 'Bob', email: 'bob@dev.example.com' },
  { name: 'Charlie', email: 'charlie@dev.example.com' },
  { name: 'Diana', email: 'diana@dev.example.com' },
  { name: 'Eve', email: 'eve@dev.example.com' },
];

export const DevLoginForm: React.FC<DevLoginFormProps> = ({ onLogin }) => {
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [showCustomForm, setShowCustomForm] = useState(false);
  const { devLogin, isDevMode } = useAuth();

  if (!isDevMode || !devLogin) {
    return null;
  }

  const handlePresetLogin = (user: { name: string; email: string }) => {
    devLogin(user.name, user.email);
    onLogin?.();
  };

  const handleCustomLogin = (e: React.FormEvent) => {
    e.preventDefault();
    if (displayName.trim()) {
      devLogin(displayName.trim(), email.trim() || undefined);
      setDisplayName('');
      setEmail('');
      setShowCustomForm(false);
      onLogin?.();
    }
  };

  return (
    <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-6 mb-6">
      <div className="text-center mb-4">
        <div className="inline-flex items-center px-3 py-1 bg-yellow-200 text-yellow-800 text-sm font-medium rounded-full">
          🔧 Development Mode
        </div>
        <h3 className="text-lg font-semibold text-gray-800 mt-2">Quick Login</h3>
        <p className="text-sm text-gray-600">
          Choose a preset user or create a custom one for testing
        </p>
      </div>

      {!showCustomForm ? (
        <div className="space-y-4">
          {/* Preset Users */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
            {presetUsers.map((user) => (
              <button
                key={user.name}
                onClick={() => handlePresetLogin(user)}
                className="p-3 text-left bg-white border border-gray-200 rounded-lg hover:bg-blue-50 hover:border-blue-300 transition-colors"
              >
                <div className="font-semibold text-gray-800">{user.name}</div>
                <div className="text-sm text-gray-500">{user.email}</div>
              </button>
            ))}
          </div>

          {/* Custom User Button */}
          <div className="text-center pt-2 border-t border-yellow-200">
            <button
              onClick={() => setShowCustomForm(true)}
              className="text-blue-600 hover:text-blue-800 font-medium text-sm"
            >
              + Create Custom User
            </button>
          </div>
        </div>
      ) : (
        <form onSubmit={handleCustomLogin} className="space-y-4">
          <div>
            <label htmlFor="displayName" className="block text-sm font-medium text-gray-700 mb-1">
              Display Name *
            </label>
            <input
              type="text"
              id="displayName"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Enter display name"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              required
            />
          </div>
          
          <div>
            <label htmlFor="email" className="block text-sm font-medium text-gray-700 mb-1">
              Email (optional)
            </label>
            <input
              type="email"
              id="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="Will auto-generate if empty"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>

          <div className="flex gap-2">
            <button
              type="submit"
              className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-lg hover:bg-blue-700 transition-colors"
            >
              Login as Custom User
            </button>
            <button
              type="button"
              onClick={() => {
                setShowCustomForm(false);
                setDisplayName('');
                setEmail('');
              }}
              className="px-4 py-2 text-gray-600 hover:text-gray-800 transition-colors"
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      <div className="mt-4 p-3 bg-blue-50 rounded-lg">
        <p className="text-xs text-blue-700">
          💡 <strong>Multi-user testing tip:</strong> Open multiple private/incognito browser windows 
          and login as different users to test multiplayer functionality.
        </p>
      </div>
    </div>
  );
};