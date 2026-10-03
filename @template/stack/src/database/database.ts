import { defineDatabase } from "@destack/db";

/** The space's shared application database. */
export const database = defineDatabase({
    name: "main",
    tables: [],
});
