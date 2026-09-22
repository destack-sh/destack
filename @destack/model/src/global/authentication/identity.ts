import {
    identifier,
    index,
    integer,
    recordColumns,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { user } from "../account/user.ts";

/** Identity records. */
export const identity = table(
    "identity",
    {
        ...recordColumns("identity"),
        /** The authenticated Destack user. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, {
                onDelete: "cascade",
            }),
        /** The configured authentication provider. */
        providerId: text("provider_id").notNull(),
        /** The provider's user identifier. */
        accountId: text("account_id").notNull(),
        /** The encrypted provider access token. */
        accessToken: text("access_token"),
        /** The encrypted provider refresh token. */
        refreshToken: text("refresh_token"),
        /** The provider-issued signed ID token. */
        idToken: text("id_token"),
        /** The access-token expiry in epoch milliseconds. */
        accessTokenExpiresAt: integer("access_token_expires_at"),
        /** The refresh-token expiry in epoch milliseconds. */
        refreshTokenExpiresAt: integer("refresh_token_expires_at"),
        /** The space-separated provider scopes. */
        scope: text("scope"),
        /** The password hash used by credential authentication. */
        password: text("password"),
    },
    (identity) => [
        unique("identity_provider_account").on(identity.providerId, identity.accountId),
        index("identity_user").on(identity.userId),
    ],
);

/** A persisted identity record. */
export type Identity = Select<typeof identity>;
