import { defineTable, index, integer, primaryKey, text } from "@destack/db";

/** The entries of every declared unique index, one per key. */
export const keyEntry = defineTable(
    "key_entry",
    {
        /** The index: the object type's package, type and index name. */
        index: text("index").notNull(),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: text("key").notNull(),
        /** The object holding the key. */
        objectId: text("object_id").notNull(),
        /** The scope the object lives in. */
        scope: text("scope").notNull(),
        /** Whether the object's write committed or may still fail. */
        state: text("state", { enum: ["reserved", "confirmed"] }).notNull(),
        /** The request whose write reserved the key. */
        requestId: text("request_id").notNull(),
        /** The reservation deadline, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        constraints: (entry) => [
            primaryKey({ columns: [entry.index, entry.key] }),
            index("key_entry_request").on(entry.requestId),
            index("key_entry_object").on(entry.index, entry.objectId),
            index("key_entry_expiry").on(entry.state, entry.expiresAt),
        ],
        log: {},
    },
);
