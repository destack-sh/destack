import { fileURLToPath, URL } from "url";

import vue from "@vitejs/plugin-vue";
import { defineConfig, loadEnv } from "vite";

// see https://vitejs.dev/config/
export default defineConfig(({ command, mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  return {
    define: {
      __APP_ENV__: env.APP_ENV,
    },
    plugins: [vue()],
    root: "./frontend",
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./frontend/src", import.meta.url)),
      },
    },
  };
});
