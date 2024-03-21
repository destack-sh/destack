/* eslint-env node */
require("@rushstack/eslint-patch/modern-module-resolution");

module.exports = {
  root: true,
  extends: [
    "plugin:vue/vue3-essential",
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
      },
    },
    {
      files: ["*.ts", "*.tsx", "*.js", "*.jsx", "*.vue"],
      rules: {
        "@typescript-eslint/no-unused-vars": "off",
      },
    },
  ],
};
