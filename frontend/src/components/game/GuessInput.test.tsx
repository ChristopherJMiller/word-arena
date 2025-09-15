import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GuessInput } from './GuessInput';

describe('GuessInput', () => {
  const defaultProps = {
    wordLength: 5,
    currentGuess: '',
    isDisabled: false,
    onGuessChange: vi.fn(),
    onSubmit: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders correct number of letter inputs', () => {
    render(<GuessInput {...defaultProps} />);
    const inputs = screen.getAllByTestId(/letter-input-/);
    expect(inputs).toHaveLength(5);
  });

  it('converts input to uppercase', async () => {
    const onGuessChange = vi.fn();
    render(<GuessInput {...defaultProps} onGuessChange={onGuessChange} />);
    
    const firstInput = screen.getByTestId('letter-input-0');
    await userEvent.type(firstInput, 'a');
    
    expect(onGuessChange).toHaveBeenCalledWith('A');
  });

  it('moves focus to next input after typing', async () => {
    render(<GuessInput {...defaultProps} />);
    
    const firstInput = screen.getByTestId('letter-input-0');
    const secondInput = screen.getByTestId('letter-input-1');
    
    await userEvent.type(firstInput, 'A');
    
    // Second input should have focus
    await waitFor(() => {
      expect(document.activeElement).toBe(secondInput);
    });
  });

  it('handles backspace to delete and move to previous input', async () => {
    const onGuessChange = vi.fn();
    render(<GuessInput {...defaultProps} currentGuess="AB" onGuessChange={onGuessChange} />);
    
    const secondInput = screen.getByTestId('letter-input-1');
    secondInput.focus();
    
    // Backspace on filled cell should clear it
    fireEvent.keyDown(secondInput, { key: 'Backspace' });
    expect(onGuessChange).toHaveBeenCalledWith('A');
  });

  it('enables submit button only when word is complete', () => {
    const { rerender } = render(<GuessInput {...defaultProps} currentGuess="HELL" />);
    
    let submitButton = screen.getByTestId('submit-guess-button');
    expect(submitButton).toBeDisabled();
    
    rerender(<GuessInput {...defaultProps} currentGuess="HELLO" />);
    submitButton = screen.getByTestId('submit-guess-button');
    expect(submitButton).not.toBeDisabled();
  });

  it('calls onSubmit when Enter is pressed with complete word', () => {
    const onSubmit = vi.fn();
    render(<GuessInput {...defaultProps} currentGuess="HELLO" onSubmit={onSubmit} />);
    
    const anyInput = screen.getByTestId('letter-input-0');
    fireEvent.keyDown(anyInput, { key: 'Enter' });
    
    expect(onSubmit).toHaveBeenCalledWith('HELLO');
  });

  it('does not submit incomplete word', () => {
    const onSubmit = vi.fn();
    render(<GuessInput {...defaultProps} currentGuess="HEL" onSubmit={onSubmit} />);
    
    const submitButton = screen.getByTestId('submit-guess-button');
    fireEvent.click(submitButton);
    
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('disables all inputs when isDisabled is true', () => {
    render(<GuessInput {...defaultProps} isDisabled={true} />);
    
    const inputs = screen.getAllByTestId(/letter-input-/);
    inputs.forEach(input => {
      expect(input).toBeDisabled();
    });
    
    const submitButton = screen.getByTestId('submit-guess-button');
    expect(submitButton).toBeDisabled();
  });

  it('handles arrow key navigation', async () => {
    render(<GuessInput {...defaultProps} currentGuess="HELLO" />);
    
    const firstInput = screen.getByTestId('letter-input-0');
    const secondInput = screen.getByTestId('letter-input-1');
    
    // Start at first input
    firstInput.focus();
    
    // Arrow right should move to second input
    fireEvent.keyDown(firstInput, { key: 'ArrowRight' });
    
    // Wait for focus to change
    await waitFor(() => {
      expect(document.activeElement).toBe(secondInput);
    });
    
    // Arrow left should move back to first input
    fireEvent.keyDown(secondInput, { key: 'ArrowLeft' });
    
    await waitFor(() => {
      expect(document.activeElement).toBe(firstInput);
    });
  });

  it('only accepts letters as input', async () => {
    const onGuessChange = vi.fn();
    render(<GuessInput {...defaultProps} onGuessChange={onGuessChange} />);
    
    const firstInput = screen.getByTestId('letter-input-0');
    
    // Try to type numbers and special characters
    await userEvent.type(firstInput, '123!@#');
    
    // Should not call onGuessChange for invalid characters
    expect(onGuessChange).not.toHaveBeenCalled();
    
    // Type a valid letter
    await userEvent.type(firstInput, 'A');
    expect(onGuessChange).toHaveBeenCalledWith('A');
  });
});