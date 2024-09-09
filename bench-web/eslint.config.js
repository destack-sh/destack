import js from "@eslint/js";
import { includeIgnoreFile } from "@eslint/compat";
import tseslint from "typescript-eslint";
import eslintConfigPrettier from "eslint-config-prettier";
import vue from "eslint-plugin-vue";
import path from "path";
import * as vueParser from "vue-eslint-parser";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const gitignorePath = path.join(__dirname, ".gitignore");

export default [
  includeIgnoreFile(gitignorePath),
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{js,ts,tsx,vue}"],
    ignores: [
      "dist/**/*.js",
      "src/proto/wire/**/*.ts",
      "env.d.ts",
      "postcss.config.js",
      "eslint.config.js",
      "tailwind.config.js",
      "vite.config.ts",
      "vitest.config.ts",
    ],
    plugins: {
      "@typescript-eslint": tseslint.plugin,
      vue: vue,
    },
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".vue"],
        ecmaVersion: "latest",
        sourceType: "module",
        project: "./tsconfig.json",
      },
      globals: {
        document: true,
        window: true,
        navigator: true,
        setTimeout: true,
        clearTimeout: true,
      },
    },
    rules: {
      ...vue.configs["vue3-recommended"].rules,
      "@typescript-eslint/no-unused-expressions": "error",
      "@typescript-eslint/no-unused-vars": "off",
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-non-null-asserted-optional-chain": "off",
      "@typescript-eslint/no-this-alias": "off",
      "prefer-const": "warn",
      "no-console": "warn",
      "vue/no-multiple-template-root": "error",
      "vue/multi-word-component-names": "off",
      "vue/no-v-html": "off",
      "vue/match-component-file-name": "error",
      "vue/no-root-v-if": "error",
    },
  },
  eslintConfigPrettier,
];
