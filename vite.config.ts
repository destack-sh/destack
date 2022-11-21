import { fileURLToPath, URL } from "url";

import vue from "@vitejs/plugin-vue";
import { defineConfig, loadEnv } from "vite";
import monacoEditorPlugin from "vite-plugin-monaco-editor";

// see https://vitejs.dev/config/
export default defineConfig(({ command, mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  return {
    define: {
      __APP_ENV__: env.APP_ENV,
      __APP_VERSION__: env.APP_VERSION,
    },
    plugins: [vue(), monacoEditorPlugin({ languageWorkers: ["editorWorkerService", "json"] })],
    root: "./frontend",
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./frontend/src", import.meta.url)),
      },
    },
    build: {
      target: "esnext",
    },
  };
});
