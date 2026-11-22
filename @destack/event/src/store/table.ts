import { defineTable, integer, text } from "@destack/db";

/** The keys of the people in a database's events, each encrypted under the host's keyring. */
export const personalKey = defineTable("personal_key", {
    /** The person. */
    subject: text("subject").primaryKey(),
    /** The key's identity. */
    id: text("id").primaryKey(),
    /** The AES-256 key's bytes, encrypted under the host's keyring. */
    sealed: text("sealed").notNull(),
    /** When the key was created, in Unix milliseconds. */
    createdAt: integer("created_at").notNull(),
});
