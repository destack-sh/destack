/* eslint-env node */
require("@rushstack/eslint-patch/modern-module-resolution");

module.exports = {
  root: true,
  extends: [
    "plugin:vue/vue3-essential",
    "eslint:recommended",
    "@vue/eslint-config-typescript/recommended",
    "@vue/eslint-config-prettier",
  ],
  env: {
    node: true,
    "vue/setup-compiler-macros": true,
  },
  ignorePatterns: ["node_modules/*", ".eslintrc.js", "frontend/dist/*", "frontend/src/gql/*"],
  overrides: [
    {
      files: ["cypress/integration/**.spec.{js,ts,jsx,tsx}"],
      extends: ["plugin:cypress/recommended"],
    },
    // TODO @Ops: use graphql-eslint (currently broken with prettier and Vue SFC files)
    // {
    //   files: ["*.js", "*.ts", "*.vue"],
    //   processor: "@graphql-eslint/graphql",
    //   plugins: ["@graphql-eslint"],
    // },
    {
      files: ["*.graphql"],
      parser: "@graphql-eslint/eslint-plugin",
      plugins: ["@graphql-eslint"],
      rules: {
        "@graphql-eslint/known-type-names": "error",
      },
    },
    {
      files: ["*.ts", "*.tsx"],
      rules: {
        "@typescript-eslint/no-empty-function": "off",
      },
    },
  ],
  // allow 'Symbol' to be used as a type
  rules: {
    "@typescript-eslint/ban-types": [
      "error",
      {
        types: {
          Symbol: false,
        },
        extendDefaults: true,
      },
    ],
    "vue/multi-word-component-names": "off",
  },
};
