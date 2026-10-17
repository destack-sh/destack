import { anyone, contained, none, principal, relation, through, union } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Package } from "@destack/package";
import { InstallationName } from "@destack/runtime";
import { ConnectionError } from "../error/error.ts";
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
export const AuthorizationEndpoints = schema.object({
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
});
/** The endpoints of an authorization server. */
export type AuthorizationEndpoints = schema.Infer<typeof AuthorizationEndpoints>;

/** Who issues an integration's tokens: its authorization server, or a package's installation. */
export const IntegrationIssuer = Object.assign(
    schema.discriminatedUnion("kind", [
        schema.object({
            kind: schema.literal("server"),
            /** The authorization server's issuer identifier (RFC 8414 2). */
            url: schema.url({ protocol: /^https$/u }),
        }),
        schema.object({
            kind: schema.literal("installation"),
            /** The package whose installation issues the tokens, at its registered release. */
            package: Package,
            /** The issuing installation's name in its space. */
            name: InstallationName,
            /** The issuer object in the installation's space. */
            issuer: ObjectReference,
        }),
    ]),
    {
        /** Read the authorization server's issuer identifier, refusing an installation's issuer. */
        url(issuer: IntegrationIssuer): string {
            if (issuer.kind !== "server") {
                throw new ConnectionError(
                    "INVALID_AUTHORIZATION",
                    `installation ${issuer.name} issues the integration's tokens, not a server`,
                );
            }

            return issuer.url;
        },
    },
);
/** Who issues an integration's tokens. */
export type IntegrationIssuer = schema.Infer<typeof IntegrationIssuer>;

/** A dynamically registered client (RFC 7591 3.2.1), with the vault of an issued secret. */
export const Registration = schema.object({
    /** The authorization server issuing its tokens. */
    issuer: IntegrationIssuer,
    /** The authorization server's endpoints. */
    endpoints: AuthorizationEndpoints,
    /** The client identifier the server assigned. */
    clientId: schema.string().min(1),
    /** How the client authenticates at the token endpoint. */
    authentication: schema.enum(CLIENT_AUTHENTICATIONS),
    /** The redirect URIs the client registered. */
    redirectUris: schema.array(schema.httpUrl()),
    /** The vault storing the secret the server issued, absent for a public client. */
    vault: ObjectReference.exactOptional(),
});
/** A client an authorization server registered dynamically. */
export type Registration = schema.Infer<typeof Registration>;

/** An integration registered at an external authorization server, used by OAuth connections. */
export const integration = defineObject(
    {
        name: "integration",
        plural: "integrations",
        // replicate the integrations of platforms and organisations into their scopes and the spaces connecting through them
        inherited: {},
        fields: {
            /** Who issues the integration's tokens. */
            issuer: field.json(IntegrationIssuer),
            /** The authorization server's endpoints. */
            endpoints: field.json(AuthorizationEndpoints),
            /** The client identifier the server assigned. */
            clientId: field.string(schema.string().min(1)),
            /** How the client authenticates at the token endpoint. */
            authentication: field.enum(CLIENT_AUTHENTICATIONS),
            /** The redirect URIs the client registered. */
            redirectUris: field.json(schema.array(schema.httpUrl())),
            /** Whether the owner registered the client or a connection did (RFC 7591). */
            registration: field.enum(["static", "dynamic"]).default("static"),
            /** The vault storing the client secret or private key, absent for a public client. */
            vault: field.json(ObjectReference).optional(),
        },
        relations: {
            // every space a public integration serves, beside its owner's spaces and the spaces below them
            public: { subjects: [anyone.all()], grantedBy: null },
        },
        // let the spaces it serves replicate it, their connection installations request its tokens by possession, and its residency's servers replicate it
        permissions: {
            read: union(contained(principal.space), relation("public"), through("scope", "serve")),
            manage: none(),
            request: union(contained(principal.installation), relation("public")),
        },
        reserved: ["request"],
    },
    (method) => ({
        /** Register an integration, storing its client secret or private key in the vault. */
        create: method.create("manage", {
            fields: ["issuer", "endpoints", "clientId", "authentication", "redirectUris", "vault"],
            input: schema.object({
                /** The client secret or private key as JWK or PEM text, stored in the vault. */
                secret: schema.sensitive(schema.string().min(1)).exactOptional(),
                /** Whether it serves every space beside its owner's and those below them. */
                isPublic: schema.boolean().exactOptional(),
            }),
            prepared: schema.object({}),
            isPredicted: false,
        }),
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
        /** Start a device authorization for another space's connection (RFC 8628 3.1). */
        authorizeDevice: method.query({
            permission: "request",
            input: schema.object({
                /** The scopes and resource asked for. */
                request: DeviceRequest,
            }),
            output: schema.sensitive(DeviceAuthorization),
            prepared: schema.sensitive(DeviceAuthorization),
            isAudited: true,
        }),
        /** Request a token for another space's connection as the client (RFC 6749 3.2). */
        requestToken: method.query({
            permission: "request",
            input: schema.object({
                /** The grant proving possession of the connection's authorization. */
                grant: schema.sensitive(TokenGrant),
            }),
            output: schema.sensitive(Token),
            prepared: schema.sensitive(Token),
            isAudited: true,
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
            isAudited: true,
        }),
        update: method.update("manage", { fields: ["endpoints", "redirectUris"] }),
        delete: method.delete("manage"),
    }),
);
/** A registered integration. */
export type Integration = Select<typeof integration.table>;
