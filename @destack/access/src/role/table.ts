import {
    boolean,
    check,
    defineTable,
    identifier,
    index,
    integer,
    sql,
    text,
    unique,
    uniqueIndex,
    type Column,
} from "@destack/db";
import { nameCheck } from "../policy/expression.ts";

/** A named set of permissions defined within a scope. */
export const accessRole = defineTable(
    "role",
    {
        /** The immutable role identifier. */
        id: identifier("id", "role").primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The revision that conditional updates compare against. */
        revision: integer("revision").notNull().default(1),
        /** The installation whose declaration manages the role, absent for roles defined at runtime. */
        managerInstallationId: identifier("manager_installation_id", "installation"),
        /** The package of the managing declaration. */
        managerPackageId: identifier("manager_package_id", "package"),
        /** The managing declaration's name within its package. */
        managerName: text("manager_name"),
        /** When the declaration stopped managing the role, null while it manages it. */
        detachedAt: integer("detached_at"),
        /** The scope defining the role, routing its changes. */
        scope: text("scope").notNull(),
        /** The name, unique within the scope. */
        name: text("name").notNull(),
        /** The purpose shown when granting the role. */
        description: text("description").notNull(),
        /** Whether the role grants every permission in its scope, as an owner does, except reserved ones. */
        isUniversal: boolean("is_universal").notNull().default(false),
    },
    {
        log: { retention: "history" },
        constraints: (role) => [
            ...managerChecks("role", role),
            unique("role_scope_name").on(role.scope, role.name),
        ],
    },
);

/** One permission a role grants wherever it is bound. */
export const accessRolePermission = defineTable(
    "role_permission",
    {
        /** The permission identifier. */
        id: identifier("id", "role-permission").primaryKey(),
        /** The role containing the permission. */
        roleId: identifier("role_id", "role")
            .notNull()
            .references(() => accessRole.id, { onDelete: "cascade" }),
        /** The scope defining the role, routing its changes. */
        scope: text("scope").notNull(),
        /** The package declaring the permission. */
        packageId: identifier("package_id", "package").notNull(),
        /** The declared object type. */
        type: text("type").notNull(),
        /** The permission name within the object type. */
        name: text("name").notNull(),
    },
    {
        log: { retention: "history" },
        constraints: (permission) => [
            uniqueIndex("role_permission_name").on(
                permission.roleId,
                permission.packageId,
                permission.type,
                permission.name,
            ),
            index("role_permission_reference").on(
                permission.packageId,
                permission.type,
                permission.name,
            ),
            nameCheck("role_permission_type", permission.type),
            nameCheck("role_permission_name", permission.name),
        ],
    },
);

/** Require a complete manager or none, a detachment only of a managed row, and one row per declaration. */
export function managerChecks(
    name: string,
    columns: {
        /** The managing installation. */
        readonly managerInstallationId: Column;
        /** The managing package. */
        readonly managerPackageId: Column;
        /** The managing declaration's name. */
        readonly managerName: Column;
        /** The time the declaration stopped managing the row. */
        readonly detachedAt: Column;
    },
) {
    return [
        check(
            `${name}_manager`,
            sql`(${columns.managerInstallationId} IS NULL AND ${columns.managerPackageId} IS NULL AND ${columns.managerName} IS NULL) OR (${columns.managerInstallationId} IS NOT NULL AND ${columns.managerPackageId} IS NOT NULL AND ${columns.managerName} IS NOT NULL)`,
        ),
        check(
            `${name}_detached`,
            sql`${columns.detachedAt} IS NULL OR (${columns.managerInstallationId} IS NOT NULL AND ${columns.detachedAt} >= 0)`,
        ),
        uniqueIndex(`${name}_manager`)
            .on(columns.managerInstallationId, columns.managerPackageId, columns.managerName)
            .where(
                sql`${columns.managerInstallationId} IS NOT NULL AND ${columns.detachedAt} IS NULL`,
            ),
    ];
}
