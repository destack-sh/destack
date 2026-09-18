import { identifier, integer, type Select, table, text } from "@destack/db";

/** A configured Destack administration endpoint. */
export const serverConnection = table("server_connection", {
    /** The local connection identifier. */
    id: identifier("id", "server-connection").primaryKey().notNull(),
    /** The administration service URL. */
    url: text("url").notNull().unique(),
    /** The displayed connection name. */
    name: text("name").notNull(),
    /** The OS keychain reference for this connection's session. */
    credential: text("credential"),
    /** Creation time in UTC epoch milliseconds. */
    createdAt: integer("created_at").notNull(),
});

/** A configured server connection. */
export type ServerConnection = Select<typeof serverConnection>;
