import colors from "tailwindcss/colors";

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    fontFamily: {
      sans: ["ui-sans-serif", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Helvetica", "Arial", "sans-serif"],
      serif: ["IBM Plex Serif", "Georgia", "Cambria", "Times New Roman", "Times", "serif"],
      mono: ["Droid Sans Mono", "monospace"],
    },
    extend: {
      colors: {
        neutral: colors.black,
        primary: colors.orange,
        secondary: colors.teal,
        accent: colors.orange,
        gray: colors.zinc,
        success: colors.green,
        hint: colors.sky,
        warning: colors.yellow,
        danger: colors.red,
      },
    },
  },
  plugins: [],
};
