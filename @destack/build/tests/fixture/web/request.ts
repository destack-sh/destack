import type { WebOptions } from "@destack/web/build";

/** Browser, server, static, and mixed outputs from one application. */
export const requests = {
    browser: { kind: "web", app: "src/app.tsx", ssr: false },
    server: { kind: "web", app: "src/app.tsx", ssr: { runtime: "bun" } },
    static: {
        kind: "web",
        app: "src/app.tsx",
        ssr: { runtime: "bun", emit: false },
        prerender: { origin: "https://example.test", routes: ["/", "/about/"] },
    },
    mixed: {
        kind: "web",
        app: "src/app.tsx",
        ssr: { runtime: "workerd" },
        prerender: { origin: "https://example.test", routes: ["/"] },
    },
} satisfies Record<string, WebOptions>;
