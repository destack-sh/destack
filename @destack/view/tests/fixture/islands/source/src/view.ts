import { defineView } from "@destack/view/declare";

/** A static page whose counter alone hydrates. */
export const site = defineView({
    name: "site",
    component: () => import("./page.tsx"),
    render: {
        mode: "static",
        origin: "https://example.test",
        hydrate: "islands",
        pages: () => import("./pages.ts"),
    },
});
