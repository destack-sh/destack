import { defineView } from "@destack/view/declare";

/** A static site of two pages and a not-found page, with a sitemap. */
export const site = defineView({
    name: "site",
    component: () => import("./page.tsx"),
    render: { mode: "static", origin: "https://example.test", pages: () => import("./pages.ts") },
});
