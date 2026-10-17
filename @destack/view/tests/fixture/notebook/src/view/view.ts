import { defineView } from "@destack/view/declare";
import { notebook } from "../object/index.ts";

/** Notebooks, listed by name. */
export const notebooks = defineView({
    name: "notebooks",
    objects: [notebook],
    component: () => import("./app.tsx"),
});
