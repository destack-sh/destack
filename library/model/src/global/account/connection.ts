import {
    check,
    foreignKey,
    identifier,
    index,
    integer,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
    uniqueIndex,
} from "@destack/db";
import { schema } from "@destack/schema";
import { spaceDirectory } from "../directory/space.ts";
import { account } from "./account.ts";
import { user } from "./user.ts";

/** An external account authorised for use by Destack software. */
export const connectedAccount = table("connected_account", {
    ...recordColumns("connected-account"),
    /** The Destack account authorised to use this connection. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "restrict",
    }),
    /** The user who authorised the external account. */
    userId: identifier("user_id", "user").notNull().references(() => user.id, {
        onDelete: "restrict",
    }),
    /** The external service provider. */
    provider: text("provider").notNull(),
    /** The provider instance, including enterprise or self-hosted installations. */
    issuer: text("issuer").notNull(),
    /** The provider's stable account identifier. */
    subject: text("subject").notNull(),
    /** The provider application receiving this authorisation. */
    applicationId: text("application_id").notNull(),
    /** A user OAuth grant or a provider application installation. */
    kind: text("kind", { enum: ["oauth", "installation"] }).notNull(),
    /** The provider's installation identifier. */
    installationId: text("installation_id"),
    /** The authorised provider scopes. */
    scopes: json("scopes", schema.array(schema.string())).notNull(),
    /** Provider-specific permission levels granted to the application. */
    permissions: json("permissions", schema.record(schema.string(), schema.string())).notNull(),
    /** The space administering the OAuth credential vault. */
    secretSpaceId: identifier("secret_space_id", "space"),
    /** The OAuth credential; installation tokens are issued using platform credentials. */
    secretId: identifier("secret_id", "secret"),
    /** The credential expiration time, when applicable. */
    expiresAt: integer("expires_at"),
    /** The time authorisation was revoked. */
    revokedAt: integer("revoked_at"),
}, (connectedAccount) => [
    foreignKey({
        columns: [connectedAccount.accountId, connectedAccount.secretSpaceId],
        foreignColumns: [spaceDirectory.accountId, spaceDirectory.id],
    }).onDelete("restrict"),
    unique("connection_account_id").on(connectedAccount.accountId, connectedAccount.id),
    uniqueIndex("connection_oauth").on(
        connectedAccount.accountId,
        connectedAccount.provider,
        connectedAccount.issuer,
        connectedAccount.applicationId,
        connectedAccount.subject,
    ).where(sql`${connectedAccount.kind} = 'oauth'`),
    uniqueIndex("connection_installation").on(
        connectedAccount.accountId,
        connectedAccount.provider,
        connectedAccount.issuer,
        connectedAccount.applicationId,
        connectedAccount.installationId,
    ).where(sql`${connectedAccount.kind} = 'installation'`),
    check(
        "connection_authorisation",
        sql`(${connectedAccount.kind} = 'oauth' AND ${connectedAccount.installationId} IS NULL AND ${connectedAccount.secretSpaceId} IS NOT NULL AND ${connectedAccount.secretId} IS NOT NULL)
            OR (${connectedAccount.kind} = 'installation' AND ${connectedAccount.installationId} IS NOT NULL AND ${connectedAccount.secretSpaceId} IS NULL AND ${connectedAccount.secretId} IS NULL)`,
    ),
    check(
        "connection_identifiers",
        sql`length(${connectedAccount.provider}) > 0 AND length(${connectedAccount.issuer}) > 0
            AND length(${connectedAccount.subject}) > 0 AND length(${connectedAccount.applicationId}) > 0
            AND (${connectedAccount.installationId} IS NULL OR length(${connectedAccount.installationId}) > 0)`,
    ),
    index("connection_user").on(connectedAccount.userId),
]);

/** An authorised external account. */
export type ConnectedAccount = Select<typeof connectedAccount>;
