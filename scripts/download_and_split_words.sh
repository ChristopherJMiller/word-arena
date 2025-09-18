#!/usr/bin/env bash

# Script to download a large English word list and split it by word length
# Downloads from dwyl/english-words repository and creates separate files for 5-7 letter words

set -e  # Exit on any error

# Configuration
WORDS_URL="https://raw.githubusercontent.com/dwyl/english-words/refs/heads/master/words_alpha.txt"
OUTPUT_DIR="${1:-./word_lists}"
TEMP_FILE="/tmp/words_alpha_download.txt"

echo "🔽 Downloading word list from GitHub..."
echo "URL: $WORDS_URL"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Download the word list
if command -v curl >/dev/null 2>&1; then
    curl -s -L "$WORDS_URL" -o "$TEMP_FILE"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$WORDS_URL" -O "$TEMP_FILE"
else
    echo "❌ Error: Neither curl nor wget is available"
    exit 1
fi

# Check if download was successful
if [ ! -f "$TEMP_FILE" ] || [ ! -s "$TEMP_FILE" ]; then
    echo "❌ Error: Failed to download word list"
    exit 1
fi

echo "✅ Download complete. Processing words..."

# Count total words for progress reporting
TOTAL_WORDS=$(wc -l < "$TEMP_FILE")
echo "📊 Total words in source: $TOTAL_WORDS"

# Process and split words by length
echo "🔄 Splitting words by length (5-7 letters)..."

# Create header comment for each file
HEADER_COMMENT="# English words from dwyl/english-words repository
# Filtered for Word Arena game (5-7 letters, alphabetic only)
# Source: $WORDS_URL
# Generated on: $(date)
"

# Split words by length (5, 6, 7 letters) and filter alphabetic only
awk '
BEGIN {
    header = "# English words from dwyl/english-words repository\n# Filtered for Word Arena game (5-7 letters, alphabetic only)\n# Source: '"$WORDS_URL"'\n# Generated on: '"$(date)"'\n"
}
{
    # Convert to lowercase and remove any whitespace
    word = tolower($0)
    gsub(/[[:space:]]/, "", word)
    
    # Check if word contains only alphabetic characters
    if (word ~ /^[a-z]+$/) {
        len = length(word)
        if (len == 5) {
            words_5[word] = 1
        } else if (len == 6) {
            words_6[word] = 1
        } else if (len == 7) {
            words_7[word] = 1
        }
    }
}
END {
    # Write 5-letter words
    print header > "'"$OUTPUT_DIR"'/words_5_letters.txt"
    for (word in words_5) {
        print word >> "'"$OUTPUT_DIR"'/words_5_letters.txt"
    }
    close("'"$OUTPUT_DIR"'/words_5_letters.txt")
    print "📝 5-letter words:", length(words_5)
    
    # Write 6-letter words
    print header > "'"$OUTPUT_DIR"'/words_6_letters.txt"
    for (word in words_6) {
        print word >> "'"$OUTPUT_DIR"'/words_6_letters.txt"
    }
    close("'"$OUTPUT_DIR"'/words_6_letters.txt")
    print "📝 6-letter words:", length(words_6)
    
    # Write 7-letter words
    print header > "'"$OUTPUT_DIR"'/words_7_letters.txt"
    for (word in words_7) {
        print word >> "'"$OUTPUT_DIR"'/words_7_letters.txt"
    }
    close("'"$OUTPUT_DIR"'/words_7_letters.txt")
    print "📝 7-letter words:", length(words_7)
    
    total_filtered = length(words_5) + length(words_6) + length(words_7)
    print "🎯 Total filtered words:", total_filtered
}
' "$TEMP_FILE"


# Clean up temporary file
rm "$TEMP_FILE"

echo ""
echo "✅ Word processing complete!"
echo "📁 Output directory: $OUTPUT_DIR"
echo "📋 Files created:"
echo "   - words_5_letters.txt (5-letter words)"
echo "   - words_6_letters.txt (6-letter words)" 
echo "   - words_7_letters.txt (7-letter words)"
echo ""
echo "🚀 To use with Word Arena server:"
echo "   export WORDS_DIRECTORY=\"$(realpath "$OUTPUT_DIR")\""
echo "   npm run dev:backend"