import {
    check,
    foreignKey,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
    uniqueIndex,
} from "@destack/db";
import { account } from "./account.ts";
import { space } from "../space/space.ts";
import { permissionChecks, permissionColumns } from "@destack/access/database";

/** An account-owned identity for external automation. */
export const serviceAccount = table(
    "service_account",
    {
        ...recordColumns("service-account"),
        /** The account administering this identity. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id),
        /** The account-local identity name. */
        name: text("name").notNull(),
        /** Revocation time for all credentials issued to this identity. */
        revokedAt: integer("revoked_at"),
    },
    (identity) => [
        unique("service_account_name").on(identity.accountId, identity.name),
        unique("service_account_account_id").on(identity.accountId, identity.id),
    ],
);

/** A token issued to software, stored only as a hash. */
export const serviceToken = table(
    "service_token",
    {
        ...recordColumns("service-token"),
        /** The account restricting the identity and all token permissions. */
        accountId: identifier("account_id", "account").notNull(),
        /** The software identity authenticated by this token. */
        serviceAccountId: identifier("service_account_id", "service-account").notNull(),
        /** The displayed token name. */
        name: text("name").notNull(),
        /** The unique hash of the bearer token. */
        tokenHash: text("token_hash").notNull().unique(),
        /** Expiry time in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** Explicit revocation time. */
        revokedAt: integer("revoked_at"),
    },
    (token) => [
        check("service_token_expiry", sql`${token.expiresAt} > ${token.createdAt}`),
        unique("service_token_account_id").on(token.accountId, token.id),
        foreignKey({
            columns: [token.accountId, token.serviceAccountId],
            foreignColumns: [serviceAccount.accountId, serviceAccount.id],
        }).onDelete("cascade"),
    ],
);

/** Restrict a software credential within its identity's current permissions. */
export const serviceTokenPermission = table(
    "service_token_permission",
    {
        /** The immutable permission identifier. */
        id: identifier("id", "token-permission").primaryKey().notNull(),
        /** The account containing the token and optional space. */
        accountId: identifier("account_id", "account").notNull(),
        /** The credential receiving this restriction. */
        tokenId: identifier("token_id", "service-token").notNull(),
        /** An optional space restriction within the account. */
        spaceId: identifier("space_id", "space"),
        ...permissionColumns(),
    },
    (permission) => [
        foreignKey({
            columns: [permission.accountId, permission.tokenId],
            foreignColumns: [serviceToken.accountId, serviceToken.id],
        }).onDelete("cascade"),
        foreignKey({
            columns: [permission.accountId, permission.spaceId],
            foreignColumns: [space.accountId, space.id],
        }).onDelete("cascade"),
        uniqueIndex("service_token_permission_scope").on(
            permission.tokenId,
            sql`coalesce(${permission.spaceId}, '')`,
            permission.packageId,
            permission.type,
            permission.name,
            sql`coalesce(${permission.objectId}, '')`,
        ),
        ...permissionChecks("service_token_permission", permission),
    ],
);

/** An identity for external automation. */
export type ServiceAccount = Select<typeof serviceAccount>;
/** A software token. */
export type ServiceToken = Select<typeof serviceToken>;
/** A persisted software credential permission. */
export type ServiceTokenPermission = Select<typeof serviceTokenPermission>;
