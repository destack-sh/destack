import { defineView } from "@destack/view/declare";
import { origin } from "./content/site.ts";
import { t } from "@destack/locale";

/** The website: every page prerendered as HTML, its interactive parts hydrating as islands. */
export const site = defineView({
    name: "site",
    title: t`Site`,
    component: () => import("./app.tsx"),
    render: {
        mode: "static",
        origin,
        hydrate: "islands",
        document: () => import("./document.tsx"),
        pages: () => import("./pages.ts"),
    },
});
