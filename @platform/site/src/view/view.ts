import { defineView } from "@destack/view/declare";
import { origin } from "./content/site.ts";

/** The website: every page prerendered as HTML, its interactive parts hydrating as islands. */
export const site = defineView({
    name: "site",
    component: () => import("./app.tsx"),
    render: {
        mode: "static",
        origin,
        hydrate: "islands",
        document: () => import("./document.tsx"),
        pages: () => import("./pages.ts"),
    },
});
