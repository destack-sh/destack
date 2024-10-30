import colors from "tailwindcss/colors";
import aspectRatio from '@tailwindcss/aspect-ratio';
import typography from '@tailwindcss/typography';
import containerQueries from '@tailwindcss/container-queries';

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
      sans: [
        "IBM Plex Sans",
        "ui-sans-serif",
        "-apple-system",
        "BlinkMacSystemFont",
        "Segoe UI",
        "Helvetica",
        "Arial",
        "sans-serif",
      ],
      serif: ["IBM Plex Serif", "Georgia", "Cambria", "Times New Roman", "Times", "serif"],
      mono: ["IBM Plex Mono", "Droid Sans Mono", "monospace"],
    },
    extend: {
      zIndex: {
        60: "60",
        70: "70",
        80: "80",
        90: "90",
        100: "100",
      },
      blur: {
        xs: "2px",
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
      textUnderlineOffset: {
        3: "3px",
      },
      colors: {
        primary: colors.amber,
        secondary: colors.sky,
        accent: colors.orange,
        gray: colors.stone,
        canvas: colors.stone,
        success: colors.lime,
        hint: colors.sky,
        warning: colors.yellow,
        danger: colors.red,
      },
      transitionProperty: {
        // extend 'colors' to include opacity
        colors: "color, background-color, border-color, text-decoration-color, fill, stroke, opacity",
      },
      transitionDuration: {
        0: "0ms",
        2000: "2000ms",
      },
    },
  },
  variants: {
    extend: {
      backgroundColor: ["not-focus", "not-focus-within"],
      borderColor: ["not-focus", "not-focus-within"],
      textColor: ["not-focus", "not-focus-within"],
    },
  },
  plugins: [
    aspectRatio,
    typography,
    containerQueries,
    
    // not-focus variants
    function ({ addVariant, e }) {
      addVariant("not-focus", ({ modifySelectors, separator }) => {
        modifySelectors(({ className }) => {
          return `.${e(`not-focus${separator}${className}`)}:not(:focus)`;
        });
      });
      addVariant("not-focus-within", ({ modifySelectors, separator }) => {
        modifySelectors(({ className }) => {
          return `.${e(`not-focus-within${separator}${className}`)}:not(:focus-within)`;
        });
      });
    },
  ],
};
