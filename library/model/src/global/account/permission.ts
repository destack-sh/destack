import { check, identifier, type Select, sql, table, text, uniqueIndex } from "@destack/db";
import { role } from "./role.ts";

/** A positive permission evaluated within a role binding's scope. */
export const rolePermission = table("role_permission", {
    /** The permission identifier. */
    id: identifier("id", "role-permission").primaryKey().notNull(),
    /** The role containing this permission. */
    roleId: identifier("role_id", "role").notNull().references(() => role.id, {
        onDelete: "cascade",
    }),
    /** The namespaced API resource type, such as destack.space or package-defined types. */
    resource: text("resource").notNull(),
    /** The exact declared action, such as read, update, publish, bind, or invoke. */
    action: text("action").notNull(),
    /** A stable object identifier within the bound scope; null grants all objects of this type. */
    resourceId: text("resource_id"),
}, (rolePermission) => [
    uniqueIndex("role_permission_object").on(
        rolePermission.roleId,
        rolePermission.resource,
        rolePermission.action,
        rolePermission.resourceId,
    ).where(sql`${rolePermission.resourceId} IS NOT NULL`),
    uniqueIndex("role_permission_all").on(
        rolePermission.roleId,
        rolePermission.resource,
        rolePermission.action,
    )
        .where(sql`${rolePermission.resourceId} IS NULL`),
    check(
        "role_permission_resource",
        sql`length(${rolePermission.resource}) > 0 AND instr(${rolePermission.resource}, '.') > 0 AND instr(${rolePermission.resource}, '*') = 0`,
    ),
    check(
        "role_permission_action",
        sql`length(${rolePermission.action}) > 0 AND instr(${rolePermission.action}, '*') = 0`,
    ),
    check(
        "role_permission_identifier",
        sql`${rolePermission.resourceId} IS NULL OR length(${rolePermission.resourceId}) > 0`,
    ),
]);

/** A role permission; grants are additive and unmatched requests are denied. */
export type RolePermission = Select<typeof rolePermission>;
