import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "@solidjs/start/config";

export default defineConfig({
    ssr: true,
    server: {
        prerender: {
            routes: ["/"],
        },
    },
    vite: {
        plugins: [tailwindcss()],
    },
});
