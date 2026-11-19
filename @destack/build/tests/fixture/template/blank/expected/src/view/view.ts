import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";

/** The start view: an empty page with Destack's mark. */
export const start = defineView({
    name: "start",
    title: t`Start`,
    audience: "anyone",
    component: () => import("./blank.tsx"),
});
