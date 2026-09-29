import { and, check, index, isNull, ne, sql, unique, type Select } from "@destack/db";
import { type Call, defineObject, field, method } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { device } from "./device.ts";
import { user } from "./user.ts";
import { revokeOnce } from "./revocation.ts";

/** The verified methods of an authentication ceremony. */
export const AUTHENTICATION_METHODS = [
    "magic-link",
    "email-otp",
    "oauth",
    "device",
    "totp",
    "webauthn",
    "recovery",
] as const;

/** A user's signed-in session. */
export const session = defineObject({
    name: "session",
    tier: "global",
    plural: "sessions",
    scope: user,
    fields: {
        /** The authenticated user. */
        userId: field.reference(user, { delete: "cascade" }),
        /** The registered device of the session; revoking it ends the session. */
        deviceId: field.reference<"device">(() => device).optional(),
        /** The session credential stored by Better Auth. */
        token: field.string().sensitive(),
        /** The client IP observed through trusted ingress. */
        ipAddress: field.string().optional(),
        /** The reported client user agent. */
        userAgent: field.string().optional(),
        /** The approximate country inferred from the observed address. */
        country: field.string().optional(),
        /** The approximate city inferred from the observed address. */
        city: field.string().optional(),
        /** The last completed authentication ceremony. */
        authenticatedAt: field.time().optional(),
        /** The verified method used in the last authentication ceremony. */
        authenticationMethod: field.enum(AUTHENTICATION_METHODS).optional(),
        /** The session expiry time. */
        expiresAt: field.time(),
        /** The time access was revoked. */
        revokedAt: field.time().optional(),
    },
    constraints: (entry) => [
        unique("session_token").on(entry.token),
        index("session_user").on(entry.userId),
        index("session_device").on(entry.deviceId),
        index("session_expiry").on(entry.expiresAt),
        check("session_expiry_order", sql`${entry.expiresAt} > ${entry.createdAt}`),
        check(
            "session_authentication_time",
            sql`${entry.authenticationMethod} IS NULL OR ${entry.authenticatedAt} IS NOT NULL`,
        ),
    ],
    permissions: ["read", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        revoke: method({ permission: "revoke" }).handle((call) => revokeOnce(call, "session")),
        revokeOthers: method({ permission: "revoke" }).handle(revokeOthers),
        attach: method
            .update(null, { isSystem: true, fields: ["deviceId"] })
            .handle(async (call, next) => {
                // refuse attaching a revoked session; the user's scope holds only their own sessions
                if (call.target!.revokedAt !== null) {
                    throw new ServiceError("FORBIDDEN", { message: "the session is revoked" });
                }

                return next();
            }),
    },
});
/** A persisted session record. */
export type Session = Select<typeof session.table>;

/** A sign-in provider account a user linked. */
export const identity = defineObject({
    name: "identity",
    tier: "global",
    plural: "identities",
    scope: user,
    fields: {
        /** The authenticated Destack user. */
        userId: field.reference(user, { delete: "cascade" }),
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
    constraints: (entry) => [
        unique("identity_provider_user").on(entry.providerId, entry.providerUserId),
        index("identity_user").on(entry.userId),
    ],
    permissions: ["read", "list"],
    methods: { get: method.get("read"), list: method.list("list") },
});
/** A persisted sign-in provider account. */
export type Identity = Select<typeof identity.table>;

/** A WebAuthn credential a user registered. */
export const passkey = defineObject({
    name: "passkey",
    tier: "global",
    plural: "passkeys",
    scope: user,
    fields: {
        /** The user-selected credential name. */
        name: field.string().optional(),
        /** The encoded WebAuthn public key. */
        publicKey: field.string(),
        /** The user authenticated by this credential. */
        userId: field.reference(user, { delete: "cascade" }),
        /** The authenticator-assigned credential identifier. */
        credentialID: field.string(),
        /** The latest authenticator signature counter. */
        counter: field.integer(),
        /** The WebAuthn single-device or multi-device credential type. */
        deviceType: field.string(),
        /** Whether the credential is backed up by its authenticator. */
        backedUp: field.boolean(),
        /** The encoded authenticator transports. */
        transports: field.string().optional(),
        /** The authenticator model identifier. */
        aaguid: field.string().optional(),
    },
    constraints: (entry) => [
        unique("passkey_credential").on(entry.credentialID),
        index("passkey_user").on(entry.userId),
    ],
    permissions: ["read", "list"],
    methods: { get: method.get("read"), list: method.list("list") },
});
/** A persisted WebAuthn credential. */
export type Passkey = Select<typeof passkey.table>;

/** A user's encrypted TOTP secret and recovery codes. */
export const twoFactor = defineObject({
    name: "two-factor",
    tier: "global",
    plural: "twoFactors",
    scope: user,
    fields: {
        /** The encrypted TOTP secret. */
        secret: field.string().sensitive(),
        /** The encrypted recovery codes in Better Auth's format. */
        backupCodes: field.string().sensitive(),
        /** The user required to complete the second factor. */
        userId: field.reference(user, { delete: "cascade" }),
        /** Whether enrollment has been confirmed. */
        verified: field.boolean().default(true),
        /** The consecutive failed verification count. */
        failedVerificationCount: field.integer().default(0),
        /** The lockout deadline in epoch milliseconds. */
        lockedUntil: field.time().optional(),
    },
    constraints: (entry) => [
        index("two_factor_user").on(entry.userId),
        index("two_factor_secret").on(entry.secret),
    ],
    permissions: ["read"],
    methods: { get: method.get("read") },
});
/** A persisted second-factor enrollment. */
export type TwoFactor = Select<typeof twoFactor.table>;

/** End every active session of the target's user but the target. */
async function revokeOthers(call: Call): Promise<unknown> {
    // read the other active sessions
    const table = session.table;
    const others = await call.database
        .select({ id: table.id })
        .from(table)
        .where(
            and(
                session.inScope(call.scope),
                ne(table.id, call.id as Session["id"]),
                isNull(table.revokedAt),
            ),
        );

    // revoke each
    for (const other of others) {
        await call.invoke(session, "revoke", { id: other.id });
    }

    return call.target;
}
