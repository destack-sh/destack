import { check, identifier, type Select, sql, table, text, uniqueIndex } from "@destack/db";
import { role } from "./role.ts";

/** A positive permission evaluated within a role binding's scope. */
export const permission = table("permission", {
    /** The permission identifier. */
    id: identifier("id", "permission").primaryKey().notNull(),
    /** The role containing this permission. */
    roleId: identifier("role_id", "role").notNull().references(() => role.id, {
        onDelete: "cascade",
    }),
    /** The namespaced API resource type, such as destack.space or package-defined types. */
    resource: text("resource").notNull(),
    /** The exact declared action, such as read, update, publish, bind, or invoke. */
    action: text("action").notNull(),
    /** An exact resource name within the bound scope; null grants all names of this type. */
    name: text("name"),
}, (permission) => [
    uniqueIndex("permission_named").on(
        permission.roleId,
        permission.resource,
        permission.action,
        permission.name,
    ).where(sql`${permission.name} IS NOT NULL`),
    uniqueIndex("permission_all").on(permission.roleId, permission.resource, permission.action)
        .where(sql`${permission.name} IS NULL`),
    check(
        "permission_resource",
        sql`length(${permission.resource}) > 0 AND instr(${permission.resource}, '.') > 0 AND instr(${permission.resource}, '*') = 0`,
    ),
    check(
        "permission_action",
        sql`length(${permission.action}) > 0 AND instr(${permission.action}, '*') = 0`,
    ),
    check("permission_name", sql`${permission.name} IS NULL OR length(${permission.name}) > 0`),
]);

/** A role permission; grants are additive and unmatched requests are denied. */
export type Permission = Select<typeof permission>;
