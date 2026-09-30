import type { Table } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { defineJournal } from "@destack/service/database";
import { note, notebook } from "../object/index.ts";

/** Replayable method requests. */
export const notesJournal = defineJournal("journal");

/** The tables of notebooks and notes, for a database embedding them. */
export const notesTables: readonly Table[] = [...notebook.tables, ...note.tables, notesJournal];

/** The database of one space's notebooks and notes. */
export const notesDatabase = defineDatabase({ name: "main", tables: notesTables });
