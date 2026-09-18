import {
    identifier,
    index,
    integer,
    json,
    recordColumns,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { schema } from "@destack/schema";
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
    /** The provider's stable account identifier. */
    subject: text("subject").notNull(),
    /** The authorised provider scopes. */
    scopes: json("scopes", schema.array(schema.string())).notNull(),
    /** The credential reference in the administering service's secret store. */
    secret: text("secret").notNull(),
    /** The credential expiration time, when applicable. */
    expiresAt: integer("expires_at"),
    /** The time authorisation was revoked. */
    revokedAt: integer("revoked_at"),
}, (connectedAccount) => [
    unique("connection_account_provider_subject").on(
        connectedAccount.accountId,
        connectedAccount.provider,
        connectedAccount.subject,
    ),
    index("connection_user").on(connectedAccount.userId),
]);

/** An authorised external account. */
export type ConnectedAccount = Select<typeof connectedAccount>;
