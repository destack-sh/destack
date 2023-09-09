const colors = require("tailwindcss/colors");

module.exports = {
  content: ["./frontend/index.html", "./frontend/src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    fontFamily: {
      sans: ["ui-sans-serif", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Helvetica", "Arial", "sans-serif"],
      serif: ["IBM Plex Serif", "Georgia", "Cambria", "Times New Roman", "Times", "serif"],
      mono: ["Droid Sans Mono", "monospace"],
    },
    extend: {
      colors: {
        sky: colors.sky,
        teal: colors.teal,
        rose: colors.rose,
        gray: colors.zinc,
      },
      blur: {
        xs: "2px",
      },
      typography: (theme) => ({
        DEFAULT: {
          // text-gray-900 for everything
          css: {
            color: theme("colors.gray.900"),
            a: {
              color: theme("colors.gray.900"),
            },
          },
        },
      }),
      animation: {
        ["fadein-500"]: "fadein-75 0.5s forwards",
        ["fadein-1000"]: "fadein-75 1.0s forwards",
        ["fadein-1500"]: "fadein-75 1.5s forwards",
        ["fadein-2000"]: "fadein-75 2.0s forwards",
      },
      keyframes: {
        ["fadein-75"]: {
          "0%": {
            opacity: 0,
          },
          "75%": {
            opacity: 0,
          },
          "100%": {
            opacity: 1,
          },
        },
      },
    },
  },
  plugins: [
    require("@tailwindcss/typography"),
    require("@tailwindcss/forms"),
    require("@tailwindcss/aspect-ratio"),
    require("@headlessui/tailwindcss"),
  ],
  darkMode: "class",
};
