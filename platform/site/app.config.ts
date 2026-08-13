import stylex from "@stylexjs/unplugin";
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
        plugins: [
            stylex.vite({
                unstable_moduleResolution: {
                    rootDir: import.meta.dirname,
                    type: "commonJS",
                },
                useCSSLayers: true,
            }),
            contentPlugin(import.meta.dirname),
        ],
    },
});
