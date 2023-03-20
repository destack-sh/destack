const colors = require("tailwindcss/colors");

module.exports = {
  content: ["./frontend/index.html", "./frontend/src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    fontFamily: {
      sans: ["IBM Plex Sans", "system-ui", "-apple-system", "BlinkMacSystemFont", "sans-serif"],
      mono: ["Droid Sans Mono", "monospace"],
    },
    extend: {
      colors: {
        sky: colors.sky,
        teal: colors.teal,
        rose: colors.rose,
        gray: colors.zinc,
      },
    },
  },
  plugins: [
    require("@tailwindcss/typography"),
    require("@tailwindcss/forms"),
    require("@tailwindcss/line-clamp"),
    require("@tailwindcss/aspect-ratio"),
    require("@headlessui/tailwindcss"),
  ],
  darkMode: "class",
};
