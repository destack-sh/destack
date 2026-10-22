import { integer, defineTable, text, defineDatabase } from "@destack/db";

/** Notes stored in the destination database. */
export const note = defineTable("note", {
    id: integer("id").primaryKey(),
    title: text("title").notNull(),
});

/** The database selected by the destination space. */
export const database = defineDatabase({
    name: "main",
    tables: [note],
});
