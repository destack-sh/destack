import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "@solidjs/start/config";

import { contentPlugin } from "./build/content";
import { prerenderRoutes } from "./src/generated/prerender-routes";

export default defineConfig({
    ssr: true,
    server: {
        prerender: {
            routes: [...prerenderRoutes],
        },
    },
    vite: {
        plugins: [contentPlugin(), tailwindcss()],
    },
});
