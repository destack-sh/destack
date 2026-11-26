import { defineView } from "@destack/view/declare";
import { notebook } from "../object/index.ts";
import { t } from "@destack/locale";

/** Notebooks, listed by name. */
export const notebooks = defineView({
    name: "notebooks",
    title: t`Notebooks`,
    objects: [notebook],
    component: () => import("./app.tsx"),
});
