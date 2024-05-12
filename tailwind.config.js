/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      fontFamily: {
        'sans': ['Inter', 'sans-serif'],
      },
      colors: {
        'primary-fg': '#fff',
        'gray-modern': {
          100: '#EEF2F6',
          200: '#E3E8EF',
          500: '#697586',
          900: '#121926',
        },
        'blue-dark': {
          500: '#2970FF',
          600: '#155EEF',
        },
        'gray-light': {
          400: '#98A2B3'
        }
      }
    },
  },
  plugins: [],
}

