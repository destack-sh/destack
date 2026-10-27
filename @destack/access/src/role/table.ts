import { defineTable, identifier, index, integer, text, unique, uniqueIndex } from "@destack/db";
import { schema } from "@destack/schema";
import { nameCheck } from "../declare/expression.ts";
import { Manager } from "../manager/manager.ts";

/** The one owner role, bound in each scope to its owners: every permission but reserved ones. */
export const OWNER_ROLE = {
    id: schema.identifier("role").parse("role-00000000-0000-7000-8000-000000000001"),
    createdAt: 0,
    updatedAt: 0,
    revision: 1,
    ...Manager.values(null),
    scope: "universe",
    name: "owner",
    description: "Every permission in the scope it is bound in except reserved ones",
} as const;

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
        /** The installation whose declaration manages it, absent if defined at runtime. */
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
    },
    {
        log: { retention: "history" },
        constraints: (role) => [
            ...Manager.constraints("role", role),
            unique("role_scope_name").on(role.scope, role.name),
        ],
        predefined: [OWNER_ROLE],
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
        /** The approvals each use waits for, none to run it at once. */
        approvals: integer("approvals").notNull().default(0),
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
            nameCheck("role_permission_type", "type"),
            nameCheck("role_permission_name", "name"),
        ],
    },
);
