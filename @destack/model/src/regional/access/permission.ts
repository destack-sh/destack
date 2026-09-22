import { identifier, type Select, sql, table, uniqueIndex } from "@destack/db";
import { permissionChecks, permissionColumns } from "@destack/access/database";
import { role } from "./role.ts";

/** A positive permission evaluated within a role binding's scope. */
export const rolePermission = table(
    "role_permission",
    {
        /** The permission identifier. */
        id: identifier("id", "role-permission").primaryKey().notNull(),
        /** The role containing this permission. */
        roleId: identifier("role_id", "role")
            .notNull()
            .references(() => role.id, { onDelete: "cascade" }),
        ...permissionColumns(),
    },
    (permission) => [
        uniqueIndex("role_permission_scope").on(
            permission.roleId,
            permission.packageId,
            permission.type,
            permission.name,
            sql`coalesce(${permission.objectId}, '')`,
        ),
        ...permissionChecks("role_permission", permission),
    ],
);

/** A role permission evaluated within its binding's scope. */
export type RolePermission = Select<typeof rolePermission>;
