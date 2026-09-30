import { defineView } from "@destack/view/declare";
import { note, notebook } from "../object/index.ts";

/** Notebooks and their notes, edited together live. */
export const notes = defineView({
    name: "notes",
    permissions: [
        notebook.permission("read"),
        notebook.permission("manage"),
        note.permission("read"),
        note.permission("edit"),
    ],
    component: () => import("./app.tsx"),
});
