import { defineService } from "@destack/service";
import { branchObjects } from "@destack/space/object";
import { note, notebook } from "../object/index.ts";

/** Notebooks and notes, with their sharing, trash, sync and branches. */
export const notesService = defineService("notes", {
    objects: { notebook, note, ...branchObjects },
});
