import type { Table } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { defineJournal } from "@destack/service/database";
import { branchTables } from "@destack/space/object";
import { note, notebook } from "../object/index.ts";

/** Replayable method requests. */
export const notesJournal = defineJournal("journal");

/** The tables of notebooks, notes and their branches, for a database embedding them. */
export const notesTables: readonly Table[] = [
    ...notebook.tables,
    ...note.tables,
    ...branchTables,
    notesJournal,
];

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({ name: "main", tables: notesTables });
