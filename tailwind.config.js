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
      boxShadow: {
        outline: "4 4 3 3px rgba(0 0 0 / 0.05)",
      },
      typography: (theme) => ({
        DEFAULT: {
          css: {
            color: theme("colors.gray.500"),
          },
        },
      }),
    },
  },
  plugins: [
    require("@tailwindcss/typography"),
    require("@tailwindcss/forms"),
    require("@tailwindcss/line-clamp"),
    require("@tailwindcss/aspect-ratio"),
    require("@headlessui/tailwindcss"),
  ],
};
