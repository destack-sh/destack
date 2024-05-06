/* eslint-env node */
require("@rushstack/eslint-patch/modern-module-resolution");

module.exports = {
  root: true,
  extends: [
    "plugin:vue/vue3-recommended",
    "eslint:recommended",
    "@vue/eslint-config-typescript",
    "@vue/eslint-config-prettier/skip-formatting",
  ],
  parserOptions: {
    ecmaVersion: "latest",
  },
  overrides: [
    {
      files: ["*.vue"],
      rules: {
        // we require single root components for our View components
        "vue/no-multiple-template-root": "error",
        "vue/multi-word-component-names": "off",
        "vue/no-v-html": "off",
        "vue/match-component-file-name": "error",
        "vue/no-root-v-if": "error",
      },
    },
    {
      files: ["*.ts", "*.tsx", "*.js", "*.jsx", "*.vue"],
      rules: {
        "@typescript-eslint/no-unused-expressions": "error",
        "@typescript-eslint/no-unused-vars": "off",
        "prefer-const": "warn",
      },
    },
  ],
};
