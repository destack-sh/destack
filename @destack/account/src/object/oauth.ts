import { principal } from "@destack/access";
import { check, foreignKey, index, sql, unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { Digest, schema } from "@destack/schema";
import { account } from "./account.ts";
import { user } from "./user.ts";

/** The grants a client may use. */
export const OAUTH_GRANT_TYPES = [
    "authorization_code",
    "refresh_token",
    "urn:ietf:params:oauth:grant-type:device_code",
] as const;

/** How a client authenticates at the token endpoint. */
export const OAUTH_CLIENT_AUTHENTICATIONS = [
    "none",
    "client_secret_basic",
    "client_secret_post",
] as const;

/** An application signing its users in with Destack. */
export const oauthClient = defineObject({
    name: "oauth-client",
    tier: "global",
    plural: "oauthClients",
    scope: [account, user],
    fields: {
        // the registration a registrar writes
        /** The public OAuth client identifier. */
        clientId: field.string(),
        /** The user registering the client. */
        userId: field.reference(principal.user).caller(),
        /** The displayed client name. */
        name: field.string(schema.string().min(1).max(200)),
        /** The client homepage URL. */
        uri: field.string(schema.httpUrl()).optional(),
        /** The client icon URL. */
        icon: field.string(schema.httpUrl()).optional(),
        /** The client terms-of-service URL. */
        tos: field.string(schema.httpUrl()).optional(),
        /** The client privacy-policy URL. */
        policy: field.string(schema.httpUrl()).optional(),
        /** The registered authorization redirect URLs. */
        redirectUris: field.json(schema.array(schema.url()).min(1)),
        /** The registered logout redirect URLs. */
        postLogoutRedirectUris: field.json(schema.array(schema.url())).optional(),
        /** How the client authenticates at the token endpoint. */
        tokenEndpointAuthMethod: field.enum(OAUTH_CLIENT_AUTHENTICATIONS),
        /** The SHA-256 digest of a confidential client's secret. */
        clientSecret: field.string(Digest).sensitive().optional(),
        /** The grants the client may use. */
        grantTypes: field.json(schema.array(schema.enum(OAUTH_GRANT_TYPES)).min(1)),
        /** The scopes the client may request, every provider scope when absent. */
        scopes: field.json(schema.array(schema.string())).optional(),
        /** Whether new authorizations of the client are disabled. */
        disabled: field.boolean().default(false),

        // the provider's further client metadata, unset at registration
        /** The client metadata discovery identifier. */
        clientDiscoveryId: field.string().optional(),
        /** The OpenID subject identifier type. */
        subjectType: field.string().optional(),
        /** The software identifier. */
        softwareId: field.string().optional(),
        /** The software version. */
        softwareVersion: field.string().optional(),
        /** The signed software statement. */
        softwareStatement: field.string().optional(),
        /** The back-channel logout endpoint. */
        backchannelLogoutUri: field.string().optional(),
        /** Whether logout requires the session identifier. */
        backchannelLogoutSessionRequired: field.boolean().optional(),
        /** The native or web application type. */
        applicationType: field.string().optional(),
        /** The serialized client public key set. */
        jwks: field.string().optional(),
        /** The client public key set URL. */
        jwksUri: field.string().optional(),
        /** The authorization provider's external subject reference. */
        referenceId: field.string().optional(),
        /** Whether this trusted client bypasses the consent prompt. */
        skipConsent: field.boolean().optional(),
        /** Whether the client supports ending sessions. */
        enableEndSession: field.boolean().optional(),
        /** Whether code exchange requires PKCE. */
        requirePKCE: field.boolean().optional(),
        /** Whether issued tokens require DPoP. */
        dpopBoundAccessTokens: field.boolean().optional(),
        /** The scopes of client-credentials grants. */
        clientCredentialsScopes: field.json(schema.array(schema.string())).optional(),
        /** The client administrator contacts. */
        contacts: field.json(schema.array(schema.string())).optional(),
        /** The enabled authorization response types. */
        responseTypes: field.json(schema.array(schema.string())).optional(),
        /** Additional OAuth client registration metadata. */
        metadata: field.json(schema.record(schema.string(), schema.json())).optional(),
    },
    permissions: ["read", "create", "update", "delete"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", {
            fields: [
                "name",
                "uri",
                "icon",
                "tos",
                "policy",
                "redirectUris",
                "postLogoutRedirectUris",
                "tokenEndpointAuthMethod",
                "clientSecret",
                "grantTypes",
                "scopes",
            ],
            isPredicted: false,
        }),
        update: method.update("update", {
            fields: [
                "name",
                "uri",
                "icon",
                "tos",
                "policy",
                "redirectUris",
                "postLogoutRedirectUris",
                "clientSecret",
                "grantTypes",
                "scopes",
                "disabled",
            ],
        }),
        delete: method.delete("delete"),
    },
    constraints: (client) => [
        unique("oauth_client_client_id").on(client.clientId),
        index("oauth_client_scope").on(client.scope),
        check(
            "oauth_client_secret",
            sql`(${client.tokenEndpointAuthMethod} = 'none') = (${client.clientSecret} IS NULL)`,
        ),
    ],
});
/** A persisted OAuth client. */
export type OAuthClient = Select<typeof oauthClient.table>;

/** A user's consent to a client's scopes. */
export const oauthConsent = defineObject({
    name: "oauth-consent",
    tier: "global",
    plural: "oauthConsents",
    scope: user,
    fields: {
        /** The authorized client's public identifier, registered in any account or by any user. */
        clientId: field.string(),
        /** The consenting user. */
        userId: field.reference(user, { delete: "cascade" }),
        /** The authorization provider's external subject reference. */
        referenceId: field.string().optional(),
        /** The approved OAuth audience URLs. */
        resources: field.json(schema.array(schema.string())).optional(),
        /** The approved OpenID user claims. */
        requestedUserInfoClaims: field.json(schema.array(schema.string())).optional(),
        /** The approved OAuth scopes. */
        scopes: field.json(schema.array(schema.string())),
    },
    permissions: ["read", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        revoke: method.delete("revoke"),
    },
    constraints: (consent) => [
        foreignKey({
            columns: [consent.clientId],
            foreignColumns: [oauthClient.table.clientId],
        }).onDelete("cascade"),
        unique("oauth_consent_client_user").on(consent.clientId, consent.userId),
        index("oauth_consent_user").on(consent.userId),
    ],
});
/** A persisted OAuth consent. */
export type OAuthConsent = Select<typeof oauthConsent.table>;
