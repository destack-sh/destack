import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";

/** The start view, opened at the root: an empty page with Destack's mark. */
export const start = defineView({
    name: "start",
    title: t`Start`,
    categories: ["main"],
    audience: "anyone",
    component: () => import("./blank.tsx"),
});
