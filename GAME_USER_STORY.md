# MVP User Story

Users enter the website and are met with a leaderboard and asking them to log in to play

Users log in using a microsoft login for SSO (ensure this can be extended later, discord is in mind)

Users can queue into a match, and are placed into a match of 2 - 16 people

The game plays as follows:

- Users go in rounds solving a collective "Wordle" puzzle. The words will be between 5 and 8 letters long.
- After a countdown, all players at the same time must try to enter a valid word for a guess.
- Wordle rules follow: any letters that exist in the word are orange, letters in the correct position are then blue (instead of yellow-green, orange-blue is more colorblind friendly).
- The player with the most correct guess (prioritizing blue letters, then orange letters) wins the round and their guess is added to the official collaborative board that all players see.
- Non-winning players see their guess added to their personal guess history with the calculated point value, but no detailed feedback breakdown.
- The winning player must guess again (it cannot be a word already guessed), followed by another countdown where all players guess simultaneously.
- This continues until the word is guessed. The word can be guessed during either the group phase or individual winner phase.
- The official board shows the shared puzzle progress with only winning guesses, while each player has a personal sidebar showing their guess history and points earned.
- A match is comprised of a series of rounds, the first player to hit a certain point threshold wins.
- Points are awarded as follows:
  - New orange letter: 1 point
  - New blue letter: 2 points
  - Guessing the word: 5 points
- Points are awarded as we go, but win conditions are only done at the end of a word. This means if two players are above the threshold to win, it's given to the player with the higher points (allowing players to clutch wins last second).

Leaderboards are kept for players of total points and total wins.
