import type { Environment } from "@destack/environment";
import type { Call } from "@destack/sync";
import { type ClientOf, createClient } from "@destack/object/client";
import { Egress, PackageMount } from "@destack/object";
import type { InstalledServerContext } from "@destack/object/workload";
import { Account, account } from "@destack/account/object";
import connectionPackage, { CONNECTION_INSTALLATION } from "../package.ts";
import { Snapshot } from "@destack/db";
import type { MethodContext, ResultsOf } from "@destack/object";
import { Identifier, type JsonValue, present, schema } from "@destack/schema";
import { RequestId, type ObjectReference, Replica, type Subject } from "@destack/sync";
import { Credential, Token } from "../credential/credential.ts";
import { ConnectionError, OAuthRefusal } from "../error/error.ts";
import type { AuthorizationClient } from "../oauth/server.ts";
import { AuthorizationProxy } from "./proxy.ts";
import { type Integration, integration, IntegrationIssuer } from "../object/integration.ts";
import { IssuedInstallation, Issuer } from "../object/issuer.ts";
import {
    ACCESS_TOKEN_TYPE,
    type Authorized,
    Connection,
    type ConnectionKind,
    connection,
    REVOKED,
} from "../object/connection.ts";
import { Assertion } from "../oauth/assertion.ts";
import type { Fetch } from "@destack/object";
import { AuthorizationServer, JWT_BEARER, TOKEN_EXCHANGE } from "../oauth/server.ts";
import { SecretReading } from "@destack/vault";
import { secret, vault } from "@destack/vault/object";
import type { ObjectStore } from "@destack/object/server";
import { AccessContext, Caller } from "@destack/access";

/** How long before expiry an access token is refreshed: a minute. */
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

/** What a token source works with. */
export interface TokenSourceOptions {
    /** Connection's own installation, the consumer the credentials in the vaults are bound to. */
    readonly consumer: Subject;
    /** Send requests to authorization servers. */
    readonly fetch: Fetch;
    /** Call the packages at an address through the host's egress as the installation. */
    readonly at: InstalledServerContext["at"];
    /** The environment the installation runs in. */
    readonly environment: Environment;
}

/** An integration as its connections' client: here with the secret, or through its primary. */
export interface IntegrationClient {
    /** The integration. */
    readonly integration: Integration;
    /** The authorization server as the client: here with the secret, or through the primary. */
    readonly server: AuthorizationClient;
}

/** What runs a token source's reads and vault calls: a call, a settlement, or the store. */
export type Runner = Pick<MethodContext, "now" | "self" | "environment"> &
    Pick<Snapshot, "read"> & {
        /** Read through a method as a caller. */
        readonly query: <Result>(caller: AccessContext, call: Call<Result>) => Promise<Result>;
        /** Run calls as a caller. */
        readonly mutate: <const Calls extends readonly Call[]>(
            caller: AccessContext,
            calls: Calls,
        ) => Promise<ResultsOf<Calls>>;
    };

/** The runners of a token source's work outside calls. */
export const Runner = {
    /** Run as a store's principal now, each call batch its own request. */
    of(
        store: Pick<ObjectStore, "environment" | "principal" | "database" | "query" | "mutate">,
    ): Runner {
        const now = store.environment.clock.now();
        const live = Snapshot.live(store.database);

        return {
            now,
            environment: store.environment,
            self: AccessContext.of(Caller.of(store.principal), now),
            read: (table) => live.read(table),
            query: (caller, call) => store.query(caller, call),
            mutate: (caller, calls) =>
                store.mutate(caller, { id: RequestId.create(store.environment), calls }),
        };
    },
};

/** The owner of a vault secret: a connection, or an integration with its client secret. */
const SecretOwner = schema.union([
    schema.identifier("connection"),
    schema.identifier("integration"),
]);

/** The tokens of connections: obtained and refreshed by the controller, read by leases. */
export class TokenSource {
    /** The consumer, fetch and primary the tokens come through. */
    readonly options: TokenSourceOptions;

    /** Source tokens with the consumer, fetch and primary. */
    constructor(options: TokenSourceOptions) {
        this.options = options;
    }

    /** Derive the vault secret of a connection or integration from its identifier. */
    static secret(id: string): Identifier<"secret"> {
        const owner = SecretOwner.parse(id);

        return Identifier.derive("secret", Identifier.uuid(owner));
    }

    /** Store a connection's or integration's secret in a vault, bound to the consumer. */
    async put(
        runner: Runner,
        reference: ObjectReference,
        id: string,
        value: string,
    ): Promise<void> {
        await runner.mutate(runner.self, [
            vault.call("put", {
                ...TokenSource.#vault(reference),
                secret: TokenSource.secret(id),
                name: id,
                value: { encoding: "text", value },
            }),
        ]);
    }

    /** Read a connection's or integration's secret from its vault as the consumer. */
    async readSecret(runner: Runner, reference: ObjectReference, id: string): Promise<string> {
        // read the secret's current version through its vault as the consumer
        const target = secret.target({ scope: reference.scope, id: TokenSource.secret(id) });
        const caller = AccessContext.of(Caller.of(this.options.consumer), runner.now);
        const reading = SecretReading.parse(
            await runner.query(caller, secret.call("read", target)),
        );

        return SecretReading.text(reading);
    }

    /** Remove a connection's or integration's secret from its vault. */
    async remove(runner: Runner, reference: ObjectReference, id: string): Promise<void> {
        await runner.mutate(runner.self, [
            vault.call("remove", {
                ...TokenSource.#vault(reference),
                secret: TokenSource.secret(id),
            }),
        ]);
    }

    /** Address a vault by its reference. */
    static #vault(reference: ObjectReference) {
        return vault.target({ scope: reference.scope, id: vault.identifier(reference.id) });
    }

    /** Read a connection's credential, refusing one without a token or with a lapsed one. */
    async current(runner: Runner, row: Connection, now: number): Promise<Credential> {
        // refuse a connection whose controller has yet to obtain its first token
        if (row.storedAt === null) {
            throw ConnectionError.of(
                "TOKEN_PENDING",
                `connection ${row.id} has no token yet, which its controller will obtain`,
            );
        }

        // refuse a lapsed token, which the controller refreshes before it expires
        const credential = await this.read(runner, row);
        const expiresAt = credential.token?.expiresAt;
        if (expiresAt !== undefined && expiresAt <= now) {
            throw ConnectionError.of(
                "TOKEN_EXPIRED",
                `connection ${row.id} has an expired token, which its controller will refresh`,
            );
        }

        return credential;
    }

    /** Read a connection's stored credential from its vault. */
    async read(
        runner: Runner,
        row: {
            readonly id: string;
            readonly vault: ObjectReference;
        },
    ): Promise<Credential> {
        return Credential.read(await this.readSecret(runner, row.vault, row.id));
    }

    /** Read an integration as client: here with its vault's secret, or through its primary. */
    async integration(
        runner: Runner,
        reference: ObjectReference | null,
    ): Promise<IntegrationClient> {
        // request a replica's tokens through its primary, which alone reads its secret
        const row = await TokenSource.find(runner, reference);
        if (await Replica.isReplicated(runner, row.scope, integration.table)) {
            const primary = await this.#primary(runner, row.scope);
            return {
                integration: row,
                server: new AuthorizationProxy(
                    row,
                    integration.reference(row.scope, row.id),
                    present(primary, `the primary of ${row.scope}`),
                    this.options.fetch,
                    runner.now,
                    this.options.environment,
                ),
            };
        }

        // request a stored integration's tokens with the secret its vault stores
        return { integration: row, server: await this.local(runner, row) };
    }

    /** Call the issuer of an integration an installation issues the tokens of. */
    async issuer(runner: Runner, reference: ObjectReference | null) {
        // require an installation issuing the integration's tokens
        const row = await TokenSource.find(runner, reference);
        const issued = row.issuer;
        if (issued.kind !== "installation") {
            throw ConnectionError.of(
                "ISSUER_NOT_INSTALLATION",
                `integration ${row.clientId} gets its tokens from ${issued.url}, not an installation`,
            );
        }

        // call the issuing installation through the egress, under its package's issuer routes
        const at = this.options.at(Egress.address(issued.name, issued.issuer.scope));
        const type = Issuer.type(issued.package);
        const client = createClient(
            { router: { issuer: type.procedures } },
            {
                url: `${at.url}${PackageMount.path(issued.package.id)}`,
                fetch: (request) => at.fetch(request),
            },
            this.options.environment,
        );
        const target = { spaceId: issued.issuer.scope, id: issued.issuer.id };
        const call = (method: string, input: object) =>
            present(client.issuer[method], `the issuer's ${method}`)({ ...target, ...input });

        // read the issuer's answers as another package's, through the template's schemas
        return {
            issue: async (installation: string) =>
                Token.parse(await call("issue", { installation })),
            uninstall: async (installation: string) => {
                await call("uninstall", {
                    installation,
                    requestId: RequestId.create(runner.environment),
                });
            },
            installation: async (parameters: Readonly<Record<string, string>>) =>
                IssuedInstallation.parse(
                    await call("installation", {
                        parameters,
                        requestId: RequestId.create(runner.environment),
                    }),
                ).installation,
        };
    }

    /** Find a connection's integration, refusing a connection without one. */
    static async find(
        runner: Pick<Runner, "read">,
        reference: ObjectReference | null,
    ): Promise<Integration> {
        // require the integration
        if (reference === null) {
            throw ConnectionError.of("INTEGRATION_REQUIRED", "the connection has no integration");
        }
        const row = await runner.read(integration.table).findFirst({
            where: {
                id: schema.identifier("integration").parse(reference.id),
            },
        });
        if (row === undefined || row.scope !== reference.scope) {
            throw ConnectionError.of(
                "INTEGRATION_NOT_FOUND",
                `integration ${reference.id} not found in ${reference.scope}`,
            );
        }

        return row;
    }

    /** Address a stored integration's server as its client, with its vault's secret. */
    async local(runner: Runner, row: Integration): Promise<AuthorizationServer> {
        return this.server(row, await this.#secret(runner, row), runner.now);
    }

    /** Register a client in a scope at a resource's authorization server. */
    async register(
        scope: string,
        request: {
            readonly resource: string;
            readonly redirectUri: string;
            readonly vault: ObjectReference | null;
        },
        runner: Runner,
    ): Promise<Integration> {
        // register at the scope's primary, else here
        const primary = await this.#primary(runner, scope);
        if (primary !== undefined) {
            return primary.integration.register({
                scope,
                requestId: RequestId.create(runner.environment),
                ...request,
            });
        }

        // register here as the system of the scope
        const [registered] = await runner.mutate(runner.self, [
            integration.call("register", {
                ...integration.in(scope),
                ...request,
            }),
        ]);

        return present(registered, `the client registered in ${scope}`);
    }

    /** Store a credential in a connection's vault. */
    async write(
        runner: Runner,
        row: { readonly id: string; readonly vault: ObjectReference },
        credential: Credential,
    ): Promise<void> {
        await this.put(runner, row.vault, row.id, Credential.write(credential));
    }

    /** Record a connection's Authorized condition and fields, refusing a deleted connection. */
    async observe(
        runner: Runner,
        row: Connection,
        authorized: Authorized,
        fields: Readonly<Record<string, JsonValue>>,
    ): Promise<void> {
        // read the connection at its current revision
        const current = await runner.read(connection.table).findFirst({ where: { id: row.id } });
        if (current === undefined) {
            throw ConnectionError.of("CONNECTION_NOT_FOUND", `connection ${row.id} is gone`);
        }

        // observe it at that revision, never over a revocation, which its controller then withdraws
        const isRevoked = Connection.authorized(current) === "Revoked";
        const condition = isRevoked ? REVOKED : authorized;
        await runner.mutate(runner.self, [
            connection.call("observe", {
                ...connection.target(current),
                observedGeneration: current.generation,
                conditions: { Authorized: condition },
                fields,
            }),
        ]);
    }

    /** Address an integration's server as its client, with its secret or public, at a time. */
    server(
        row: Pick<Integration, "issuer" | "endpoints" | "clientId" | "authentication">,
        clientSecret: string | undefined,
        now: number,
    ): AuthorizationServer {
        return new AuthorizationServer({
            issuer: IntegrationIssuer.url(row.issuer),
            endpoints: row.endpoints,
            client: {
                clientId: row.clientId,
                authentication: row.authentication,
                ...(clientSecret === undefined ? {} : { secret: clientSecret }),
            },
            fetch: this.options.fetch,
            now,
            environment: this.options.environment,
        });
    }

    /** Decide whether this space stores the integrations of a scope's account, as its primary. */
    isPrimary(chain: readonly string[]): boolean {
        const [scope, enclosing] = chain;
        const accountId = scope !== undefined && account.identifies(scope) ? scope : enclosing;

        return accountId !== undefined && Account.home(accountId) === this.options.consumer.scope;
    }

    /** Connect to the connections at a scope's primary, absent where this space is the primary. */
    async #primary(
        view: Pick<Snapshot, "read">,
        scope: string,
    ): Promise<ClientOf<typeof connectionPackage> | undefined> {
        // store this space's own integrations here
        const primary = await Account.homeOf(view, scope);
        if (primary === this.options.consumer.scope) {
            return undefined;
        }

        // call the connection installation at the primary through the egress
        const at = this.options.at(Egress.address(CONNECTION_INSTALLATION, primary));

        return createClient(
            connectionPackage,
            {
                url: `${at.url}${PackageMount.path(connectionPackage.id)}`,
                fetch: (request) => at.fetch(request),
            },
            this.options.environment,
        );
    }

    /** Read a stored integration's secret from its vault, absent for a public client. */
    async #secret(runner: Runner, row: Integration): Promise<string | undefined> {
        return row.vault === null ? undefined : this.readSecret(runner, row.vault, row.id);
    }

    /** Report whether a failure ends a person's grant, refused or without a refresh token. */
    static isGrantExpired(error: unknown): boolean {
        return (
            OAuthRefusal.code(error) === "invalid_grant" ||
            ConnectionError.is(error, "REFRESH_TOKEN_MISSING")
        );
    }

    /** Obtain and store a new token, marking the connection authorized or its grant expired. */
    async refresh(runner: Runner, row: Connection, now: number): Promise<Credential> {
        // obtain the token from the stored credential, or from none before the first is stored
        const credential = row.storedAt === null ? {} : await this.read(runner, row);
        let token: Token;
        try {
            token = await this.#obtain(runner, row, credential, now);
        } catch (error) {
            if (TokenSource.isGrantExpired(error)) {
                const message = "the authorization server refused the grant; authorize again";
                await this.observe(
                    runner,
                    row,
                    { status: "false", reason: "Expired", message },
                    { expiresAt: null },
                );
            }
            throw error;
        }

        // store the token and the refresh token a server did not rotate
        const refreshToken = token.refreshToken ?? credential.token?.refreshToken;
        const next: Credential = {
            ...credential,
            token: { ...token, ...(refreshToken === undefined ? {} : { refreshToken }) },
        };
        await this.write(runner, row, next);

        // mark the connection authorized until the token expires
        await this.observe(
            runner,
            row,
            { status: "true", reason: "Authorized", message: "" },
            {
                expiresAt: token.expiresAt ?? null,
                scopes: token.scopes ?? row.scopes,
                storedAt: now,
                failures: 0,
            },
        );

        return next;
    }

    /** Obtain a new token the way a connection's kind does. */
    async #obtain(
        runner: Runner,
        row: Connection,
        credential: Credential,
        now: number,
    ): Promise<Token> {
        // issue an installation's token through the integration's issuer
        const protocol = row.protocol;
        if (protocol.kind === "installation") {
            const issued = await this.issuer(runner, row.integration);

            return issued.issue(requireSubject(row));
        }
        // request a token at the token endpoint for a grant the connection has itself
        const client = await this.integration(runner, row.integration);
        if (protocol.kind === "client-credentials" || protocol.kind === "jwt") {
            return this.#grant(client.server, row, credential, now);
        }
        // exchange the source connection's token (RFC 8693)
        else if (protocol.kind === "token-exchange") {
            const source = await this.#source(runner, protocol.source, now);

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

    /** Request a client credentials (RFC 6749 4.4) or JWT bearer (RFC 7523 2.1) token now. */
    async #grant(
        server: AuthorizationClient,
        row: Connection,
        credential: Credential,
        now: number,
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
            throw ConnectionError.of("CREDENTIAL_MISSING", "the connection has no signing key");
        }
        const assertion = await Assertion.sign(
            {
                issuer: protocol.issuer,
                subject: protocol.subject,
                audience: server.endpoints.token ?? server.issuer,
                key: material.privateKey,
                ...scopeParameter(row),
            },
            now,
            this.options.environment,
        );

        return server.token({ grant_type: JWT_BEARER, assertion, ...scopeParameter(row) });
    }

    /** Read the current access token of a token exchange's source connection. */
    async #source(runner: Runner, reference: ObjectReference, now: number): Promise<string> {
        // read the source connection's current token
        const row = present(
            await runner.read(connection.table).findFirst({
                where: {
                    id: schema.identifier("connection").parse(reference.id),
                },
            }),
            `the source connection ${reference.id}`,
        );
        const token = (await this.current(runner, row, now)).token;
        if (token === undefined) {
            throw ConnectionError.of(
                "CREDENTIAL_MISSING",
                `connection ${reference.id} has no access token`,
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
        throw ConnectionError.of(
            "REFRESH_TOKEN_MISSING",
            "the connection has no refresh token: authorize again",
        );
    }

    return server.token({
        grant_type: "refresh_token",
        refresh_token: refreshToken,
        ...scopeParameter(row),
    });
}

/** Write a connection's requested scopes as a scope parameter, absent without any. */
function scopeParameter(row: Connection): { readonly scope?: string } {
    return row.scopes.length === 0 ? {} : { scope: row.scopes.join(" ") };
}

/** Require the installation a connection authorized. */
function requireSubject(row: Connection): string {
    if (row.subject === null) {
        throw ConnectionError.of(
            "CONNECTION_NOT_INSTALLED",
            `connection ${row.id} is not installed yet`,
        );
    }

    return row.subject;
}
