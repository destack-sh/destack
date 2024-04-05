import colors from "tailwindcss/colors";

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    boxShadow: {
      // flat, hard outer shadows
      sm: "0 1px 0 0 rgba(0, 0, 0, 0.05)",
      DEFAULT: "0 1px 0 0 rgba(0, 0, 0, 0.1)",
      md: "0 2px 0 0 rgba(0, 0, 0, 0.1)",
      lg: "0 4px 0 0 rgba(0, 0, 0, 0.1)",
      xl: "0 8px 0 0 rgba(0, 0, 0, 0.1)",
      "2xl": "0 12px 0 0 rgba(0, 0, 0, 0.1)",
      // flat, hard inner shadows
      "inset-sm": "inset 0 1px 0 0 rgba(0, 0, 9, 0.05)",
      inset: "inset 0 1px 0 0 rgba(0, 0, 0, 0.1)",
      "inset-md": "inset 0 2px 0 0 rgba(0, 0, 0, 0.1)",
      "inset-lg": "inset 0 4px 0 0 rgba(0, 0, 0, 0.1)",
      "inset-xl": "inset 0 8px 0 0 rgba(0, 0, 0, 0.1)",
    },
    fontFamily: {
      sans: ["IBM Plex Sans", "ui-sans-serif", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Helvetica", "Arial", "sans-serif"],
      serif: ["IBM Plex Serif", "Georgia", "Cambria", "Times New Roman", "Times", "serif"],
      mono: ["IBM Plex Mono", "Droid Sans Mono", "monospace"],
    },
    extend: {
      zIndex: {
        '60': '60',
        '70': '70',
        '80': '80',
        '90': '90',
        '100': '100',
      },
      screens: {
        "-2xl": { max: "1535px" },
        "-xl": { max: "1279px" },
        "-lg": { max: "1023px" },
        "-md": { max: "767px" },
        "-sm": { max: "639px" },
        "-xs": { max: "479px" },
      },
      containers: {
        "-2xl": { max: "1535px" },
        "-xl": { max: "1279px" },
        "-lg": { max: "1023px" },
        "-md": { max: "767px" },
        "-sm": { max: "639px" },
        "-xs": { max: "479px" },
        "-2xs": { max: "399px" },
        "-3xs": { max: "319px" },
        "-4xs": { max: "239px" },
      },
      colors: {
        primary: colors.amber,
        secondary: colors.sky,
        accent: colors.amber,
        gray: colors.slate,
        canvas: colors.slate,
        success: colors.lime,
        hint: colors.sky,
        warning: colors.yellow,
        danger: colors.rose,
      },
      transitionProperty: {
        // extend 'colors' to include opacity
        'colors': 'color, background-color, border-color, text-decoration-color, fill, stroke, opacity',
      }
    },
  },
  plugins: [],
};
