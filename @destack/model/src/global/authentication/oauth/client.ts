import {
    boolean,
    check,
    foreignKey,
    identifier,
    index,
    integer,
    json,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { schema } from "@destack/schema";
import { user } from "../../account/user.ts";
import { account } from "../../account/account.ts";
import { serviceAccount } from "../../account/service.ts";

/** An OAuth client registration. */
export const oauthClient = table(
    "oauth_client",
    {
        /** The registration identifier. */
        id: identifier("id", "oauth-client").primaryKey(),
        /** The public OAuth client identifier. */
        clientId: text("client_id").notNull().unique(),
        /** The account administering an account-registered client. */
        accountId: identifier("account_id", "account").references(() => account.id, {
            onDelete: "cascade",
        }),
        /** The software identity authorizing client-credentials grants. */
        serviceAccountId: identifier("service_account_id", "service-account"),

        /** The protected confidential-client credential. */
        clientSecret: text("client_secret"),
        /** The client metadata discovery identifier. */
        clientDiscoveryId: text("client_discovery_id"),

        /** The OpenID subject identifier type. */
        subjectType: text("subject_type"),
        /** The displayed client name. */
        name: text("name"),
        /** The client homepage URL. */
        uri: text("uri"),
        /** The client icon URL. */
        icon: text("icon"),
        /** The client terms-of-service URL. */
        tos: text("tos"),
        /** The client privacy-policy URL. */
        policy: text("policy"),
        /** The software identifier. */
        softwareId: text("software_id"),
        /** The software version. */
        softwareVersion: text("software_version"),
        /** The signed software statement. */
        softwareStatement: text("software_statement"),

        /** The back-channel logout endpoint. */
        backchannelLogoutUri: text("backchannel_logout_uri"),
        /** The token-endpoint authentication method. */
        tokenEndpointAuthMethod: text("token_endpoint_auth_method"),
        /** The native or web application type. */
        applicationType: text("application_type"),
        /** The serialized client public key set. */
        jwks: text("jwks"),
        /** The client public key set URL. */
        jwksUri: text("jwks_uri"),
        /** The authorization provider's external subject reference. */
        referenceId: text("reference_id"),

        /** Whether new client authorizations are disabled. */
        disabled: boolean("disabled"),
        /** Whether this trusted client bypasses the consent prompt. */
        skipConsent: boolean("skip_consent"),
        /** Whether the client supports ending sessions. */
        enableEndSession: boolean("enable_end_session"),
        /** Whether logout requires the session identifier. */
        backchannelLogoutSessionRequired: boolean("backchannel_logout_session_required"),
        /** Whether authorization-code exchange requires PKCE. */
        requirePKCE: boolean("require_pkce"),
        /** Whether issued tokens require DPoP. */
        dpopBoundAccessTokens: boolean("dpop_bound_access_tokens"),

        /** The scopes the client may request. */
        scopes: json("scopes", schema.array(schema.string())),
        /** The scopes available to client-credentials grants. */
        clientCredentialsScopes: json("client_credentials_scopes", schema.array(schema.string())),
        /** The client administrator contacts. */
        contacts: json("contacts", schema.array(schema.string())),
        /** The registered authorization redirect URLs. */
        redirectUris: json("redirect_uris", schema.array(schema.string())).notNull(),
        /** The registered logout redirect URLs. */
        postLogoutRedirectUris: json("post_logout_redirect_uris", schema.array(schema.string())),
        /** The enabled OAuth grant types. */
        grantTypes: json("grant_types", schema.array(schema.string())),
        /** The enabled authorization response types. */
        responseTypes: json("response_types", schema.array(schema.string())),

        /** The user registering the client. */
        userId: identifier("user_id", "user").references(() => user.id, { onDelete: "restrict" }),
        /** The registration time in epoch milliseconds. */
        createdAt: integer("created_at"),
        /** The last update in epoch milliseconds. */
        updatedAt: integer("updated_at"),
        /** Additional OAuth client registration metadata. */
        metadata: json("metadata", schema.record(schema.string(), schema.json())),
    },
    (client) => [
        index("oauth_client_user").on(client.userId),
        index("oauth_client_account").on(client.accountId),
        foreignKey({
            columns: [client.accountId, client.serviceAccountId],
            foreignColumns: [serviceAccount.accountId, serviceAccount.id],
        }).onDelete("cascade"),
        check(
            "oauth_client_service_account",
            sql`${client.serviceAccountId} IS NULL OR ${client.accountId} IS NOT NULL`,
        ),
    ],
);

/** A persisted OAuth client record. */
export type OAuthClient = Select<typeof oauthClient>;
