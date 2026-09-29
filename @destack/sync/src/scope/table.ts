import { check, identifier, integer, json, sql, text, defineTable } from "@destack/db";
import { schema } from "@destack/schema";

/** A scope object's row: its parent, type, suspension and transfer fence, stored in the scope itself. */
export const scopeTable = defineTable(
    "scope",
    {
        /** The scope of the record and the scope object's identifier. */
        scope: text("scope").primaryKey().notNull(),
        /** The scope containing this one, the universe for users and organisations. */
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
        /** When a transfer stopped the scope's writes. */
        fencedAt: integer("fenced_at"),
        /** The cell a transfer moves the scope to. */
        movedTo: text("moved_to"),
    },
    {
        log: { retention: "history" },
        constraints: (scope) => [
            check("scope_fence", sql`(${scope.fencedAt} IS NULL) = (${scope.movedTo} IS NULL)`),
        ],
    },
);
