import type { connect } from "../client/client.ts";
import { and, eq } from "@destack/db";
import type { Condition } from "@destack/object";
import { aligned, type Identifier, type JsonValue, present, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import type { ObjectReference, Subject } from "@destack/sync";
import { Credential, type Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import type { InstallationIssuer } from "../installation/installation.ts";
import type { AuthorizationClient } from "../oauth/server.ts";
import { IntegrationCopy } from "./copy.ts";
import { HomeClient } from "./home.ts";
import { type Integration, integration } from "../object/integration.ts";
import {
    ACCESS_TOKEN_TYPE,
    type Connection,
    type ConnectionKind,
    connection,
} from "../object/connection.ts";
import { Assertion } from "../oauth/assertion.ts";
import type { Fetch } from "@destack/service";
import { AuthorizationServer, JWT_BEARER, TOKEN_EXCHANGE } from "../oauth/server.ts";
import { Secret } from "@destack/vault/server";
import type { ObjectStore } from "@destack/object/server";
import { Caller } from "@destack/access";

/** How long before its expiry an access token is refreshed: a minute, as client libraries refresh ahead of clock drift and request time. */
export const REFRESH_MARGIN = 60_000;

/** The kinds whose credential is material their owner supplied, sent as it is. */
export const SUPPLIED_KINDS: ReadonlySet<ConnectionKind> = new Set([
    "key",
    "signature",
    "sigv4",
    "mtls",
]);

/** The kinds a person authorizes, whose tokens refresh once granted. */
export const PERSON_KINDS: ReadonlySet<ConnectionKind> = new Set([
    "authorization-code",
    "installation",
    "device-code",
]);

/** What a token source works with: the consumer of the credentials, the fetch, the installation issuers and the home of other scopes' integrations. */
export interface TokenSourceOptions {
    /** The machine serving the space, the consumer the credentials in the vaults are bound to. */
    readonly consumer: Subject;
    /** Send requests to authorization servers and installation issuers. */
    readonly fetch: Fetch;
    /** The issuers of installed integrations, by their issuer identifiers. */
    readonly issuers: readonly InstallationIssuer[];
    /** Connect to the connection service of the space keeping a scope's integrations and connections, as this space or a signing installation, absent where every integration is kept here. */
    readonly home?: (scope: string, signer?: Subject) => ReturnType<typeof connect>;
}

/** An integration as the client of its connections: its authorization server here with the secret, or through its home for a copy. */
export interface IntegrationClient {
    /** The integration. */
    readonly integration: Integration;
    /** The authorization server as the client: here with the secret, or through the integration's home. */
    readonly server: AuthorizationClient;
    /** The secret or private key, kept only where the integration is kept, absent for a copy and for a public client. */
    readonly secret: string | undefined;
}

/** The current tokens of connections, refreshed once per connection at a time, as the controller and the lender ask for them. */
export class TokenSource {
    /** The consumer, fetch, installation issuers and home the tokens come through. */
    readonly options: TokenSourceOptions;
    /** The refreshes in flight, by connection, which concurrent askers join. */
    readonly #refreshing = new Map<string, Promise<Credential>>();

    /** Source tokens with the service's options. */
    constructor(options: TokenSourceOptions) {
        this.options = options;
    }

    /** Derive the vault secret of a connection or integration from its identifier. */
    static secret(id: string): Identifier<"secret"> {
        return schema.identifier("secret").parse(`secret-${id.slice(id.indexOf("-") + 1)}`);
    }

    /** Read a connection's credential, refreshing a token that expires within the margin. */
    async current(
        system: Omit<ObjectStore, "router">,
        row: Connection,
        now: number,
    ): Promise<Credential> {
        // send supplied material and a token that stays valid
        const credential = await this.read(system, row);
        const expiresAt = credential.token?.expiresAt;
        if (
            SUPPLIED_KINDS.has(row.protocol.kind) ||
            (credential.token !== undefined &&
                (expiresAt === undefined || expiresAt - REFRESH_MARGIN > now))
        ) {
            return credential;
        }

        return this.refresh(system, row, now);
    }

    /** Refresh a connection's token once at a time, writing it to the vault and the expiry to the connection. */
    refresh(
        system: Omit<ObjectStore, "router">,
        row: Connection,
        now: number,
    ): Promise<Credential> {
        // join a refresh in flight as a rotated refresh token is used once (RFC 9700 4.14)
        const flying = this.#refreshing.get(row.id);
        if (flying !== undefined) {
            return flying;
        }

        // refresh once and forget the flight when it settles
        const refreshing = this.#refresh(system, row, now).finally(() =>
            this.#refreshing.delete(row.id),
        );
        this.#refreshing.set(row.id, refreshing);

        return refreshing;
    }

    /** Read a connection's credential from its vault, empty before the first write. */
    async read(
        system: Omit<ObjectStore, "router">,
        row: { readonly id: string; readonly vault: ObjectReference },
    ): Promise<Credential> {
        const { consumer } = this.options;
        const value = await Secret.read(system, row.vault, TokenSource.secret(row.id), consumer);

        return value === undefined ? {} : Credential.read(value);
    }

    /** Read an integration as its connections' client: here with the secret its vault keeps, or through its home for a copy. */
    async integration(
        system: Omit<ObjectStore, "router">,
        reference: ObjectReference | null,
    ): Promise<IntegrationClient> {
        // require the integration
        if (reference === null) {
            throw new ConnectionError("INVALID_AUTHORIZATION", "the connection has no integration");
        }
        const { database } = system;
        const [row] = await database
            .select()
            .from(integration.table)
            .where(
                and(
                    eq(integration.table.scope, reference.scope),
                    eq(integration.table.id, schema.identifier("integration").parse(reference.id)),
                ),
            );
        if (row === undefined) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `no integration ${reference.id} in ${reference.scope}`,
            );
        }

        // request a copy's tokens through its home, which alone reads its secret
        const home = this.options.home;
        if (home !== undefined && (await IntegrationCopy.isCopied(database, reference))) {
            return {
                integration: row,
                server: new HomeClient(row, reference, home(reference.scope), this.options.fetch),
                secret: undefined,
            };
        }

        // request a kept integration's tokens with the secret its vault keeps
        const secret = await this.#secret(system, row);

        return { integration: row, server: this.server(row, secret), secret };
    }

    /** Address a kept integration's authorization server as its client, with the secret its vault keeps. */
    async local(
        system: Omit<ObjectStore, "router">,
        row: Integration,
    ): Promise<AuthorizationServer> {
        return this.server(row, await this.#secret(system, row));
    }

    /** Register a client in a scope at the authorization server of a resource, at the home of the scope's integrations, or here where they are kept. */
    async register(
        scope: string,
        request: {
            readonly resource: string;
            readonly redirectUri: string;
            readonly vault: ObjectReference | null;
        },
        system: Omit<ObjectStore, "router">,
    ): Promise<Integration> {
        // register at the home as this space
        const home = this.options.home;
        if (home !== undefined) {
            return home(scope).integration.register({
                scope,
                requestId: RequestId.create(),
                ...request,
            });
        }

        // register here as the system of the scope
        const [registered] = await system.mutate(Caller.of(system.principal, system.clock()), {
            id: RequestId.create(),
            calls: [
                integration.call("register", {
                    ...integration.target({ scope }),
                    ...request,
                }),
            ],
        });

        return present(registered, `the client registered in ${scope}`);
    }

    /** Find the issuer of an installed integration, refusing an integration no configured issuer serves. */
    issuer(found: Pick<Integration, "issuer">): InstallationIssuer {
        const issuer = this.options.issuers.find((each) => each.issuer === found.issuer);
        if (issuer === undefined) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `no installation issuer serves ${found.issuer}`,
            );
        }

        return issuer;
    }

    /** Keep a credential in a connection's vault. */
    async write(
        system: Omit<ObjectStore, "router">,
        row: { readonly id: string; readonly vault: ObjectReference },
        credential: Credential,
    ): Promise<void> {
        await Secret.keep(
            system,
            row.vault,
            TokenSource.secret(row.id),
            Credential.write(credential),
            this.options.consumer,
        );
    }

    /** Record a connection's Authorized condition with the fields an authorization writes, refusing a connection gone meanwhile. */
    async observe(
        system: Omit<ObjectStore, "router">,
        row: Connection,
        authorized: Condition,
        fields: Readonly<Record<string, JsonValue>>,
        now: number,
    ): Promise<void> {
        // read the connection at its current revision
        const [current] = await system.database
            .select()
            .from(connection.table)
            .where(eq(connection.table.id, row.id));
        if (current === undefined) {
            throw new ConnectionError("UNAUTHORIZED", `connection ${row.id} is gone`);
        }

        // observe it at that revision
        await system.mutate(Caller.of(system.principal, now), {
            id: RequestId.create(now),
            calls: [
                connection.call("observe", {
                    ...connection.target(current),
                    observedGeneration: current.generation,
                    conditions: { Authorized: authorized },
                    fields,
                }),
            ],
        });
    }

    /** Address an integration's authorization server as its client, with its secret or as a public client. */
    server(
        row: Pick<Integration, "issuer" | "endpoints" | "clientId" | "authentication">,
        secret: string | undefined,
    ): AuthorizationServer {
        return new AuthorizationServer({
            issuer: row.issuer,
            endpoints: row.endpoints,
            client: {
                clientId: row.clientId,
                authentication: row.authentication,
                ...(secret === undefined ? {} : { secret }),
            },
            fetch: this.options.fetch,
        });
    }

    /** Read a kept integration's secret from its vault, absent for a public client. */
    async #secret(
        system: Omit<ObjectStore, "router">,
        row: Integration,
    ): Promise<string | undefined> {
        return row.vault === null
            ? undefined
            : Secret.read(system, row.vault, TokenSource.secret(row.id), this.options.consumer);
    }

    /** Obtain a new token for a connection, keep it, and mark the connection authorized or its grant expired. */
    async #refresh(
        system: Omit<ObjectStore, "router">,
        row: Connection,
        now: number,
    ): Promise<Credential> {
        // obtain the token and mark a refused grant expired for a person to authorize again
        const credential = await this.read(system, row);
        let token: Token;
        try {
            token = await this.#obtain(system, row, credential, now);
        } catch (error) {
            if (error instanceof ConnectionError && error.error === "invalid_grant") {
                const message = "the authorization server refused the grant; authorize again";
                await this.observe(
                    system,
                    row,
                    { status: "false", reason: "Expired", message },
                    { expiresAt: null },
                    now,
                );
            }
            throw error;
        }

        // keep the token and the refresh token a server did not rotate
        const refreshToken = token.refreshToken ?? credential.token?.refreshToken;
        const next: Credential = {
            ...credential,
            token: { ...token, ...(refreshToken === undefined ? {} : { refreshToken }) },
        };
        await this.write(system, row, next);

        // mark the connection authorized until the token expires
        await this.observe(
            system,
            row,
            { status: "true", reason: "Authorized", message: "" },
            { expiresAt: token.expiresAt ?? null, scopes: token.scopes ?? row.scopes },
            now,
        );

        return next;
    }

    /** Obtain a new token the way a connection's kind does. */
    async #obtain(
        system: Omit<ObjectStore, "router">,
        row: Connection,
        credential: Credential,
        now: number,
    ): Promise<Token> {
        // issue an installation's token with the integration's private key
        const protocol = row.protocol;
        const client = await this.integration(system, row.integration);
        if (protocol.kind === "installation") {
            const installation = requireSubject(row);

            return this.issuer(client.integration).token(
                client.integration.clientId,
                installation,
                requireSecret(client),
            );
        }
        // request a token at the token endpoint for a grant the connection holds itself
        else if (protocol.kind === "client-credentials" || protocol.kind === "jwt") {
            return this.#grant(client.server, row, credential);
        }
        // exchange the source connection's token (RFC 8693)
        else if (protocol.kind === "token-exchange") {
            const source = await this.#source(system, protocol.source, now);

            return client.server.token({
                grant_type: TOKEN_EXCHANGE,
                subject_token: source,
                subject_token_type: ACCESS_TOKEN_TYPE,
                ...scopeParameter(row),
                ...(row.resource === null ? {} : { resource: row.resource }),
            });
        }

        return refreshGrant(client.server, row, credential);
    }

    /** Request a client credentials token (RFC 6749 4.4) or a JWT bearer token (RFC 7523 2.1). */
    async #grant(
        server: AuthorizationClient,
        row: Connection,
        credential: Credential,
    ): Promise<Token> {
        // request a client credentials token for the resource
        const protocol = row.protocol;
        if (protocol.kind !== "jwt") {
            const resource = row.resource === null ? {} : { resource: row.resource };

            return server.token({
                grant_type: "client_credentials",
                ...scopeParameter(row),
                ...resource,
            });
        }

        // sign an assertion with the connection's key and present it
        const material = credential.material;
        if (material?.kind !== "jwt") {
            throw new ConnectionError("UNAUTHORIZED", "the connection holds no signing key");
        }
        const assertion = await Assertion.sign({
            issuer: protocol.issuer,
            subject: protocol.subject,
            audience: server.endpoints.token ?? server.issuer,
            key: material.privateKey,
            ...scopeParameter(row),
        });

        return server.token({ grant_type: JWT_BEARER, assertion, ...scopeParameter(row) });
    }

    /** Read the current access token of a token exchange's source connection. */
    async #source(
        system: Omit<ObjectStore, "router">,
        reference: ObjectReference,
        now: number,
    ): Promise<string> {
        // read the source connection's current token
        const rows = await system.database
            .select()
            .from(connection.table)
            .where(
                and(
                    eq(connection.table.scope, reference.scope),
                    eq(connection.table.id, schema.identifier("connection").parse(reference.id)),
                ),
            );
        const token = (await this.current(system, aligned(rows, 0), now)).token;
        if (token === undefined) {
            throw new ConnectionError(
                "UNAUTHORIZED",
                `connection ${reference.id} holds no access token`,
            );
        }

        return token.accessToken;
    }
}

/** Refresh a person's grant with its refresh token (RFC 6749 6). */
function refreshGrant(
    server: AuthorizationClient,
    row: Connection,
    credential: Credential,
): Promise<Token> {
    // require the refresh token, asking a person to authorize again without one
    const refreshToken = credential.token?.refreshToken;
    if (refreshToken === undefined) {
        throw new ConnectionError(
            "REFUSED",
            "the connection holds no refresh token; authorize again",
            { error: "invalid_grant" },
        );
    }

    return server.token({
        grant_type: "refresh_token",
        refresh_token: refreshToken,
        ...scopeParameter(row),
    });
}

/** Write a connection's requested scopes as a token request's scope parameter, absent without any. */
function scopeParameter(row: Connection): { readonly scope?: string } {
    return row.scopes.length === 0 ? {} : { scope: row.scopes.join(" ") };
}

/** Require the installation a connection authorized. */
function requireSubject(row: Connection): string {
    if (row.subject === null) {
        throw new ConnectionError("UNAUTHORIZED", `connection ${row.id} is installed nowhere yet`);
    }

    return row.subject;
}

/** Require an integration's private key. */
export function requireSecret(client: IntegrationClient): string {
    if (client.secret === undefined) {
        throw new ConnectionError(
            "INVALID_AUTHORIZATION",
            `integration ${client.integration.clientId} holds no private key`,
        );
    }

    return client.secret;
}
