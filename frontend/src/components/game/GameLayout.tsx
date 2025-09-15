import React, { useState } from 'react';
import { GameBoardContainer } from './GameBoard';
import { GuessInputContainer } from './GuessInput';
import { PlayerListContainer } from './PlayerList';
import { GuessHistoryContainer } from './GuessHistory';
import { CountdownTimerContainer } from './CountdownTimer';

export const GameLayout: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'players' | 'history'>('players');

  return (
    <div className="min-h-screen bg-gray-50 p-4">
      {/* Desktop Layout - 3 columns */}
      <div className="hidden lg:grid lg:grid-cols-[300px_1fr_300px] gap-6 max-w-7xl mx-auto">
        {/* Left Column - Players */}
        <div className="bg-white rounded-lg shadow-md p-4">
          <PlayerListContainer />
        </div>

        {/* Center Column - Game Board */}
        <div className="flex flex-col gap-6">
          {/* Timer */}
          <div className="bg-white rounded-lg shadow-md p-4">
            <CountdownTimerContainer />
          </div>

          {/* Game Board */}
          <div className="bg-white rounded-lg shadow-md p-6">
            <GameBoardContainer />
          </div>

          {/* Guess Input */}
          <div className="bg-white rounded-lg shadow-md p-6">
            <GuessInputContainer />
          </div>
        </div>

        {/* Right Column - Guess History */}
        <div className="bg-white rounded-lg shadow-md p-4">
          <GuessHistoryContainer />
        </div>
      </div>

      {/* Tablet Layout - 2 columns */}
      <div className="hidden md:grid md:grid-cols-[250px_1fr] lg:hidden gap-4 max-w-5xl mx-auto">
        {/* Left Column - Players */}
        <div className="bg-white rounded-lg shadow-md p-4">
          <PlayerListContainer />
        </div>

        {/* Right Column - Game + History */}
        <div className="flex flex-col gap-4">
          {/* Timer */}
          <div className="bg-white rounded-lg shadow-md p-4">
            <CountdownTimerContainer />
          </div>

          {/* Game Board */}
          <div className="bg-white rounded-lg shadow-md p-6">
            <GameBoardContainer />
          </div>

          {/* Guess Input */}
          <div className="bg-white rounded-lg shadow-md p-6">
            <GuessInputContainer />
          </div>

          {/* Guess History */}
          <div className="bg-white rounded-lg shadow-md p-4">
            <GuessHistoryContainer />
          </div>
        </div>
      </div>

      {/* Mobile Layout - Stacked with tabs */}
      <div className="md:hidden flex flex-col gap-4 max-w-md mx-auto">
        {/* Timer */}
        <div className="bg-white rounded-lg shadow-md p-3">
          <CountdownTimerContainer />
        </div>

        {/* Game Board */}
        <div className="bg-white rounded-lg shadow-md p-4">
          <GameBoardContainer />
        </div>

        {/* Guess Input */}
        <div className="bg-white rounded-lg shadow-md p-4">
          <GuessInputContainer />
        </div>

        {/* Tabs for Players/History */}
        <div className="bg-white rounded-lg shadow-md">
          <div className="flex border-b">
            <button
              onClick={() => setActiveTab('players')}
              className={`flex-1 py-3 px-4 font-semibold transition-colors ${
                activeTab === 'players'
                  ? 'text-blue-600 border-b-2 border-blue-600'
                  : 'text-gray-600 hover:text-gray-800'
              }`}
              data-testid="players-tab"
            >
              Players
            </button>
            <button
              onClick={() => setActiveTab('history')}
              className={`flex-1 py-3 px-4 font-semibold transition-colors ${
                activeTab === 'history'
                  ? 'text-blue-600 border-b-2 border-blue-600'
                  : 'text-gray-600 hover:text-gray-800'
              }`}
              data-testid="history-tab"
            >
              My Guesses
            </button>
          </div>
          <div className="p-4">
            {activeTab === 'players' ? (
              <PlayerListContainer />
            ) : (
              <GuessHistoryContainer />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};