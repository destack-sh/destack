import { defineDatabaseSchema, integer, table, text } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";

/** The database selected by the destination space. */
export const database = defineDatabase({ name: "main", spec: { dialect: "sqlite" } });

/** Notes stored in the destination database. */
const note = table("note", { id: integer("id").primaryKey(), title: text("title").notNull() });

/** Tables and committed migration files distributed with the package. */
export const notes = defineDatabaseSchema({
    name: "notes",
    tables: { note },
    migrations: new URL("./migration/", import.meta.url),
});
