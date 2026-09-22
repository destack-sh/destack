import {
    check,
    defineDatabaseSchema,
    identifier,
    index,
    integer,
    sql,
    table,
    text,
} from "@destack/db";

/** Persist explicit sharing independently of application ownership and hierarchy. */
export const accessGrant = table(
    "access_grant",
    {
        id: text("id").primaryKey().notNull(),
        packageId: identifier("package_id", "package").notNull(),
        type: text("type").notNull(),
        scope: text("scope").notNull(),
        objectId: text("object_id").notNull(),
        relation: text("relation").notNull(),
        subjectKind: text("subject_kind", {
            enum: ["user", "service-account", "group", "share-token", "everyone"],
        }).notNull(),
        subjectAuthority: text("subject_authority").notNull(),
        subjectId: text("subject_id").notNull(),
        createdAt: integer("created_at").notNull(),
        expiresAt: integer("expires_at"),
        revokedAt: integer("revoked_at"),
    },
    (grant) => [
        index("access_grant_object").on(
            grant.scope,
            grant.packageId,
            grant.type,
            grant.objectId,
            grant.relation,
        ),
        index("access_grant_subject").on(
            grant.subjectKind,
            grant.subjectAuthority,
            grant.subjectId,
            grant.scope,
        ),
        check(
            "access_grant_expiry",
            sql`${grant.expiresAt} IS NULL OR ${grant.expiresAt} > ${grant.createdAt}`,
        ),
        check(
            "access_grant_subject",
            sql`(${grant.subjectKind} = 'everyone' AND ${grant.subjectAuthority} = '' AND ${grant.subjectId} = '') OR (${grant.subjectKind} <> 'everyone' AND length(${grant.subjectAuthority}) > 0 AND length(${grant.subjectId}) > 0)`,
        ),
    ],
);

/** Bearer credentials whose current state governs all grants made to the token. */
export const accessToken = table(
    "access_token",
    {
        id: text("id").primaryKey().notNull(),
        scope: text("scope").notNull(),
        digest: text("digest").notNull().unique(),
        createdAt: integer("created_at").notNull(),
        expiresAt: integer("expires_at"),
        revokedAt: integer("revoked_at"),
    },
    (token) => [
        check(
            "access_token_expiry",
            sql`${token.expiresAt} IS NULL OR ${token.expiresAt} > ${token.createdAt}`,
        ),
    ],
);

/** Compose access persistence into the database containing protected application records. */
export const accessSchema = defineDatabaseSchema({
    name: "destack-access",
    tables: { accessGrant, accessToken },
    migrations: new URL("./migration/", import.meta.url),
});
