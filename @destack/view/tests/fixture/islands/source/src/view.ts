import { defineView } from "@destack/view/declare";
import { t } from "@destack/locale";

/** A static page whose counter alone hydrates. */
export const site = defineView({
    name: "site",
    title: t`Site`,
    component: () => import("./page.tsx"),
    render: {
        mode: "static",
        origin: "https://example.test",
        hydrate: "islands",
        pages: () => import("./pages.ts"),
    },
});
