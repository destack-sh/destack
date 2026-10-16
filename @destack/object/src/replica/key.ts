import { defineTable, index, integer, text } from "@destack/db";

/** The keys a store follows a shape at on demand, each until it lapses. */
export const followKey = defineTable(
    "follow_key",
    {
        /** The followed shape's name. */
        shape: text("shape").primaryKey(),
        /** The scope the followed rows live in. */
        scope: text("scope").primaryKey(),
        /** The key's parameters beside the scope, as canonical JSON. */
        parameters: text("parameters").primaryKey(),
        /** The principal the replica is decided for, as canonical JSON. */
        subject: text("subject").primaryKey(),
        /** The lapse time, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    { constraints: (key) => [index("follow_key_expiry").on(key.expiresAt)] },
);
