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
        /** The installation whose declaration manages the relationship, absent for relationships granted at runtime. */
        managerInstallationId: identifier("manager_installation_id", "installation"),
        /** The package of the managing declaration. */
        managerPackageId: identifier("manager_package_id", "package"),
        /** The managing declaration's name within its package. */
        managerName: text("manager_name"),
        /** When the declaration stopped managing the relationship, null while it manages it. */
        detachedAt: integer("detached_at"),
        /** The scope the relationship lives in and routes changes to. */
        scope: text("scope").notNull(),
        /** The scope containing the object. */
        objectScope: text("object_scope").notNull(),
        /** The package declaring the object's type. */
        packageId: identifier("package_id", "package").notNull(),
        /** The object's type. */
        type: text("type").notNull(),
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
        /** The request the relationship applies to alone, until it commits. */
        requestId: text("request_id"),
        /** The credential a call presents for the relationship to apply, such as a session or an agent instance. */
        credential: text("credential"),
        /** The digest of the link secret a request must present. */
        linkSecret: text("link_secret"),
        /** The minimum authentication assurance level. */
        assurance: integer("assurance"),
        /** The longest time since authentication, in milliseconds. */
        maxAge: integer("max_age"),
        /** The key of the principal a delegate must act for, making the relationship a delegation. */
        onBehalfOf: text("on_behalf_of"),
    },
    {
        log: { retention: "history" },
        constraints: (relationship) => [
            index("relationship_request").on(relationship.requestId),
            index("relationship_expiry").on(relationship.expiresAt),
            index("relationship_max_age").on(relationship.maxAge),
            index("relationship_delegation").on(relationship.onBehalfOf),
            check("relationship_assurance", {
                assurance: { OR: [{ isNull: true }, { gte: 1, lte: 3 }] },
            }),
            check("relationship_max_age", { maxAge: { OR: [{ isNull: true }, { gt: 0 }] } }),
            ...managerChecks("relationship", relationship),
            uniqueIndex("relationship_tuple")
                .on(
                    relationship.objectScope,
                    relationship.packageId,
                    relationship.type,
                    relationship.objectId,
                    relationship.relation,
                    relationship.roleId,
                    relationship.subjectPackageId,
                    relationship.subjectType,
                    relationship.subjectScope,
                    relationship.subjectId,
                    relationship.subjectRelation,
                    relationship.onBehalfOf,
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
            ...["type", "relation", "subjectType", "subjectRelation"].map((property, position) =>
                nameCheck(`relationship_name_${position}`, property),
            ),
        ],
    },
);

/** A relationship as `Relationship.encode` stores it, its subject and conditions in columns of their own. */
export type RelationshipRow = Select<typeof accessRelationship>;

/** The relationship columns of the table or one of its aliases. */
export type RelationshipColumnMap = Readonly<
    Record<
        | "scope"
        | "objectScope"
        | "packageId"
        | "type"
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
        | "maxAge"
        | "onBehalfOf",
        SQLWrapper
    >
>;
