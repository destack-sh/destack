import { check, defineTable, foreignKey, identifier, sql, text, unique } from "@destack/db";
import { resource } from "@destack/space/stack";

/** A bucket resource administered in a space. */
export const bucket = defineTable(
    "bucket",
    {
        /** The underlying resource. */
        resourceId: identifier("resource_id", "resource").primaryKey().notNull(),
        /** The space the bucket lives in. */
        scope: identifier("scope", "space").notNull(),
        /** The fixed resource kind. */
        kind: text("kind").notNull().default("bucket"),
    },
    {
        constraints: (entry) => [
            unique("bucket_scope_id").on(entry.scope, entry.resourceId),
            foreignKey({
                columns: [entry.scope, entry.resourceId, entry.kind],
                foreignColumns: [resource.scope, resource.id, resource.kind],
            }).onDelete("restrict"),
            check("bucket_kind", sql`${entry.kind} = 'bucket'`),
        ],
    },
);
