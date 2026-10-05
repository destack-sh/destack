import { type Table, defineDatabase } from "@destack/db";
import { journal } from "@destack/audit/stack";
import { page } from "../object/index.ts";

/** The tables of pages, for a database embedding them. */
export const pagesTables: readonly Table[] = [...page.tables, journal];

/** The database of one space's pages. */
export const pagesDatabase = defineDatabase({ name: "main", tables: pagesTables });
