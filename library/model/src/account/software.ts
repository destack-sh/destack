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
} from "@destack/db";
import { installation } from "../space/installation.ts";
import { space } from "../space/space.ts";
import { account } from "./account.ts";

/** An account-owned identity used by an installation or external automation. */
export const serviceAccount = table("service_account", {
    ...recordColumns("service-account"),
    /** The account administering this identity. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id),
    /** The account-local identity name. */
    name: text("name").notNull(),
    /** The installation's space, absent for external automation. */
    spaceId: identifier("space_id", "space"),
    /** The installation using this identity. */
    installationId: identifier("installation_id", "installation"),
    /** Revocation time for all credentials issued to this identity. */
    revokedAt: integer("revoked_at"),
}, (identity) => [
    unique("service_account_name").on(identity.accountId, identity.name),
    unique("service_account_account_id").on(identity.accountId, identity.id),
    unique("service_account_installation").on(identity.installationId),
    foreignKey({
        columns: [identity.accountId, identity.spaceId],
        foreignColumns: [space.accountId, space.id],
    }),
    foreignKey({
        columns: [identity.spaceId, identity.installationId],
        foreignColumns: [installation.spaceId, installation.id],
    }),
    check(
        "service_account_installation_space",
        sql`(${identity.spaceId} IS NULL) = (${identity.installationId} IS NULL)`,
    ),
]);

/** A token issued to software, stored only as a hash. */
export const serviceToken = table(
    "service_token",
    {
        ...recordColumns("service-token"),
        /** The software identity authenticated by this token. */
        serviceAccountId: identifier("service_account_id", "service-account").notNull().references(
            () => serviceAccount.id,
        ),
        /** The displayed token name. */
        name: text("name").notNull(),
        /** The unique hash of the bearer token. */
        tokenHash: text("token_hash").notNull().unique(),
        /** Expiry time in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** Explicit revocation time. */
        revokedAt: integer("revoked_at"),
    },
    (
        token,
    ) => [
        check("service_token_expiry", sql`${token.expiresAt} > ${token.createdAt}`),
    ],
);

/** A software identity. */
export type ServiceAccount = Select<typeof serviceAccount>;
/** A software token. */
export type ServiceToken = Select<typeof serviceToken>;
