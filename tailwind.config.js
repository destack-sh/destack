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
