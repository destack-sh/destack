import { identifier, integer, type Select, table, text } from "@destack/db";

/** A Git working directory registered on this host. */
export const checkout = table("checkout", {
    /** The checkout identifier. */
    id: identifier("id", "checkout").primaryKey().notNull(),
    /** The source repository identifier, whose registration can be remote. */
    repositoryId: identifier("repository_id", "repository"),
    /** The absolute working directory path. */
    path: text("path").notNull().unique(),
    /** Registration time in UTC epoch milliseconds. */
    createdAt: integer("created_at").notNull(),
});

/** A registered checkout. */
export type Checkout = Select<typeof checkout>;
