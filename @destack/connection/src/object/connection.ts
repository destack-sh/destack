import { type Select } from "@destack/db";
import { ConnectionError } from "../error/error.ts";
import { none, permission, relation, through, union } from "@destack/access";
import { type MethodContextOf, CONSUMER, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Credential, Material } from "../credential/credential.ts";
import { ConnectionKind } from "../declare/connection.ts";
import { account, space } from "@destack/account/object";

/** The token type of an access token exchanged for another (RFC 8693 3). */
export const ACCESS_TOKEN_TYPE = "urn:ietf:params:oauth:token-type:access_token";

/** How a connection authenticates at its external system, with what each kind needs. */
export const ConnectionProtocol = schema.discriminatedUnion("kind", [
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
]);
/** How a connection authenticates at its external system. */
export type ConnectionProtocol = schema.Infer<typeof ConnectionProtocol>;

/** A connection leased to a call: its protocol, current credential and the API they are for. */
export const ConnectionLease = schema.object({
    /** How requests carry the credential. */
    protocol: ConnectionProtocol,
    /** The current credential. */
    credential: Credential,
    /** The API the credential is for, below which its calls go. */
    resource: schema.url({ protocol: /^https$/u }),
});
/** A connection leased to a call. */
export type ConnectionLease = schema.Infer<typeof ConnectionLease>;

/** A kind of protocol a connection authenticates with. */
export type ConnectionProtocolKind = ConnectionProtocol["kind"];

/** A started authorization: the page the person approves it on, and the request's state. */
export const AuthorizationStart = schema.discriminatedUnion("kind", [
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
]);
/** A started authorization. */
export type AuthorizationStart = schema.Infer<typeof AuthorizationStart>;

/** What a completed authorization granted, its credential already in the vault. */
export const AuthorizationGrant = schema.object({
    /** The request the completion used. */
    request: schema.identifier("authorization-request"),
    /** The external account's identifier, where the server gives it. */
    subject: schema.string().min(1).exactOptional(),
    /** The scopes granted. */
    scopes: schema.array(schema.string().min(1)),
    /** The access token's expiry, absent for a token that does not expire. */
    expiresAt: schema.number().int().exactOptional(),
});
/** What a completed authorization granted. */
export type AuthorizationGrant = schema.Infer<typeof AuthorizationGrant>;

/** One external account's credential, the resource of the connection kind, leased to calls. */
export const connection = defineObject(
    {
        name: "connection",
        plural: "connections",
        scope: [space, account],
        provisioned: { kind: ConnectionKind },
        // let people bind a connection they manage to one installation at a time, such as per repository
        bindable: { by: "manage" },
        // replicate the connections of accounts and organizations into their scopes
        propagated: {},
        fields: {
            /** How the connection authenticates. */
            protocol: field.json(ConnectionProtocol),
            /** The integration whose client receives tokens, for OAuth kinds and installations. */
            integration: field.json(ObjectReference).optional(),
            /** The scopes the authorization granted, which may differ from the requested ones. */
            granted: field.json(schema.array(schema.string().min(1))).default([]),

            // the controller's authorization
            /** The external account's identifier, such as a user or installation identifier. */
            subject: field.string(schema.string().min(1)).optional(),
            /** The current access token's expiry, absent for credentials that do not expire. */
            expiresAt: field.time().optional(),
            /** When the vault last stored the credential, absent while it stores none. */
            storedAt: field.time().optional(),
            /** The refreshes that failed in a row since the last one that succeeded. */
            failures: field.integer().default(0),
            /** When the controller next refreshes, retries or polls, absent while none is due. */
            renewAt: field.time().optional(),
        },
        // let its consumers and the servers of its scope read it, and its consumers use it
        permissions: {
            read: union(relation(CONSUMER), through("scope", "serve")),
            manage: none(),
            // let the people managing connections declare them, and an account's home its stacks' ones
            declare: union(permission("manage"), through("scope", "provision")),
            // let its consumers use it through the egress, and the people managing it through their tools
            use: union(relation(CONSUMER), permission("manage")),
            // let the installations of an account's primary control the account's connections
            control: through("scope", "provision"),
        },
        reserved: ["use"],
    },
    (method) => ({
        /** Declare a connection of a spec, its credential then rotated in or authorized. */
        create: method.create("declare", {
            fields: [
                "name",
                "definitionPackageId",
                "definitionVersion",
                "definitionName",
                "spec",
                "placement",
                "retention",
                "protocol",
                "integration",
            ],
        }),
        /** Replace a supplied credential's material, storing it in the vault first. */
        rotate: method.mutation({
            permission: "manage",
            input: schema.object({
                /** The material the connection's kind sends, such as a new API key. */
                material: schema.sensitive(Material),
            }),
            prepared: schema.object({}),
            isPredicted: false,
        }),
        /** Lease the connection's protocol and current credential to a call its egress sends. */
        lease: method.query({
            permission: "use",
            output: schema.sensitive(ConnectionLease),
            prepared: schema.sensitive(ConnectionLease),
            isAudited: true,
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
        /** Disconnect the external account, whose controller revokes the credential. */
        disconnect: method.mutation({ permission: "manage" }),
        delete: method.delete("manage"),
    }),
);
/** A persisted connection. */
export type Connection = Select<typeof connection.table>;

/** Why a connection is or is not authorized, as its Authorized condition records it. */
export const AUTHORIZED_REASONS = [
    "Authorized",
    "Supplied",
    "Pending",
    "Denied",
    "Expired",
    "Failing",
    "Disconnected",
] as const;

/** Why a connection is or is not authorized. */
export const AuthorizedReason = schema.enum(AUTHORIZED_REASONS);
/** Why a connection is or is not authorized. */
export type AuthorizedReason = schema.Infer<typeof AuthorizedReason>;

/** A connection's Authorized condition as its verbs and controller write it. */
export interface Authorized {
    /** Whether the connection is authorized, true, false or unknown. */
    readonly status: "true" | "false" | "unknown";
    /** Why. */
    readonly reason: AuthorizedReason;
    /** What a person reads about it. */
    readonly message: string;
}

/** The Ready condition of a connection, which the egress reaches at its identifier. */
export const READY = {
    status: "true",
    reason: "External",
    message: "called through the egress below its identifier",
} as const;

/** The Authorized condition of a disconnected connection. */
export const DISCONNECTED: Authorized = {
    status: "false",
    reason: "Disconnected",
    message: "disconnected",
};

/** Connections: the checks their creation runs, and the installation an authorized one holds. */
export const Connection = {
    /** Read why a connection is or is not authorized, absent before its controller reports it. */
    authorized(row: Pick<Connection, "conditions">): AuthorizedReason | undefined {
        const condition = row.conditions["Authorized"];

        return condition === undefined ? undefined : AuthorizedReason.parse(condition.reason);
    },

    /** Read the installation an authorized connection holds of an integration, null otherwise. */
    installation(
        row: Pick<Connection, "protocol" | "conditions" | "subject">,
        integrated: { readonly clientId: string } | undefined,
        clientId: string,
    ): string | null {
        const isAuthorized =
            row.protocol.kind === "installation" &&
            row.conditions["Authorized"]?.status === "true" &&
            integrated?.clientId === clientId;

        return isAuthorized ? row.subject : null;
    },

    /** Require a token exchange's source to be a same-scope connection the caller manages. */
    async requireSource(
        context: MethodContextOf<typeof connection, "create", unknown>,
    ): Promise<void> {
        // check only a token exchange
        const { protocol } = context.input;
        if (protocol.kind !== "token-exchange") {
            return;
        }

        // refuse a source outside the connection's scope, then one the caller does not manage
        const { source } = protocol;
        if (!connection.is(source) || source.scope !== context.scope) {
            throw ConnectionError.of(
                "EXCHANGE_SOURCE_FOREIGN",
                `a token exchange's source must be a connection of ${context.scope}`,
            );
        }
        await context.require(connection.permission("manage"), source);
    },
};
