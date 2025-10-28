/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          300: '#7dd3fc', // sky-300
          400: '#38bdf8', // sky-400
        },
      },
    },
  },
  plugins: [],
};
