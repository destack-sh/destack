import { defineTable, index, integer, text } from "@destack/db";

/** The keys a store follows a shape at on demand, each until its lease lapses. */
export const replicaLease = defineTable(
    "replica_lease",
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
    { constraints: (lease) => [index("replica_lease_expiry").on(lease.expiresAt)] },
);
