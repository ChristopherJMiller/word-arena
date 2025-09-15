/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Word Arena game colors (colorblind friendly)
        'correct': '#0066cc', // Blue for correct letters
        'present': '#ff8800', // Orange for present letters
        'absent': '#666666',  // Gray for absent letters
      }
    },
  },
  plugins: [],
}