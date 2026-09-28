import { defineTable, integer, json, text } from "@destack/db";
import { schema } from "@destack/schema";

/** The calls whose external work settles once. */
export const settlement = defineTable(
    "settlement",
    {
        /** The call's idempotency key. */
        id: text("id").primaryKey(),
        /** The object type the call acts on, by name. */
        object: text("object").notNull(),
        /** The method's name. */
        method: text("method").notNull(),
        /** The scope the call acts in. */
        scope: text("scope").notNull(),
        /** The object the call acts on or creates. */
        target: text("target"),
        /** The prepared value, null until the transaction commits. */
        prepared: json("prepared", schema.object({ value: schema.json() })),
        /** When the call reserved the settlement, in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** When a settler last claimed it, null while unclaimed. */
        claimedAt: integer("claimed_at"),
    },
    { log: {} },
);
