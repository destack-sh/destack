import { defineDatabase } from "@destack/db/declare";
import { defineJournal } from "@destack/service/database";
import { page } from "../object/index.ts";

/** Replayable method requests. */
export const pagesJournal = defineJournal("journal");

/** The database of one space's pages. */
export const pageDatabase = defineDatabase({
    name: "main",
    tables: [...page.tables, pagesJournal],
});
