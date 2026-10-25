import { principal } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Digest, schema } from "@destack/schema";
import { account } from "./account.ts";
import { user } from "./user.ts";
import { DisplayName } from "./display-name.ts";

/** The grants a client may use. */
export const OAUTH_GRANT_TYPES = [
    "authorization_code",
    "refresh_token",
    "urn:ietf:params:oauth:grant-type:device_code",
] as const;

/** The scopes the provider supports, which a client registers when it names none (RFC 7591 2). */
export const OAUTH_SCOPES = ["openid", "profile", "email", "offline_access"] as const;

/** How a client authenticates at the token endpoint. */
export const OAUTH_CLIENT_AUTHENTICATIONS = [
    "none",
    "client_secret_basic",
    "client_secret_post",
] as const;

/** An application signing its users in with Destack. */
export const application = defineObject(
    {
        name: "application",
        plural: "applications",
        scope: [account, user],
        fields: {
            /** The application's public OAuth client identifier. */
            clientId: field.string(),
            /** The user registering the client. */
            user: field.reference(principal.user).caller(),
            /** The displayed client name. */
            name: field.string(DisplayName),
            /** The client's homepage URL (RFC 7591 client_uri). */
            clientUri: field.string(schema.httpUrl()).optional(),
            /** The client's logo URL (RFC 7591 logo_uri). */
            logoUri: field.string(schema.httpUrl()).optional(),
            /** The client's terms of service URL (RFC 7591 tos_uri). */
            tosUri: field.string(schema.httpUrl()).optional(),
            /** The client's privacy policy URL (RFC 7591 policy_uri). */
            policyUri: field.string(schema.httpUrl()).optional(),
            /** The registered authorization redirect URLs. */
            redirectUris: field.json(schema.array(schema.url()).min(1)),
            /** How the client authenticates at the token endpoint. */
            tokenEndpointAuthMethod: field.enum(OAUTH_CLIENT_AUTHENTICATIONS),
            /** The SHA-256 digest of a confidential client's secret. */
            clientSecret: field.string(Digest).sensitive().optional(),
            /** The grants the client may use. */
            grantTypes: field.json(schema.array(schema.enum(OAUTH_GRANT_TYPES)).min(1)),
            /** The scopes the client may request, every supported one unless it names some. */
            scopes: field.json(schema.array(schema.enum(OAUTH_SCOPES))).default([...OAUTH_SCOPES]),
            /** Whether new authorizations of the client are disabled. */
            isDisabled: field.boolean().default(false),
        },
        permissions: ["read", "create", "update", "delete"],
        constraints: {
            clientId: { kind: "unique", on: ["clientId"] },
            scope: { kind: "index", on: ["scope"] },
            secret: {
                kind: "check",
                condition: {
                    OR: [
                        { tokenEndpointAuthMethod: "none", clientSecret: { isNull: true } },
                        {
                            tokenEndpointAuthMethod: { ne: "none" },
                            clientSecret: { isNotNull: true },
                        },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("create", {
            fields: [
                "name",
                "clientUri",
                "logoUri",
                "tosUri",
                "policyUri",
                "redirectUris",
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
                "clientUri",
                "logoUri",
                "tosUri",
                "policyUri",
                "redirectUris",
                "clientSecret",
                "grantTypes",
                "scopes",
                "isDisabled",
            ],
        }),
        delete: method.delete("delete"),
    }),
);
/** A persisted application. */
export type Application = Select<typeof application.table>;
