import { defineService } from "@destack/service";
import { note, notebook } from "../object/index.ts";

/** Notebooks and notes, with their sharing, trash and sync. */
export const notesService = defineService("notes", {
    objects: { notebook, note },
});
