import { defineView } from "@destack/view/declare";
import { notebook } from "../object/index.ts";

/** Notebooks, listed by name. */
export const notebooks = defineView({
    name: "notebooks",
    objects: [notebook],
    permissions: { space: [notebook.permission("read")] },
    component: () => import("./app.tsx"),
});
