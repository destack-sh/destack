import { defineView } from "@destack/view/declare";
import { t } from "@destack/locale";

/** A static site of two pages and a not-found page, with a sitemap. */
export const site = defineView({
    name: "site",
    title: t`Site`,
    component: () => import("./page.tsx"),
    render: { mode: "static", origin: "https://example.test", pages: () => import("./pages.ts") },
});
