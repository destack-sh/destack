import {
    check,
    defineDatabase,
    defineTable,
    identifier,
    index,
    integer,
    json,
    type Select,
    text,
    textEnum,
} from "@destack/db";
import { call, journal } from "@destack/object";
import { schema } from "@destack/schema";
import { setting } from "@destack/setting/object";
import { identityKey } from "../identity/index.ts";
import { session } from "../object/session.ts";
import { user } from "../object/user.ts";
import accountPackage from "../package.ts";

/** An expiring authentication challenge stored by the universe. */
export const verification = defineTable(
    "verification",
    {
        /** The challenge's key. */
        id: text("id").primaryKey(),
        /** The challenge purpose and subject. */
        identifier: text("identifier").notNull(),
        /** The protected challenge value in the authentication provider's format. */
        value: text("value").notNull(),
        /** The expiry in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The creation time in epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The last update in epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
    },
    {
        constraints: (challenge) => [
            index("verification_identifier").on(challenge.identifier),
            index("verification_expiry").on(challenge.expiresAt),
        ],
    },
);

/** A shared authentication rate-limit counter. */
export const rateLimit = defineTable(
    "rate_limit",
    {
        /** The counter identifier. */
        id: identifier("id", "rate-limit").primaryKey(),
        /** The request bucket selected by Better Auth. */
        key: text("key").notNull().unique(),
        /** The number of requests in the current window. */
        count: integer("count").notNull(),
        /** The most recent request in epoch milliseconds. */
        lastRequest: integer("last_request").notNull(),
    },
    {
        constraints: (limit) => [index("rate_limit_last_request").on(limit.lastRequest)],
    },
);

/** A persisted authentication challenge. */
export type Verification = Select<typeof verification>;
/** A persisted authentication rate-limit counter. */
export type RateLimit = Select<typeof rateLimit>;

/** A browser-approved device authorization grant. */
export const deviceAuthorization = defineTable(
    "device_authorization",
    {
        /** The authorization identifier. */
        id: identifier("id", "device-authorization").primaryKey(),
        /** The credential used by the initiating client to poll for approval. */
        deviceCode: text("device_code").notNull().unique(),
        /** The code confirmed by the user in the approving browser. */
        userCode: text("user_code").notNull().unique(),
        /** The approving user. */
        userId: identifier("user_id", "user").references(() => user.table.id, {
            onDelete: "cascade",
        }),
        /** The authorization deadline in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The browser approval state. */
        status: textEnum("status", ["pending", "approved", "denied"]).notNull(),
        /** The most recent token poll in epoch milliseconds. */
        lastPolledAt: integer("last_polled_at"),
        /** The minimum polling interval in milliseconds. */
        pollingInterval: integer("polling_interval"),
        /** The requesting client. */
        clientId: text("client_id"),
        /** The client receiving tokens from the approved grant: registered, or a device's. */
        oauthClientId: text("oauth_client_id"),
        /** The OAuth audience URLs approved with the device grant. */
        resources: json("resources", schema.array(schema.string())),
        /** The requested space-separated OAuth scopes. */
        scope: text("scope"),
    },
    {
        constraints: (authorization) => [
            index("device_authorization_expiry").on(authorization.expiresAt),
            index("device_authorization_user").on(authorization.userId),
            check("device_authorization_status", {
                status: { in: ["pending", "approved", "denied"] },
            }),
        ],
    },
);

/** A persisted device authorization grant. */
export type DeviceAuthorization = Select<typeof deviceAuthorization>;

/** A consumed private-key JWT assertion retained until its expiry. */
export const oauthClientAssertion = defineTable(
    "oauth_client_assertion",
    {
        /** The OAuth provider's digest of the client and assertion identifier. */
        id: text("id").primaryKey(),
        /** The earliest safe deletion time in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        constraints: (assertion) => [
            index("oauth_client_assertion_expiry").on(assertion.expiresAt),
        ],
    },
);

/** A persisted consumed client assertion. */
export type OAuthClientAssertion = Select<typeof oauthClientAssertion>;

/** An OAuth refresh-token family member. */
export const oauthRefreshToken = defineTable(
    "oauth_refresh_token",
    {
        /** The token record identifier. */
        id: identifier("id", "oauth-refresh-token").primaryKey(),
        /** The token digest stored by the OAuth provider. */
        token: text("token").notNull().unique(),
        /** The client receiving the token: a registered OAuth client or a device's client. */
        clientId: text("client_id").notNull(),
        /** The browser session authorizing the grant. */
        sessionId: identifier("session_id", "session").references(() => session.table.id, {
            onDelete: "set null",
        }),
        /** The user authorizing the grant. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.table.id, { onDelete: "cascade" }),
        /** The authorization provider's external subject reference. */
        referenceId: text("reference_id"),
        /** The authorization-code family used for revocation. */
        authorizationCodeId: text("authorization_code_id"),
        /** The granted OAuth audience URLs. */
        resources: json("resources", schema.array(schema.string())),
        /** The requested OpenID user claims. */
        requestedUserInfoClaims: json("requested_user_info_claims", schema.array(schema.string())),
        /** The token expiry in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The token creation time in epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The revocation time in epoch milliseconds. */
        revoked: integer("revoked"),
        /** The sender-constraining confirmation claim. */
        confirmation: json("confirmation", schema.jsonObject()),
        /** The granted OAuth scopes. */
        scopes: json("scopes", schema.array(schema.string())).notNull(),
        /** The token rotation time in epoch milliseconds. */
        rotatedAt: integer("rotated_at"),
        /** The protected response retained for bounded rotation retries. */
        rotationReplayResponse: text("rotation_replay_response"),
        /** The rotation retry deadline in epoch milliseconds. */
        rotationReplayExpiresAt: integer("rotation_replay_expires_at"),
        /** The original user authentication time in epoch milliseconds. */
        authTime: integer("auth_time"),
    },
    {
        constraints: (token) => [
            index("oauth_refresh_client").on(token.clientId),
            index("oauth_refresh_user").on(token.userId),
            index("oauth_refresh_session").on(token.sessionId),
            index("oauth_refresh_code").on(token.authorizationCodeId),
            index("oauth_refresh_expiry").on(token.expiresAt),
        ],
    },
);

/** A persisted OAuth refresh token record. */
export type OAuthRefreshToken = Select<typeof oauthRefreshToken>;

/** An opaque OAuth access token. */
export const oauthAccessToken = defineTable(
    "oauth_access_token",
    {
        /** The token record identifier. */
        id: identifier("id", "oauth-access-token").primaryKey(),
        /** The token digest stored by the OAuth provider. */
        token: text("token").notNull().unique(),
        /** The client receiving the token: a registered OAuth client or a device's client. */
        clientId: text("client_id").notNull(),
        /** The browser session authorizing the grant. */
        sessionId: identifier("session_id", "session").references(() => session.table.id, {
            onDelete: "set null",
        }),
        /** The user authorizing the grant. */
        userId: identifier("user_id", "user").references(() => user.table.id, {
            onDelete: "cascade",
        }),
        /** The authorization provider's external subject reference. */
        referenceId: text("reference_id"),
        /** The authorization-code family used for revocation. */
        authorizationCodeId: text("authorization_code_id"),
        /** The granted OAuth audience URLs. */
        resources: json("resources", schema.array(schema.string())),
        /** The requested OpenID user claims. */
        requestedUserInfoClaims: json("requested_user_info_claims", schema.array(schema.string())),
        /** The token expiry in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The token creation time in epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The revocation time in epoch milliseconds. */
        revoked: integer("revoked"),
        /** The sender-constraining confirmation claim. */
        confirmation: json("confirmation", schema.jsonObject()),
        /** The granted OAuth scopes. */
        scopes: json("scopes", schema.array(schema.string())).notNull(),
        /** The refresh token that issued this access token. */
        refreshId: identifier("refresh_id", "oauth-refresh-token").references(
            () => oauthRefreshToken.id,
            { onDelete: "cascade" },
        ),
    },
    {
        constraints: (token) => [
            index("oauth_access_client").on(token.clientId),
            index("oauth_access_user").on(token.userId),
            index("oauth_access_session").on(token.sessionId),
            index("oauth_access_refresh").on(token.refreshId),
            index("oauth_access_code").on(token.authorizationCodeId),
            index("oauth_access_expiry").on(token.expiresAt),
        ],
    },
);

/** A persisted OAuth access token record. */
export type OAuthAccessToken = Select<typeof oauthAccessToken>;

/** The universe's database. */
export const accountDatabase = defineDatabase({
    name: "account",
    tables: [
        // the package's objects
        ...Object.values(accountPackage.objects).flatMap((type) => type.tables),

        // the authentication protocols' records
        verification,
        rateLimit,
        deviceAuthorization,
        oauthClientAssertion,
        oauthRefreshToken,
        oauthAccessToken,

        // the universe's identity keys, settings, journal and calls
        identityKey,
        ...setting.tables,
        journal,
        ...call.tables,
    ],
});
