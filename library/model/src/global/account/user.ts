import { recordColumns, type Select, table, text } from "@destack/db";

/** User records. */
export const user = table("user", {
    ...recordColumns("user"),
    /** The display name. */
    name: text("name").notNull(),
});

/** A persisted user record. */
export type User = Select<typeof user>;
