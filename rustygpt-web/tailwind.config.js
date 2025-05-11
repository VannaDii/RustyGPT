/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{rs,html,js}'],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      animation: {
        'typing-dot': 'typing-dot 1.4s infinite',
      },
      keyframes: {
        'typing-dot': {
          '0%, 60%, 100%': { opacity: 0.4, transform: 'scale(0.8)' },
          '30%': { opacity: 1, transform: 'scale(1)' },
        },
      },
    },
  },
  plugins: [require('@tailwindcss/forms'), require('@tailwindcss/typography'), require('daisyui')],
  daisyui: {
    themes: [
      {
        light: {
          primary: '#10a37f',
          'primary-focus': '#0d8c6d',
          'primary-content': '#ffffff',
          secondary: '#6b46fe',
          'secondary-focus': '#5635d9',
          'secondary-content': '#ffffff',
          accent: '#1fb2a6',
          'accent-focus': '#1a9085',
          'accent-content': '#ffffff',
          'base-100': '#ffffff',
          'base-200': '#f7f7f8',
          'base-300': '#ececf1',
          'base-content': '#343541',
          neutral: '#f7f7f8',
          'neutral-focus': '#e5e5e5',
          'neutral-content': '#343541',
          info: '#3abff8',
          success: '#36d399',
          warning: '#fbbd23',
          error: '#f87272',
        },
        dark: {
          primary: '#10a37f',
          'primary-focus': '#0d8c6d',
          'primary-content': '#ffffff',
          secondary: '#8c84fc',
          'secondary-focus': '#7b73eb',
          'secondary-content': '#ffffff',
          accent: '#1fb2a6',
          'accent-focus': '#1a9085',
          'accent-content': '#ffffff',
          'base-100': '#343541',
          'base-200': '#202123',
          'base-300': '#2a2b32',
          'base-content': '#ececf1',
          neutral: '#343541',
          'neutral-focus': '#404152',
          'neutral-content': '#ececf1',
          info: '#3abff8',
          success: '#36d399',
          warning: '#fbbd23',
          error: '#f87272',
        },
      },
    ],
  },
};
