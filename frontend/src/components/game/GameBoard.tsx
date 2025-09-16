import React from "react";
import type { GuessResult, LetterStatus } from "../../types/generated";

interface GameBoardProps {
  guesses: GuessResult[];
  wordLength: number;
  maxGuesses?: number;
  currentGuess?: string;
  isCurrentPlayer?: boolean;
}

interface LetterTileProps {
  letter: string;
  status: LetterStatus | "empty" | "pending";
}

const LetterTile: React.FC<LetterTileProps> = ({ letter, status }) => {
  const getStatusClasses = () => {
    switch (status) {
      case "Correct":
        return "bg-blue-500 text-white border-blue-600";
      case "Present":
        return "bg-orange-500 text-white border-orange-600";
      case "Absent":
        return "bg-gray-500 text-white border-gray-600";
      case "pending":
        return "bg-white text-gray-900 border-gray-400 animate-pulse";
      case "empty":
      default:
        return "bg-white border-gray-300";
    }
  };

  return (
    <div
      className={`
        w-12 h-12 md:w-14 md:h-14 
        border-2 rounded-lg
        flex items-center justify-center
        font-bold text-xl md:text-2xl uppercase
        transition-all duration-300
        ${getStatusClasses()}
      `}
      data-testid={`letter-tile-${status}`}
    >
      {letter}
    </div>
  );
};

export const GameBoard: React.FC<GameBoardProps> = ({
  guesses,
  wordLength,
  maxGuesses = 6,
  currentGuess = "",
  isCurrentPlayer = false,
}) => {
  // Create rows for the board
  const rows: Array<{
    letters: Array<{
      letter: string;
      status: LetterStatus | "empty" | "pending";
    }>;
  }> = [];

  // Add completed guesses
  guesses.forEach((guess) => {
    const letters = guess.letters.map((letterResult) => ({
      letter: letterResult.letter,
      status: letterResult.status,
    }));
    rows.push({ letters });
  });

  // Add current guess if player is actively guessing
  if (isCurrentPlayer && currentGuess && rows.length < maxGuesses) {
    const currentLetters: {
      letter: string;
      status: LetterStatus | "empty" | "pending";
    }[] = currentGuess.split("").map((letter) => ({
      letter,
      status: "pending" as const,
    }));

    // Pad with empty tiles
    while (currentLetters.length < wordLength) {
      currentLetters.push({ letter: "", status: "empty" as const });
    }

    rows.push({ letters: currentLetters });
  }

  // Fill remaining rows with empty tiles
  while (rows.length < maxGuesses) {
    const emptyLetters = Array(wordLength)
      .fill(null)
      .map(() => ({
        letter: "",
        status: "empty" as const,
      }));
    rows.push({ letters: emptyLetters });
  }

  return (
    <div className="flex flex-col gap-2" data-testid="game-board">
      <h2 className="text-xl md:text-2xl font-bold text-center mb-2">
        Collaborative Board
      </h2>
      <div className="flex flex-col gap-2">
        {rows.map((row, rowIndex) => (
          <div
            key={rowIndex}
            className="flex gap-2 justify-center"
            data-testid={`game-row-${rowIndex}`}
          >
            {row.letters.map((tile, colIndex) => (
              <LetterTile
                key={`${rowIndex}-${colIndex}`}
                letter={tile.letter}
                status={tile.status}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
};

// Container component that connects to store
export const GameBoardContainer: React.FC = () => {
  const { gameState, currentGuess } = useGameStore();

  if (!gameState) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-gray-500">Waiting for game to start...</p>
      </div>
    );
  }

  return (
    <GameBoard
      guesses={gameState.official_board}
      wordLength={gameState.word_length}
      currentGuess={currentGuess}
      isCurrentPlayer={gameState.current_phase === "Guessing"}
    />
  );
};

// Import store (will be at top of file in final version)
import { useGameStore } from "../../store/gameStore";
