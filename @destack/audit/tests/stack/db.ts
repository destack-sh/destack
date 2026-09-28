import { defineTable, text } from "@destack/db";

/** The documents the scenarios change. */
export const document = defineTable("document", { name: text("name").primaryKey().notNull() });

/** The accounts whose history the scenarios read. */
export const accountRecord = defineTable("account_record", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
});
