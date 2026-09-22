import {
    check,
    foreignKey,
    identifier,
    index,
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
import { user } from "./user.ts";
import { permissionChecks, permissionColumns } from "@destack/access/database";
import { space } from "../space/space.ts";

/** A user-issued credential restricted to one account and explicit permissions. */
export const personalAccessToken = table(
    "personal_access_token",
    {
        ...recordColumns("personal-access-token"),
        /** The user whose current permissions constrain this token. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, { onDelete: "cascade" }),
        /** The account whose assets this token may access. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id, { onDelete: "cascade" }),
        /** The name displayed when reviewing issued credentials. */
        name: text("name").notNull(),
        /** The SHA-256 hash of the random bearer credential. */
        tokenHash: text("token_hash").notNull().unique(),
        /** The credential expiry in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The time the credential was revoked. */
        revokedAt: integer("revoked_at"),
    },
    (token) => [
        unique("personal_access_token_account_id").on(token.accountId, token.id),
        index("personal_access_token_user").on(token.userId),
        unique("personal_access_token_user_id").on(token.userId, token.id),
        check("personal_access_token_expiry", sql`${token.expiresAt} > ${token.createdAt}`),
    ],
);

/** An explicit token permission intersected with the user's current authorization. */
export const personalAccessTokenPermission = table(
    "personal_access_token_permission",
    {
        /** The permission identifier. */
        id: identifier("id", "token-permission").primaryKey(),
        /** The account restricting this permission. */
        accountId: identifier("account_id", "account").notNull(),
        /** The token receiving the permission. */
        tokenId: identifier("token_id", "personal-access-token").notNull(),
        /** An optional space restriction within the token's account. */
        spaceId: identifier("space_id", "space"),
        ...permissionColumns(),
    },
    (permission) => [
        foreignKey({
            columns: [permission.accountId, permission.tokenId],
            foreignColumns: [personalAccessToken.accountId, personalAccessToken.id],
        }).onDelete("cascade"),
        foreignKey({
            columns: [permission.accountId, permission.spaceId],
            foreignColumns: [space.accountId, space.id],
        }).onDelete("cascade"),
        uniqueIndex("personal_access_token_permission_scope").on(
            permission.tokenId,
            sql`coalesce(${permission.spaceId}, '')`,
            permission.packageId,
            permission.type,
            permission.name,
            sql`coalesce(${permission.objectId}, '')`,
        ),
        ...permissionChecks("personal_access_token_permission", permission),
    ],
);

/** A persisted personal access token. */
export type PersonalAccessToken = Select<typeof personalAccessToken>;
/** A persisted personal access token permission. */
export type PersonalAccessTokenPermission = Select<typeof personalAccessTokenPermission>;
