import { identifier, index, integer, json, type Select, table, text } from "@destack/db";
import { schema } from "@destack/schema";
import { user } from "../../account/user.ts";
import { session } from "../session.ts";
import { oauthClient } from "./client.ts";

/** An OAuth refresh-token family member. */
export const oauthRefreshToken = table(
    "oauth_refresh_token",
    {
        /** The token record identifier. */
        id: identifier("id", "oauth-refresh-token").primaryKey(),
        /** The token digest stored by the OAuth provider. */
        token: text("token").notNull().unique(),
        /** The client receiving the token. */
        clientId: text("client_id")
            .notNull()
            .references(() => oauthClient.clientId, { onDelete: "cascade" }),
        /** The browser session authorizing the grant. */
        sessionId: identifier("session_id", "session").references(() => session.id, {
            onDelete: "set null",
        }),
        /** The user authorizing the grant. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, { onDelete: "cascade" }),
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
        confirmation: json("confirmation", schema.record(schema.string(), schema.json())),
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
    (token) => [
        index("oauth_refresh_client").on(token.clientId),
        index("oauth_refresh_user").on(token.userId),
        index("oauth_refresh_session").on(token.sessionId),
        index("oauth_refresh_code").on(token.authorizationCodeId),
        index("oauth_refresh_expiry").on(token.expiresAt),
    ],
);

/** A persisted OAuth refresh token record. */
export type OAuthRefreshToken = Select<typeof oauthRefreshToken>;

/** An opaque OAuth access token. */
export const oauthAccessToken = table(
    "oauth_access_token",
    {
        /** The token record identifier. */
        id: identifier("id", "oauth-access-token").primaryKey(),
        /** The token digest stored by the OAuth provider. */
        token: text("token").notNull().unique(),
        /** The client receiving the token. */
        clientId: text("client_id")
            .notNull()
            .references(() => oauthClient.clientId, { onDelete: "cascade" }),
        /** The browser session authorizing the grant. */
        sessionId: identifier("session_id", "session").references(() => session.id, {
            onDelete: "set null",
        }),
        /** The user authorizing the grant. */
        userId: identifier("user_id", "user").references(() => user.id, { onDelete: "cascade" }),
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
        confirmation: json("confirmation", schema.record(schema.string(), schema.json())),
        /** The granted OAuth scopes. */
        scopes: json("scopes", schema.array(schema.string())).notNull(),
        /** The refresh token that issued this access token. */
        refreshId: identifier("refresh_id", "oauth-refresh-token").references(
            () => oauthRefreshToken.id,
            { onDelete: "cascade" },
        ),
    },
    (token) => [
        index("oauth_access_client").on(token.clientId),
        index("oauth_access_user").on(token.userId),
        index("oauth_access_session").on(token.sessionId),
        index("oauth_access_refresh").on(token.refreshId),
        index("oauth_access_code").on(token.authorizationCodeId),
        index("oauth_access_expiry").on(token.expiresAt),
    ],
);

/** A persisted OAuth access token record. */
export type OAuthAccessToken = Select<typeof oauthAccessToken>;
