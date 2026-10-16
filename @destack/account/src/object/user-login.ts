import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { user } from "./user.ts";

/** A way a user signs in: their account at a sign-in provider, or their email and password. */
export const userLogin = defineObject(
    {
        name: "user-login",
        plural: "userLogins",
        scope: user,
        fields: {
            /** The authenticated Destack user. */
            user: field.reference(user, { delete: "cascade" }),
            /** The configured authentication provider. */
            providerId: field.string(),
            /** The provider's user identifier. */
            providerUserId: field.string(),
            /** The encrypted provider access token. */
            accessToken: field.string().sensitive().optional(),
            /** The encrypted provider refresh token. */
            refreshToken: field.string().sensitive().optional(),
            /** The provider-issued signed ID token. */
            idToken: field.string().sensitive().optional(),
            /** The access-token expiry in epoch milliseconds. */
            accessTokenExpiresAt: field.time().optional(),
            /** The refresh-token expiry in epoch milliseconds. */
            refreshTokenExpiresAt: field.time().optional(),
            /** The space-separated provider scopes. */
            providerScope: field.string().optional(),
            /** The password hash credential authentication checks. */
            password: field.string().sensitive().optional(),
        },
        permissions: ["read", "list"],
        constraints: {
            providerUser: { kind: "unique", on: ["providerId", "providerUserId"] },
            user: { kind: "index", on: ["userId"] },
        },
    },
    (method) => ({ list: method.list("list") }),
);
/** A persisted way a user signs in. */
export type UserLogin = Select<typeof userLogin.table>;
