import {
    check,
    identifier,
    index,
    integer,
    defineTable,
    text,
    uniqueIndex,
    type Select,
    type SQLWrapper,
} from "@destack/db";
import { accessRole } from "../role/table.ts";
import { Manager } from "../manager/manager.ts";
import { nameCheck } from "../declare/expression.ts";

/** Relate an object to a subject through a declared relation or a bound role. */
export const accessRelationship = defineTable(
    "relationship",
    {
        /** The immutable relationship identifier. */
        id: identifier("id", "relationship").primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The revision a declaration's reconciliation advances. */
        revision: integer("revision").notNull().default(1),
        /** The installation whose declaration manages it, absent if granted at runtime. */
        managerInstallationId: identifier("manager_installation_id", "installation"),
        /** The package of the managing declaration. */
        managerPackageId: identifier("manager_package_id", "package"),
        /** The managing declaration's name within its package. */
        managerName: text("manager_name"),
        /** When the declaration stopped managing the relationship, null while it manages it. */
        detachedAt: integer("detached_at"),
        /** The scope the relationship lives in and routes changes to. */
        scope: text("scope").notNull(),
        /** The package declaring the object's type. */
        objectPackageId: identifier("object_package_id", "package").notNull(),
        /** The object's type. */
        objectType: text("object_type").notNull(),
        /** The scope containing the object. */
        objectScope: text("object_scope").notNull(),
        /** The object's identifier. */
        objectId: text("object_id").notNull(),
        /** The declared relation, absent for a role binding. */
        relation: text("relation"),
        /** The bound role, absent for a declared relation. */
        roleId: identifier("role_id", "role").references(() => accessRole.id, {
            onDelete: "no action",
        }),
        /** The package declaring the subject's type. */
        subjectPackageId: identifier("subject_package_id", "package").notNull(),
        /** The subject's type. */
        subjectType: text("subject_type").notNull(),
        /** The scope containing the subject, or `*` for every scope. */
        subjectScope: text("subject_scope").notNull(),
        /** The subject's identifier, or `*` for every object of its type in its scope. */
        subjectId: text("subject_id").notNull(),
        /** The relation whose members are the subjects, for a subject set. */
        subjectRelation: text("subject_relation"),
        /** The object's key, as `ObjectReference.key` writes it. */
        objectKey: text("object_key").notNull(),
        /** The key of the object whose set is the subject, absent for a direct subject. */
        subjectSetKey: text("subject_set_key"),
        /** Optional expiry in UTC epoch milliseconds. */
        expiresAt: integer("expires_at"),
        /** The digest of the link secret a request must present, for a share link. */
        linkSecret: text("link_secret"),
    },
    {
        log: { retention: "history", isShared: true },
        constraints: (relationship) => [
            index("relationship_expiry").on(relationship.expiresAt),
            index("relationship_object_key").on(relationship.objectKey),
            ...Manager.constraints("relationship", relationship),
            uniqueIndex("relationship_tuple")
                .on(
                    relationship.objectPackageId,
                    relationship.objectType,
                    relationship.objectScope,
                    relationship.objectId,
                    relationship.relation,
                    relationship.roleId,
                    relationship.subjectPackageId,
                    relationship.subjectType,
                    relationship.subjectScope,
                    relationship.subjectId,
                    relationship.subjectRelation,
                    relationship.linkSecret,
                )
                .nullsNotDistinct(),
            index("relationship_subject").on(
                relationship.subjectPackageId,
                relationship.subjectType,
                relationship.subjectScope,
                relationship.subjectId,
                relationship.subjectRelation,
            ),
            index("relationship_scope_subject").on(
                relationship.objectScope,
                relationship.subjectPackageId,
                relationship.subjectType,
                relationship.subjectScope,
                relationship.subjectId,
            ),
            index("relationship_role").on(relationship.roleId),
            check("relationship_label", {
                OR: [
                    { relation: { isNull: true }, roleId: { isNotNull: true } },
                    { relation: { isNotNull: true }, roleId: { isNull: true } },
                ],
            }),
            check("relationship_wildcard", {
                OR: [
                    { subjectRelation: { isNull: true } },
                    { subjectId: { ne: "*" }, subjectScope: { ne: "*" } },
                ],
            }),
            check("relationship_expiry", {
                expiresAt: { OR: [{ isNull: true }, { gt: { field: "createdAt" } }] },
            }),
            ...["objectType", "relation", "subjectType", "subjectRelation"].map(
                (property, position) => nameCheck(`relationship_name_${position}`, property),
            ),
        ],
    },
);

/** A relationship binding a role. */
export type RoleBinding = Select<typeof accessRelationship> & { readonly roleId: string };

/** The relationship columns of the table or one of its aliases. */
export type RelationshipColumnMap = Readonly<
    Record<
        | "scope"
        | "objectPackageId"
        | "objectType"
        | "objectScope"
        | "objectId"
        | "relation"
        | "roleId"
        | "subjectPackageId"
        | "subjectType"
        | "subjectScope"
        | "subjectId"
        | "subjectRelation"
        | "createdAt"
        | "expiresAt"
        | "linkSecret",
        SQLWrapper
    >
>;
