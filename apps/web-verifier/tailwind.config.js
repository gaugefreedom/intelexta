/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      // If you had custom 'brand' colors before, you can keep them, 
      // but the new code uses standard 'emerald' and 'slate'.
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [
    require("tailwindcss-animate"), // <--- ADD THIS
  ],
}