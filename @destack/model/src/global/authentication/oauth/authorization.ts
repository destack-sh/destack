import {
    boolean,
    identifier,
    index,
    integer,
    json,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { schema } from "@destack/schema";
import { user } from "../../account/user.ts";
import { oauthClient } from "./client.ts";

/** An OAuth protected API and its token policy. */
export const oauthResource = table("oauth_resource", {
    /** The protected API record identifier. */
    id: identifier("id", "oauth-resource").primaryKey(),
    /** The audience URL used by OAuth resource indicators. */
    identifier: text("identifier").notNull().unique(),
    /** The displayed API name. */
    name: text("name").notNull(),
    /** The access-token lifetime in seconds. */
    accessTokenTtl: integer("access_token_ttl"),
    /** The refresh-token lifetime in seconds. */
    refreshTokenTtl: integer("refresh_token_ttl"),
    /** The token signing algorithm override. */
    signingAlgorithm: text("signing_algorithm"),
    /** The signing key identifier override. */
    signingKeyId: text("signing_key_id"),
    /** The permitted OAuth scopes. */
    allowedScopes: json("allowed_scopes", schema.array(schema.string())),
    /** Additional claims accepted by the OAuth provider. */
    customClaims: json("custom_claims", schema.record(schema.string(), schema.json())),
    /** Whether callers must prove possession of their token's key. */
    dpopBoundAccessTokensRequired: boolean("dpop_bound_access_tokens_required").default(false),
    /** Whether issuance for this audience is disabled. */
    disabled: boolean("disabled").default(false),
    /** The creation time in epoch milliseconds. */
    createdAt: integer("created_at"),
    /** The update time in epoch milliseconds. */
    updatedAt: integer("updated_at"),
    /** The OAuth provider's policy format version. */
    policyVersion: integer("policy_version").default(1),
    /** Additional protected API metadata. */
    metadata: json("metadata", schema.record(schema.string(), schema.json())),
});

/** Permit a client to request tokens for one OAuth audience. */
export const oauthClientResource = table(
    "oauth_client_resource",
    {
        /** The grant identifier. */
        id: identifier("id", "oauth-client-resource").primaryKey(),
        /** The permitted client. */
        clientId: text("client_id")
            .notNull()
            .references(() => oauthClient.clientId, { onDelete: "cascade" }),
        /** The protected API's audience URL. */
        resourceId: text("resource_id")
            .notNull()
            .references(() => oauthResource.identifier, { onDelete: "cascade" }),
        /** Additional grant metadata. */
        metadata: json("metadata", schema.record(schema.string(), schema.json())),
        /** The grant creation time in epoch milliseconds. */
        createdAt: integer("created_at"),
    },
    (grant) => [
        unique("oauth_client_resource_grant").on(grant.clientId, grant.resourceId),
        index("oauth_client_resource_resource").on(grant.resourceId),
    ],
);

/** A user's consent to an OAuth client's scopes and audiences. */
export const oauthConsent = table(
    "oauth_consent",
    {
        /** The consent identifier. */
        id: identifier("id", "oauth-consent").primaryKey(),
        /** The authorized client. */
        clientId: text("client_id")
            .notNull()
            .references(() => oauthClient.clientId, { onDelete: "cascade" }),
        /** The consenting user. */
        userId: identifier("user_id", "user").references(() => user.id, { onDelete: "cascade" }),
        /** The authorization provider's external subject reference. */
        referenceId: text("reference_id"),
        /** The approved OAuth audience URLs. */
        resources: json("resources", schema.array(schema.string())),
        /** The approved OpenID user claims. */
        requestedUserInfoClaims: json("requested_user_info_claims", schema.array(schema.string())),
        /** The approved OAuth scopes. */
        scopes: json("scopes", schema.array(schema.string())).notNull(),
        /** The creation time in epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The update time in epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
    },
    (consent) => [
        index("oauth_consent_client").on(consent.clientId),
        index("oauth_consent_user").on(consent.userId),
    ],
);

/** A consumed private-key JWT assertion retained until its expiry. */
export const oauthClientAssertion = table(
    "oauth_client_assertion",
    {
        /** The OAuth provider's digest of the client and assertion identifier. */
        id: text("id").primaryKey(),
        /** The earliest safe deletion time in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    (assertion) => [index("oauth_client_assertion_expiry").on(assertion.expiresAt)],
);

/** A persisted protected API. */
export type OAuthResource = Select<typeof oauthResource>;
/** A persisted client audience grant. */
export type OAuthClientResource = Select<typeof oauthClientResource>;
/** A persisted user consent. */
export type OAuthConsent = Select<typeof oauthConsent>;
/** A persisted consumed client assertion. */
export type OAuthClientAssertion = Select<typeof oauthClientAssertion>;
