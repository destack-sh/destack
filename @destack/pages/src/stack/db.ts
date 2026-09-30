import type { Table } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { defineJournal } from "@destack/service/database";
import { page } from "../object/index.ts";

/** Replayable method requests. */
export const pagesJournal = defineJournal("journal");

/** The tables of pages, for a database embedding them. */
export const pagesTables: readonly Table[] = [...page.tables, pagesJournal];

/** The database of one space's pages. */
export const pagesDatabase = defineDatabase({ name: "main", tables: pagesTables });
