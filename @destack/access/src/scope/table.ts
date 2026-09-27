import { identifier, integer, json, sql, text, defineTable } from "@destack/db";
import { schema } from "@destack/schema";

/** The scope each scope object is: the scope containing it, its type, and whether it is suspended, living in the scope itself as its access does. */
export const accessScope = defineTable(
    "scope",
    {
        /** The scope, which the record lives in, and which is also the scope object's identifier. */
        scope: text("scope").primaryKey().notNull(),
        /** The scope containing this one, the global scope for users and organisations. */
        parent: text("parent").notNull(),
        /** The enclosing scopes nearest first, as far as this row's own database knew them when recording it. */
        ancestors: json("ancestors", schema.array(schema.string()))
            .notNull()
            .default(sql`'[]'`),
        /** The package declaring the scope object's type. */
        packageId: identifier("package_id", "package").notNull(),
        /** The scope object's type. */
        type: text("type").notNull(),
        /** When the scope was suspended, denying every access within it until resumed. */
        suspendedAt: integer("suspended_at"),
    },
    { log: { tier: "history" } },
);
