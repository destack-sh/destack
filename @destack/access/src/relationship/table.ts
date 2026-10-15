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
import { accessRole, managerChecks } from "../role/table.ts";
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
        /** Optional expiry in UTC epoch milliseconds. */
        expiresAt: integer("expires_at"),
        /** The only request the relationship applies to, until it commits. */
        requestId: text("request_id"),
        /** The credential a call presents for it to apply, such as a session or agent. */
        credential: text("credential"),
        /** The digest of the link secret a request must present. */
        linkSecret: text("link_secret"),
        /** The minimum authentication assurance level. */
        assurance: integer("assurance"),
        /** The longest time since authentication, in milliseconds. */
        maxAge: integer("max_age"),
    },
    {
        log: { retention: "history" },
        constraints: (relationship) => [
            index("relationship_request").on(relationship.requestId),
            index("relationship_expiry").on(relationship.expiresAt),
            index("relationship_max_age").on(relationship.maxAge),
            check("relationship_assurance", {
                assurance: { OR: [{ isNull: true }, { gte: 1, lte: 3 }] },
            }),
            check("relationship_max_age", { maxAge: { OR: [{ isNull: true }, { gt: 0 }] } }),
            ...managerChecks("relationship", relationship),
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
                    relationship.requestId,
                    relationship.credential,
                    relationship.linkSecret,
                    relationship.assurance,
                    relationship.maxAge,
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

/** A relationship as `Relationship.encode` stores it, with subject and condition columns. */
export type RelationshipRow = Select<typeof accessRelationship>;

/** A relationship binding a role. */
export type RoleBinding = RelationshipRow & { readonly roleId: string };

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
        | "requestId"
        | "credential"
        | "linkSecret"
        | "assurance"
        | "maxAge",
        SQLWrapper
    >
>;
