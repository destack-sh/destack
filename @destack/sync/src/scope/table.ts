import { check, identifier, integer, json, text, defineTable } from "@destack/db";
import { schema } from "@destack/schema";

/** A scope object's row: parent, type, suspension, storage cap and fence, in the scope itself. */
export const scopeTable = defineTable(
    "scope",
    {
        /** The scope of the record and the scope object's identifier. */
        scope: text("scope").primaryKey(),
        /** The scope containing this one, the universe for users and organisations. */
        parent: text("parent").notNull(),
        /** The enclosing scopes nearest first, as this row's database knew them at recording. */
        ancestors: json("ancestors", schema.array(schema.string())).notNull().default([]),
        /** The scope whose objects partition this scope's rows. */
        partition: text("partition").notNull(),
        /** The package declaring the scope object's type. */
        packageId: identifier("package_id", "package").notNull(),
        /** The scope object's type. */
        type: text("type").notNull(),
        /** When the scope was suspended, denying every access within it until resumed. */
        suspendedAt: integer("suspended_at"),
        /** When the scope's storage was capped, refusing writes but deletes until lifted. */
        cappedAt: integer("capped_at"),
        /** When a transfer stopped the scope's writes. */
        fencedAt: integer("fenced_at"),
        /** The machine a transfer moves the scope to. */
        movedTo: text("moved_to"),
    },
    {
        log: { retention: "history" },
        constraints: () => [
            check("scope_fence", {
                OR: [
                    { fencedAt: { isNull: true }, movedTo: { isNull: true } },
                    { fencedAt: { isNotNull: true }, movedTo: { isNotNull: true } },
                ],
            }),
        ],
    },
);
