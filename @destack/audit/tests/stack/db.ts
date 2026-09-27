import { defineTable, text } from "@destack/db";

/** Application state changed in the same transaction as its audit event. */
export const document = defineTable("document", { name: text("name").primaryKey().notNull() });

/** Accounts, the scope objects whose history the service scenarios read. */
export const accountRecord = defineTable("account_record", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
});
