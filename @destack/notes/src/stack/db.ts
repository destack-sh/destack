import { type Table, defineDatabase } from "@destack/db";
import { journal } from "@destack/audit";
import { branchTables } from "@destack/space/object";
import { note, notebook } from "../object/index.ts";

/** The tables of notebooks, notes and their branches, for a database embedding them. */
export const notesTables: readonly Table[] = [
    ...notebook.tables,
    ...note.tables,
    ...branchTables,
    journal,
];

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({ name: "main", tables: notesTables });
