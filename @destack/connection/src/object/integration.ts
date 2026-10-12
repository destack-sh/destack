import { contained, none, principal, resource, through, union } from "@destack/access";
import { VISIBILITIES } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Token } from "../credential/credential.ts";
import { DeviceRequest, TokenGrant, TokenRevocation } from "../oauth/grant.ts";
import { DeviceAuthorization } from "../oauth/server.ts";

/** How a client authenticates at a token endpoint, with the method names of RFC 7591. */
export const CLIENT_AUTHENTICATIONS = [
    "none",
    "client_secret_basic",
    "client_secret_post",
    "private_key_jwt",
] as const;

/** How a client authenticates at a token endpoint. */
export type ClientAuthentication = (typeof CLIENT_AUTHENTICATIONS)[number];

/** The endpoints of an authorization server (RFC 8414 2), and its installation page. */
export const AuthorizationEndpoints = defineSchema(
    schema.object({
        /** The authorization endpoint, for the authorization code grant. */
        authorization: schema.url({ protocol: /^https$/u }).exactOptional(),
        /** The token endpoint. */
        token: schema.url({ protocol: /^https$/u }).exactOptional(),
        /** The revocation endpoint (RFC 7009). */
        revocation: schema.url({ protocol: /^https$/u }).exactOptional(),
        /** The device authorization endpoint (RFC 8628). */
        deviceAuthorization: schema.url({ protocol: /^https$/u }).exactOptional(),
        /** The dynamic client registration endpoint (RFC 7591). */
        registration: schema.url({ protocol: /^https$/u }).exactOptional(),
        /** The page where a person installs the integration, for app installations. */
        installation: schema.url({ protocol: /^https$/u }).exactOptional(),
        /** Whether the server returns its issuer in authorization responses (RFC 9207). */
        isIssuerReturned: schema.boolean(),
    }),
);
/** The endpoints of an authorization server. */
export type AuthorizationEndpoints = schema.Infer<typeof AuthorizationEndpoints>;

/** A dynamically registered client (RFC 7591 3.2.1), with the vault of an issued secret. */
export const Registration = defineSchema(
    schema.object({
        /** The authorization server's issuer identifier. */
        issuer: schema.url({ protocol: /^https$/u }),
        /** The authorization server's endpoints. */
        endpoints: AuthorizationEndpoints,
        /** The client identifier the server assigned. */
        clientId: schema.string().min(1),
        /** How the client authenticates at the token endpoint. */
        authentication: schema.enum(CLIENT_AUTHENTICATIONS),
        /** The redirect URIs the client registered. */
        redirectUris: schema.array(schema.httpUrl()),
        /** The vault keeping the secret the server issued, absent for a public client. */
        vault: ObjectReference.exactOptional(),
    }),
);
/** A client an authorization server registered dynamically. */
export type Registration = schema.Infer<typeof Registration>;

/** An integration registered at an external authorization server, used by OAuth connections. */
export const integration = defineObject({
    name: "integration",
    plural: "integrations",
    // copy the integrations of platforms and organisations into their scopes and the spaces connecting through them
    inherited: {},
    fields: {
        /** The authorization server's issuer identifier (RFC 8414 2). */
        issuer: field.string(schema.url({ protocol: /^https$/u })),
        /** The authorization server's endpoints. */
        endpoints: field.json(AuthorizationEndpoints),
        /** The client identifier the server assigned. */
        clientId: field.string(schema.string().min(1)),
        /** How the client authenticates at the token endpoint. */
        authentication: field.enum(CLIENT_AUTHENTICATIONS),
        /** The redirect URIs the client registered. */
        redirectUris: field.json(schema.array(schema.httpUrl())),
        /** Whether the owner registered the client, or a connection did dynamically (RFC 7591). */
        registration: field.enum(["static", "dynamic"]).default("static"),
        /** The vault keeping the client secret or private key, absent for a public client. */
        vault: field.json(ObjectReference).optional(),
        /** Whether it serves its owner's spaces and the spaces below them, or every space. */
        visibility: field.enum(VISIBILITIES).default("private"),
    },
    attributes: { visibility: "string" },
    // let the spaces it serves copy it and request its tokens, and the servers of its scope's residency copy it
    permissions: {
        read: union(
            contained(principal.space),
            resource({ visibility: "public" }),
            through("scope", "serve"),
        ),
        manage: none(),
        request: union(contained(principal.space), resource({ visibility: "public" })),
    },
    reserved: ["request"],
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    /** Register an integration, keeping its client secret or private key in the vault. */
    create: method.create("manage", {
        fields: [
            "issuer",
            "endpoints",
            "clientId",
            "authentication",
            "redirectUris",
            "vault",
            "visibility",
        ],
        input: schema.object({
            /** The client secret or private key as JWK or PEM text, kept in the vault. */
            secret: schema.sensitive(schema.string().min(1)).exactOptional(),
        }),
        prepared: schema.object({}),
        isPredicted: false,
    }),
    update: method.update("manage", { fields: ["endpoints", "redirectUris"] }),
    /** Request a token for another space's connection as the client (RFC 6749 3.2). */
    requestToken: method.query({
        permission: "request",
        input: schema.object({
            /** The grant proving possession of the connection's authorization. */
            grant: schema.sensitive(TokenGrant),
        }),
        output: schema.sensitive(Token),
        prepared: schema.sensitive(Token),
        audited: true,
    }),
    /** Revoke a token of another space's connection as the client (RFC 7009 2.1). */
    revokeToken: method.query({
        permission: "request",
        input: schema.object({
            /** The token revoked. */
            revocation: schema.sensitive(TokenRevocation),
        }),
        output: schema.object({}),
        prepared: schema.object({}),
        audited: true,
    }),
    /** Start a device authorization for another space's connection (RFC 8628 3.1). */
    authorizeDevice: method.query({
        permission: "request",
        input: schema.object({
            /** The scopes and resource asked for. */
            request: DeviceRequest,
        }),
        output: schema.sensitive(DeviceAuthorization),
        prepared: schema.sensitive(DeviceAuthorization),
        audited: true,
    }),
    delete: method.delete("manage"),
    /** Register a client at a resource's authorization server, for a space's connection. */
    register: method.create("request", {
        fields: [],
        input: schema.object({
            /** The protected resource whose metadata lists the authorization server. */
            resource: schema.url({ protocol: /^https$/u }),
            /** The URI the authorization server redirects the person back to. */
            redirectUri: schema.httpUrl(),
            /** The scope's vault for an issued secret, null if the connection has none. */
            vault: ObjectReference.nullable(),
        }),
        prepared: Registration,
        isPredicted: false,
    }),
}));
/** A registered integration. */
export type Integration = Select<typeof integration.table>;
