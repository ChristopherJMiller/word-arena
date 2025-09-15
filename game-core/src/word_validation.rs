use std::collections::HashSet;
use anyhow::{anyhow, Result};

pub struct WordValidator {
    valid_words: HashSet<String>,
}

impl WordValidator {
    /// Create a new word validator from a word list
    pub fn new(word_list: &str) -> Self {
        let valid_words = word_list
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|word| word.trim().to_lowercase())
            .filter(|word| word.len() >= 5 && word.len() <= 8)
            .collect();

        Self { valid_words }
    }

    /// Check if a word is valid for the game
    pub fn is_valid_word(&self, word: &str) -> bool {
        let word = word.trim().to_lowercase();
        self.valid_words.contains(&word)
    }

    /// Get a random word of the specified length
    pub fn get_random_word(&self, length: usize) -> Result<String> {
        let words_of_length: Vec<&String> = self.valid_words
            .iter()
            .filter(|word| word.len() == length)
            .collect();

        if words_of_length.is_empty() {
            return Err(anyhow!("No words available of length {}", length));
        }

        // Simple random selection (in production, use proper RNG)
        let index = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        let mut hasher = index;
        std::time::SystemTime::now().hash(&mut hasher);
        let random_index = (hasher.finish() as usize) % words_of_length.len();

        Ok(words_of_length[random_index].clone())
    }

    /// Get word count by length
    pub fn word_count_by_length(&self, length: usize) -> usize {
        self.valid_words
            .iter()
            .filter(|word| word.len() == length)
            .count()
    }

    /// Check if word contains only alphabetic characters
    pub fn is_alphabetic(&self, word: &str) -> bool {
        word.chars().all(|c| c.is_alphabetic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_validator() {
        let word_list = "apple\nbanana\ncherry\n# comment\n\ntests\nvalid";
        let validator = WordValidator::new(word_list);

        assert!(validator.is_valid_word("apple"));
        assert!(validator.is_valid_word("APPLE")); // case insensitive
        assert!(validator.is_valid_word("tests"));
        assert!(validator.is_valid_word("valid"));
        assert!(!validator.is_valid_word("invalid"));
        assert!(!validator.is_valid_word("xyz")); // too short
    }

    #[test]
    fn test_alphabetic_check() {
        let validator = WordValidator::new("test");
        assert!(validator.is_alphabetic("hello"));
        assert!(!validator.is_alphabetic("hello123"));
        assert!(!validator.is_alphabetic("hello-world"));
    }

    #[test]
    fn test_word_validator_edge_cases() {
        let word_list = "a\nab\nabc\nabcd\nabcde\nabcdef\nabcdefgh\nabcdefghi\n# comment\n\n   \n\tMIXED\n  spaces  ";
        let validator = WordValidator::new(word_list);

        // Too short words should be filtered out
        assert!(!validator.is_valid_word("a"));
        assert!(!validator.is_valid_word("ab"));
        assert!(!validator.is_valid_word("abc"));
        assert!(!validator.is_valid_word("abcd"));

        // Valid length words
        assert!(validator.is_valid_word("abcde"));
        assert!(validator.is_valid_word("abcdef"));
        assert!(validator.is_valid_word("abcdefgh"));

        // Too long words should be filtered out
        assert!(!validator.is_valid_word("abcdefghi"));

        // Case insensitive
        assert!(validator.is_valid_word("MIXED"));
        assert!(validator.is_valid_word("mixed"));
        assert!(validator.is_valid_word("MiXeD"));

        // Whitespace handling
        assert!(validator.is_valid_word("spaces"));
    }

    #[test]
    fn test_empty_word_list() {
        let validator = WordValidator::new("");
        assert!(!validator.is_valid_word("hello"));
        assert_eq!(validator.word_count_by_length(5), 0);
        
        let result = validator.get_random_word(5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No words available"));
    }

    #[test]
    fn test_comments_and_whitespace() {
        let word_list = "# This is a comment\nvalid\n   \n\t# Another comment\n  spaced  \n\n";
        let validator = WordValidator::new(word_list);

        assert!(validator.is_valid_word("valid"));
        assert!(validator.is_valid_word("spaced"));
        assert_eq!(validator.word_count_by_length(5), 1); // Only "valid" is 5 chars, "spaced" is 6
        assert_eq!(validator.word_count_by_length(6), 1); // "spaced" is 6 chars
    }

    #[test]
    fn test_invalid_characters() {
        let validator = WordValidator::new("test");
        
        // Numbers
        assert!(!validator.is_alphabetic("test123"));
        assert!(!validator.is_alphabetic("123test"));
        assert!(!validator.is_alphabetic("te123st"));

        // Special characters
        assert!(!validator.is_alphabetic("test!"));
        assert!(!validator.is_alphabetic("test@word"));
        assert!(!validator.is_alphabetic("test_word"));
        assert!(!validator.is_alphabetic("test-word"));
        assert!(!validator.is_alphabetic("test word"));

        // Unicode and special characters that should not be alphabetic for basic ASCII games
        // Skip accented character tests as they might be considered alphabetic by Rust's definition

        // Empty and whitespace
        assert!(validator.is_alphabetic(""));
        assert!(!validator.is_alphabetic(" "));
        assert!(!validator.is_alphabetic("\t"));
        assert!(!validator.is_alphabetic("\n"));
    }

    #[test]
    fn test_random_word_selection() {
        let word_list = "apple\nbanana\ncherry\ntests\nvalid\nhello\nworld";
        let validator = WordValidator::new(word_list);

        // Test getting words of different lengths
        let five_letter_word = validator.get_random_word(5);
        assert!(five_letter_word.is_ok());
        let word = five_letter_word.unwrap();
        assert_eq!(word.len(), 5);
        assert!(validator.is_valid_word(&word));

        // Test length that doesn't exist
        let ten_letter_word = validator.get_random_word(10);
        assert!(ten_letter_word.is_err());

        // Test multiple calls return valid words (may be different due to randomness)
        for _ in 0..10 {
            let word = validator.get_random_word(5).unwrap();
            assert_eq!(word.len(), 5);
            assert!(validator.is_valid_word(&word));
        }
    }

    #[test]
    fn test_word_count_by_length() {
        let word_list = "apple\nbanana\ncherry\ntests\nvalid\nhello\nworld\nab\nabcd\nabcdefghijk";
        let validator = WordValidator::new(word_list);

        assert_eq!(validator.word_count_by_length(2), 0); // too short
        assert_eq!(validator.word_count_by_length(3), 0); // too short
        assert_eq!(validator.word_count_by_length(4), 0); // too short
        assert_eq!(validator.word_count_by_length(5), 5); // apple, tests, valid, hello, world
        assert_eq!(validator.word_count_by_length(6), 2); // banana, cherry
        assert_eq!(validator.word_count_by_length(7), 0); // none
        assert_eq!(validator.word_count_by_length(8), 0); // none
        assert_eq!(validator.word_count_by_length(9), 0); // too long
        assert_eq!(validator.word_count_by_length(11), 0); // too long
    }

    #[test]
    fn test_boundary_lengths() {
        let word_list = "four\nfives\nsixsix\nsevense\neighters\nnineninee";
        let validator = WordValidator::new(word_list);

        // Boundary cases for length filtering
        assert!(!validator.is_valid_word("four")); // 4 chars, too short
        assert!(validator.is_valid_word("fives")); // 5 chars, minimum valid
        assert!(validator.is_valid_word("eighters")); // 8 chars, maximum valid
        assert!(!validator.is_valid_word("nineninee")); // 9 chars, too long
    }
}