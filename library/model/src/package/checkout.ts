import { identifier, integer, type Select, table, text } from "@destack/db";

/** A local Git working directory. */
export const checkout = table("checkout", {
    /** The checkout identifier. */
    id: identifier("id", "checkout").primaryKey().notNull(),
    /** The source repository identifier, whose registration can be remote. */
    repositoryId: identifier("repository_id", "repository").notNull(),
    /** The absolute working directory path. */
    path: text("path").notNull().unique(),
    /** SpaceRegistration time in UTC epoch milliseconds. */
    createdAt: integer("created_at").notNull(),
});

/** A registered checkout. */
export type Checkout = Select<typeof checkout>;
