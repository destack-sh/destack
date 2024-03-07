import colors from "tailwindcss/colors";

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    boxShadow: {
      // flat, hard shadows
      sm: '0 1px 0 0 rgba(0, 0, 0, 0.05)',
      DEFAULT: '0 1px 0 0 rgba(0, 0, 0, 0.1)',
      md: '0 2px 0 0 rgba(0, 0, 0, 0.1)',
      lg: '0 4px 0 0 rgba(0, 0, 0, 0.1)',
      xl: '0 8px 0 0 rgba(0, 0, 0, 0.1)',
      '2xl': '0 12px 0 0 rgba(0, 0, 0, 0.1)',
    },
    fontFamily: {
      sans: ["ui-sans-serif", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Helvetica", "Arial", "sans-serif"],
      serif: ["IBM Plex Serif", "Georgia", "Cambria", "Times New Roman", "Times", "serif"],
      mono: ["Droid Sans Mono", "monospace"],
    },
    extend: {
      screens: {
        '-2xl': {'max': '1535px'},
        '-xl': {'max': '1279px'},
        '-lg': {'max': '1023px'},
        '-md': {'max': '767px'},
        '-sm': {'max': '639px'},
      },
      colors: {
        primary: colors.amber,
        secondary: colors.emerald,
        accent: colors.amber,
        gray: colors.slate,
        success: colors.green,
        hint: colors.sky,
        warning: colors.yellow,
        danger: colors.red,
      },
    },
  },
  plugins: [],
};
