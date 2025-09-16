import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { GameBoard } from "./GameBoard";
import type { GuessResult } from "../../types/generated";

describe("GameBoard", () => {
  const mockGuesses: GuessResult[] = [
    {
      word: "HELLO",
      player_id: "player1",
      letters: [
        { letter: "H", status: "Absent", position: 0 },
        { letter: "E", status: "Present", position: 1 },
        { letter: "L", status: "Correct", position: 2 },
        { letter: "L", status: "Correct", position: 3 },
        { letter: "O", status: "Absent", position: 4 },
      ],
      points_earned: 4,
      timestamp: "2024-01-01T00:00:00Z",
    },
  ];

  it("renders the game board with correct title", () => {
    render(<GameBoard guesses={[]} wordLength={5} />);
    expect(screen.getByText("Collaborative Board")).toBeInTheDocument();
  });

  it("renders empty board with correct dimensions", () => {
    render(<GameBoard guesses={[]} wordLength={5} maxGuesses={6} />);
    const board = screen.getByTestId("game-board");
    expect(board).toBeInTheDocument();

    // Should have 6 rows
    const rows = screen.getAllByTestId(/game-row-/);
    expect(rows).toHaveLength(6);
  });

  it("displays completed guesses with correct letter statuses", () => {
    render(<GameBoard guesses={mockGuesses} wordLength={5} />);

    // Check for correct status tiles
    const correctTiles = screen.getAllByTestId("letter-tile-Correct");
    expect(correctTiles).toHaveLength(2); // Two L's are correct

    const presentTiles = screen.getAllByTestId("letter-tile-Present");
    expect(presentTiles).toHaveLength(1); // E is present

    const absentTiles = screen.getAllByTestId("letter-tile-Absent");
    expect(absentTiles).toHaveLength(2); // H and O are absent
  });

  it("shows current guess when player is actively guessing", () => {
    render(
      <GameBoard
        guesses={mockGuesses}
        wordLength={5}
        currentGuess="WOR"
        isCurrentPlayer={true}
      />,
    );

    // Should show pending tiles for current guess
    const pendingTiles = screen.getAllByTestId("letter-tile-pending");
    expect(pendingTiles.length).toBeGreaterThan(0);
  });

  it("fills remaining rows with empty tiles", () => {
    render(<GameBoard guesses={mockGuesses} wordLength={5} maxGuesses={6} />);

    // With 1 guess and 6 max, should have empty tiles
    const emptyTiles = screen.getAllByTestId("letter-tile-empty");
    expect(emptyTiles.length).toBeGreaterThan(0);
  });

  it("handles different word lengths correctly", () => {
    render(<GameBoard guesses={[]} wordLength={7} maxGuesses={6} />);

    const firstRow = screen.getByTestId("game-row-0");
    const tiles = firstRow.querySelectorAll('[data-testid^="letter-tile-"]');
    expect(tiles).toHaveLength(7);
  });
});
