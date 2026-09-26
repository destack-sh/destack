import { defineDatabase } from "@destack/db/declare";
import { defineJournal } from "@destack/service/database";
import { note, notebook } from "../object/index.ts";

/** Replayable method requests. */
export const notesJournal = defineJournal("journal");

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({
    name: "main",
    tables: [...notebook.tables, ...note.tables, notesJournal],
});
