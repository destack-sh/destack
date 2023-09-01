import { fileURLToPath, URL } from "url";

import vue from "@vitejs/plugin-vue";
import { defineConfig, loadEnv } from "vite";
import monacoEditorPlugin from "vite-plugin-monaco-editor";
import { watch } from "fs";
import type { Plugin } from "vite";

function reloadOnVersionChange(): Plugin {
  return {
    name: "reload-on-version-change",
    apply: "serve", // only use this plugin in serve mode
    configureServer(server: any) {
      // Watch for changes in package.json file
      watch("package.json", (eventType, filename) => {
        if (filename) {
          server.ws.send({
            type: "full-reload",
            path: "*",
          });
        }
      });
    },
  };
}

// see https://vitejs.dev/config/
export default defineConfig(({ command, mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  return {
    define: {
      VITE_APP_ENV: env.APP_ENV,
      VITE_APP_VERSION: JSON.stringify(process.env.npm_package_version),
      VITE_APP_GIT_COMMIT: JSON.stringify(process.env.GIT_COMMIT),
      // I'm not sure why process.env is required suddenly, but it fixes an error in babel (?).
      "process.env": {},
    },
    plugins: [vue(), monacoEditorPlugin({ languageWorkers: ["editorWorkerService", "json"] }), reloadOnVersionChange()],
    root: "./frontend",
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./frontend/src", import.meta.url)),
      },
    },
    build: {
      target: "esnext",
    },
    server: {
      host: "127.0.0.1",
      port: 3000,
    },
  };
});
