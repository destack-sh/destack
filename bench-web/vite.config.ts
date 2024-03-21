import { fileURLToPath, URL } from "node:url";

import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import vueJsx from "@vitejs/plugin-vue-jsx";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [vue(), vueJsx()],
  resolve: {
    // Resolve grpc-web/Vite issue (see https://github.com/grpc/grpc-web/issues/1242#issuecomment-1816249928)
    preserveSymlinks: true,
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    chunkSizeWarningLimit: 4096,
    minify: mode !== "unminified",
  },
}));
