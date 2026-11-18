import { defineTable, Expression, index, integer, text } from "@destack/db";

/** The follower of the keys a store acquires for itself. */
export const STORE_FOLLOWER = "store";

/** The keys a store follows a shape at on demand, each for a follower, open or until it lapses. */
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
        /** The follower following the key: the store itself, a client, or a follower's stream. */
        follower: text("follower").primaryKey(),
        /** The lapse time in UTC epoch milliseconds, absent while the follower follows the key. */
        expiresAt: integer("expires_at"),
    },
    {
        // give the keys from before followers their one follower, the store
        convert: { 2: { follower: Expression.literal(STORE_FOLLOWER) } },
        constraints: (key) => [index("follow_key_expiry").on(key.expiresAt)],
    },
);
