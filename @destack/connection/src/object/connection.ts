import { type Select } from "@destack/db";
import { none, relation, through, union } from "@destack/access";
import { type CallOf, CONSUMER, defineObject, field } from "@destack/object";
import { CallError } from "@destack/runtime/error";
import { defineSchema, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Credential, Material } from "../credential/credential.ts";

/** The token type of an access token exchanged for another (RFC 8693 3). */
export const ACCESS_TOKEN_TYPE = "urn:ietf:params:oauth:token-type:access_token";

/** How a connection authenticates at its external system, with what each kind needs. */
export const ConnectionProtocol = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** An API key sent in a header. */
            kind: schema.literal("key"),
            /** The header sending the key, such as authorization or x-api-key. */
            header: schema.string().min(1),
            /** The scheme before the key, such as Bearer, absent to send the key alone. */
            scheme: schema.string().min(1).exactOptional(),
        }),
        schema.object({
            /** HTTP message signatures (RFC 9421). */
            kind: schema.literal("signature"),
            /** The signature algorithm (RFC 9421 6.2.2). */
            algorithm: schema.enum(["hmac-sha256", "ed25519"]),
            /** The key identifier the verifier looks the key up by. */
            keyId: schema.string().min(1),
        }),
        schema.object({
            /** AWS Signature Version 4. */
            kind: schema.literal("sigv4"),
            /** The AWS region, such as eu-central-1. */
            region: schema.string().min(1),
            /** The service's signing name, such as s3 or execute-api. */
            service: schema.string().min(1),
        }),
        schema.object({
            /** A JWT assertion exchanged for an access token (RFC 7523 2.1). */
            kind: schema.literal("jwt"),
            /** The assertion's issuer, such as a service account's address. */
            issuer: schema.string().min(1),
            /** The assertion's subject, the issuer itself for a service account. */
            subject: schema.string().min(1),
        }),
        schema.object({
            /** The authorization code grant with PKCE (RFC 6749 4.1, RFC 7636). */
            kind: schema.literal("authorization-code"),
        }),
        schema.object({
            /** The client credentials grant (RFC 6749 4.4). */
            kind: schema.literal("client-credentials"),
        }),
        schema.object({
            /** An integration installed on an external account, whose issuer issues its tokens. */
            kind: schema.literal("installation"),
        }),
        schema.object({
            /** The device authorization grant (RFC 8628). */
            kind: schema.literal("device-code"),
        }),
        schema.object({
            /** A token exchanged for another connection's token (RFC 8693). */
            kind: schema.literal("token-exchange"),
            /** The connection whose current access token is the subject token. */
            source: ObjectReference,
        }),
        schema.object({
            /** A TLS client certificate (RFC 8705). */
            kind: schema.literal("mtls"),
        }),
    ]),
);
/** How a connection authenticates at its external system. */
export type ConnectionProtocol = schema.Infer<typeof ConnectionProtocol>;

/** A connection lent to a bound installation: its protocol and current credential. */
export const Loan = defineSchema(
    schema.object({
        /** How requests carry the credential. */
        protocol: ConnectionProtocol,
        /** The current credential. */
        credential: Credential,
    }),
);
/** A connection lent to a bound installation. */
export type Loan = schema.Infer<typeof Loan>;

/** A kind of connection. */
export type ConnectionKind = ConnectionProtocol["kind"];

/** A started authorization: the page the person approves it on, and the request's state. */
export const AuthorizationStart = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** An authorization page with code grant and PKCE (RFC 6749 4.1, RFC 7636). */
            kind: schema.literal("page"),
            /** The page the person approves the authorization on. */
            url: schema.httpUrl(),
            /** The state the authorization server returns. */
            state: schema.string().min(1),
            /** The redirect URI the authorization server returns to. */
            redirectUri: schema.httpUrl(),
            /** The PKCE code verifier. */
            verifier: schema.string().min(1),
            /** The client of a dynamic registration, then used by the connection, if any. */
            registered: ObjectReference.nullable(),
        }),
        schema.object({
            /** An integration's installation page. */
            kind: schema.literal("installation"),
            /** The page the person installs the integration on. */
            url: schema.httpUrl(),
            /** The state the installation page returns. */
            state: schema.string().min(1),
            /** The redirect URI the installation page returns to. */
            redirectUri: schema.httpUrl(),
        }),
        schema.object({
            /** The device authorization grant (RFC 8628). */
            kind: schema.literal("device"),
            /** The verification page the person enters the user code on. */
            url: schema.httpUrl(),
            /** The device code the controller polls with. */
            deviceCode: schema.string().min(1),
            /** The code the person enters. */
            userCode: schema.string().min(1),
            /** The seconds between polls. */
            interval: schema.number().int().positive(),
        }),
    ]),
);
/** A started authorization. */
export type AuthorizationStart = schema.Infer<typeof AuthorizationStart>;

/** What a completed authorization granted, its credential already in the vault. */
export const AuthorizationGrant = defineSchema(
    schema.object({
        /** The request the completion used. */
        request: schema.identifier("authorization-request"),
        /** The external account's identifier, where the server gives it. */
        subject: schema.string().min(1).exactOptional(),
        /** The scopes granted. */
        scopes: schema.array(schema.string().min(1)),
        /** The access token's expiry, absent for a token that does not expire. */
        expiresAt: schema.number().int().exactOptional(),
    }),
);
/** What a completed authorization granted. */
export type AuthorizationGrant = schema.Infer<typeof AuthorizationGrant>;

/** A revoked connection's credential, withdrawn at its server and destroyed after commit. */
export const Withdrawal = defineSchema(
    schema.object({
        /** The connection. */
        id: schema.identifier("connection"),
        /** How the connection authenticated. */
        protocol: ConnectionProtocol,
        /** The integration whose client the tokens were issued to. */
        integration: ObjectReference.nullable(),
        /** The external account, the installation of an installed integration. */
        subject: schema.string().min(1).nullable(),
        /** The vault keeping the credential. */
        vault: ObjectReference,
        /** When the vault last took the credential, null while it keeps none. */
        keptAt: schema.number().int().nullable(),
    }),
);
/** A revoked connection's credential. */
export type Withdrawal = schema.Infer<typeof Withdrawal>;

/** Credentials for one external account, kept in a vault and lent to outbound calls. */
export const connection = defineObject({
    name: "connection",
    plural: "connections",
    controlled: true,
    // relate the installations a person lends the connection to as its consumers
    bindable: { by: "manage" },
    // replicate the connections of accounts and organisations into their scopes
    inherited: {},
    fields: {
        /** How the connection authenticates. */
        protocol: field.json(ConnectionProtocol),
        /** The integration whose client receives tokens, for OAuth kinds and installations. */
        integration: field.json(ObjectReference).optional(),
        /** The protected resource of the tokens (RFC 8707), listing its server (RFC 9728). */
        resource: field.string(schema.url({ protocol: /^https$/u })).optional(),
        /** The scopes requested, then the ones granted. */
        scopes: field.json(schema.array(schema.string().min(1))).default([]),
        /** The vault keeping the credential. */
        vault: field.json(ObjectReference),

        // the controller's authorization
        /** The external account's identifier, such as a user or installation identifier. */
        subject: field.string(schema.string().min(1)).optional(),
        /** The current access token's expiry, absent for credentials that do not expire. */
        expiresAt: field.time().optional(),
        /** When the vault last took the connection's credential, absent while it keeps none. */
        keptAt: field.time().optional(),
    },
    // let its consumers and the servers of its scope read it, and its consumers borrow it
    permissions: {
        read: union(relation(CONSUMER), through("scope", "serve")),
        manage: none(),
        lend: relation(CONSUMER),
    },
    reserved: ["lend"],
}).methods((method) => ({
    /** Create a connection, keeping the material its kind sends in the vault. */
    create: method.create("manage", {
        fields: ["protocol", "integration", "resource", "scopes", "vault"],
        input: schema.object({
            /** The material a key, signature, SigV4, JWT or mTLS connection sends. */
            material: schema.sensitive(Material).exactOptional(),
        }),
        prepared: schema.object({}),
        isPredicted: false,
    }),
    /** Lend the connection's protocol and current credential to an installation bound to it. */
    lend: method.query({
        permission: "lend",
        output: schema.sensitive(Loan),
        prepared: schema.sensitive(Loan),
        isAudited: true,
    }),
    /** List the scope's authorized installations of the integrations with a client identifier. */
    installations: method.query({
        permission: "lend",
        target: "scope",
        input: schema.object({
            /** The integrations' client identifier, such as a GitHub App's. */
            clientId: schema.string().min(1),
            /** The one connection to read, every one when absent. */
            id: schema.identifier("connection").exactOptional(),
            /** The issuer's installation to read, every one when absent. */
            installation: schema.string().min(1).exactOptional(),
        }),
        output: schema.object({
            /** The authorized installations. */
            items: schema.array(
                schema.object({
                    /** The connection. */
                    id: schema.identifier("connection"),
                    /** The issuer's identifier of the installation. */
                    installation: schema.string(),
                }),
            ),
        }),
    }),
    /** Start authorizing the external account, answering the page a person approves it on. */
    authorize: method.mutation({
        permission: "manage",
        input: schema.object({
            /** The URI the authorization server redirects the person back to. */
            redirectUri: schema.httpUrl().exactOptional(),
        }),
        output: schema.object({
            /** The page the person approves the authorization on. */
            url: schema.httpUrl(),
            /** The code the person enters on the page, for the device authorization grant. */
            userCode: schema.string().min(1).exactOptional(),
        }),
        prepared: AuthorizationStart,
    }),
    /** Complete the authorization with the authorization server's redirect parameters. */
    complete: method.mutation({
        permission: "manage",
        input: schema.object({
            /** The redirect's query parameters, its state and code among them. */
            parameters: schema.record(schema.string(), schema.string()),
        }),
        prepared: AuthorizationGrant,
    }),
    /** Revoke the credential at the authorization server and destroy it. */
    revoke: method.mutation({ permission: "manage", prepared: Withdrawal }),
    delete: method.delete("manage"),
}));
/** A persisted connection. */
export type Connection = Select<typeof connection.table>;

/** The checks a connection's creation runs. */
export const Connection = {
    /** Require a token exchange's source to be a same-scope connection the caller manages. */
    async requireSource(call: CallOf<typeof connection, "create", unknown>): Promise<void> {
        // check only a token exchange
        const { protocol } = call.input;
        if (protocol.kind !== "token-exchange") {
            return;
        }

        // refuse a source outside the connection's scope, then one the caller does not manage
        const { source } = protocol;
        if (!connection.is(source) || source.scope !== call.scope) {
            throw new CallError("FORBIDDEN", {
                message: `a token exchange's source must be a connection of ${call.scope}`,
            });
        }
        await call.require(connection.permission("manage"), source);
    },
};
