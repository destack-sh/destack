import type { ApplicationOptions } from "../../../src/index.ts";

/** Browser, server, static, and mixed outputs from one application. */
export const requests = {
    browser: { kind: "web", ssr: false },
    server: { kind: "web", ssr: { runtime: "bun" } },
    static: {
        kind: "web",
        ssr: { runtime: "bun", emit: false },
        prerender: { origin: "https://example.test", routes: ["/", "/about/"] },
    },
    mixed: {
        kind: "web",
        ssr: { runtime: "workerd" },
        prerender: { origin: "https://example.test", routes: ["/"] },
    },
} satisfies Record<string, ApplicationOptions>;
