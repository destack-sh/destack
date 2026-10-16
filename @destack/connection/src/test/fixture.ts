import { Package, type PackageId } from "@destack/package";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/runtime/authentication";
import { Account, account, space } from "@destack/account/object";
import { AccessContext, Authentication, Caller, principal, relation, Role } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { STACKS } from "@destack/account/test";
import {
    defineDatabase,
    defineTable,
    type DatabaseConnection,
    eq,
    TABLE,
    type Table,
    text,
    Transaction,
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { defineObject, INTRINSIC, type ObjectType } from "@destack/object";
import { defineServer, ObjectStore } from "@destack/object/server";
import { callTables, testEvents } from "@destack/object/test";
import type { EventStore } from "@destack/event";
import { Clock, Identifier, present, schema } from "@destack/schema";
import { binding, capture, deployment, installation } from "@destack/stack/object";
import { secret, vault } from "@destack/vault/object";
import { implementSecret, implementSecretVersion, implementVault } from "@destack/vault/server";
import { vaultTables } from "@destack/vault/stack";
import vaultPackage from "@destack/vault/package";
import { ResourceContext } from "@destack/resource/context";
import { Health } from "@destack/runtime/health";
import { RequestId } from "@destack/runtime/request";
import { Server, RequestContext } from "@destack/runtime/server";
import { reconciliation, TestBearer, testCallKey } from "@destack/runtime/test";
import { ControlLoop } from "@destack/runtime/control";
import { expect, onTestFinished } from "@destack/test";
import { type ObjectReference, Scope, Subject, Uplink } from "@destack/sync";
import { v7 } from "uuid";
import { createClient, type ClientOf } from "@destack/runtime/client";
import {
    authorizationRequest,
    integration,
    connection,
    type IntegrationIssuer,
} from "../object/index.ts";
import { ISSUING, testIssuer } from "./issuer.ts";
import { Pkce } from "@destack/runtime/authentication";
import type { Fetch } from "@destack/runtime";
import {
    integrationFollow,
    implementIntegration,
    implementConnection,
    Runner,
    TokenSource,
} from "../server/index.ts";
import connectionPackage, { CONNECTION_INSTALLATION } from "../package.ts";
import { ConnectionRequest } from "../credential/request.ts";
import { Egress, PackageHandle, PackageMount } from "@destack/runtime";
import type { ServerContext } from "@destack/runtime/workload";
import { integrationTables, connectionTables } from "../stack/index.ts";

/** The issuer of the test authorization server. */
export const ISSUER = "https://auth.test";

/** The name of the installation serving the fixture's issuer. */
const ISSUER_INSTALLATION = "issuer";

/** A protected resource whose metadata lists the test authorization server. */
export const RESOURCE = "https://mcp.test/mcp";

/** How long the test server's access tokens last, in seconds. */
const TOKEN_SECONDS = 3600;

/** The origin clients call a fixture's connections at, in process. */
const ORIGIN = "https://connection.test";

/** The person owning the fixture's scopes. */
const OWNER = schema.identifier("user").parse("user-01a1134b-0000-7000-8000-00000000c0de");

/** The machine serving the space, consuming its credentials. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01a1134b-0000-7000-8000-00000000c4ed");

/** The tables a vault's checks read, which a space's database replicates. */
const SPACE_REPLICATED: readonly Table[] = [
    account.table,
    space.table,
    installation.table,
    binding.table,
    capture.table,
    deployment.table,
];

/** The spaces placed in the scopes of a home, as the directory places them. */
export const placementTable = defineTable(
    "placement",
    {
        /** The placed space. */
        id: text("id").primaryKey(),
        /** The scope the space lives in. */
        scope: text("scope").notNull(),
    },
    { log: {} },
);

/** A space placed in a scope as a principal inside it, as directory placements do. */
export const placement = defineObject({
    name: "placement",
    plural: "placements",
    [INTRINSIC]: {
        table: placementTable,
        mapping: {
            table: placementTable,
            id: "id",
            scope: "scope",
            attributes: {},
            relations: { self: { column: "id", scope: Scope.universe.id } },
        },
    },
    relations: { self: { subjects: [principal.space], grantedBy: null } },
    permissions: { read: relation("self") },
});

/** A database of connections and their vaults, as a space keeps it. */
export const connectionDatabase = defineDatabase({
    name: "connection",
    tables: [...connectionTables, ...integrationTables, ...vaultTables, ...callTables],
    replicated: SPACE_REPLICATED,
});

/** An authorization server in memory, issuing numbered tokens that rotate on refresh. */
export class TestAuthorizationServer {
    /** The client secret of the static test client. */
    readonly clientSecret = "client-secret";
    /** The requests it received, as method, path and form or JSON body. */
    readonly requests: {
        readonly path: string;
        readonly body: Readonly<Record<string, string>>;
    }[] = [];
    /** The tokens revoked so far. */
    readonly revoked: string[] = [];
    /** The authorization codes issued and not yet used, with their PKCE challenge and redirect. */
    readonly #codes = new Map<
        string,
        { readonly challenge: string; readonly redirectUri: string }
    >();
    /** The refresh tokens that are current. */
    readonly #refreshTokens = new Set<string>();
    /** The device codes, approved or not. */
    readonly #devices = new Map<string, { readonly userCode: string; isApproved: boolean }>();
    /** The tokens issued so far, numbering the next. */
    #issued = 0;
    /** Answer requests to other origins, such as an installation issuer's API. */
    other: Fetch = async () => new Response(null, { status: 404 });

    /** Answer a request to the server or the protected resource's metadata. */
    readonly fetch = async (request: Request): Promise<Response> => {
        // hand requests to other origins on
        const url = new URL(request.url);
        if (url.origin !== ISSUER && url.origin !== new URL(RESOURCE).origin) {
            return this.other(request);
        }

        // read the path and a form body
        const content = request.body === null ? "" : await request.text();
        const body = Object.fromEntries(
            new URLSearchParams(content.startsWith("{") ? "" : content),
        );
        this.requests.push({ path: `${url.origin}${url.pathname}`, body });

        // answer metadata, registration, device authorization, tokens and revocation
        const path = `${url.origin}${url.pathname}`;
        if (path === "https://mcp.test/.well-known/oauth-protected-resource/mcp") {
            return Response.json({ resource: RESOURCE, authorization_servers: [ISSUER] });
        } else if (path === `${ISSUER}/.well-known/oauth-authorization-server`) {
            return Response.json({
                issuer: ISSUER,
                ...TestAuthorizationServer.endpoints(),
                authorization_response_iss_parameter_supported: true,
            });
        } else if (path === `${ISSUER}/register`) {
            return Response.json({
                client_id: `dynamic-${v7()}`,
                token_endpoint_auth_method: "none",
            });
        } else if (path === `${ISSUER}/device`) {
            const deviceCode = `device-${v7()}`;
            const userCode = `USER-${this.#devices.size + 1}`;
            this.#devices.set(deviceCode, { userCode, isApproved: false });

            return Response.json({
                device_code: deviceCode,
                user_code: userCode,
                verification_uri: `${ISSUER}/activate`,
                interval: 1,
            });
        } else if (path === `${ISSUER}/revoke`) {
            this.revoked.push(String(body["token"]));

            return new Response(null, { status: 200 });
        }

        return this.#token(request, body);
    };

    /** The server's endpoints as its metadata lists them (RFC 8414 2). */
    static endpoints() {
        return {
            authorization_endpoint: `${ISSUER}/authorize`,
            token_endpoint: `${ISSUER}/token`,
            revocation_endpoint: `${ISSUER}/revoke`,
            device_authorization_endpoint: `${ISSUER}/device`,
            registration_endpoint: `${ISSUER}/register`,
        };
    }

    /** Approve an authorization page as a person does, answering the redirect's parameters. */
    async approve(page: string): Promise<Record<string, string>> {
        // issue a code bound to the page's challenge and redirect
        const url = new URL(page);
        const code = `code-${v7()}`;
        this.#codes.set(code, {
            challenge: String(url.searchParams.get("code_challenge")),
            redirectUri: String(url.searchParams.get("redirect_uri")),
        });

        return { code, state: String(url.searchParams.get("state")), iss: ISSUER };
    }

    /** Approve a device authorization by user code, as a person does on the verification page. */
    approveDevice(userCode: string): void {
        for (const device of this.#devices.values()) {
            if (device.userCode === userCode) {
                device.isApproved = true;
            }
        }
    }

    /** Answer the token endpoint for a grant. */
    async #token(request: Request, body: Readonly<Record<string, string>>): Promise<Response> {
        // refuse a confidential client presenting another secret
        const basic = request.headers.get("authorization");
        if (basic !== null && basic !== `Basic ${btoa(`static-client:${this.clientSecret}`)}`) {
            return TestAuthorizationServer.#error("invalid_client");
        }

        // check the grant
        const grant = body["grant_type"];
        if (grant === "authorization_code") {
            const code = this.#codes.get(String(body["code"]));
            this.#codes.delete(String(body["code"]));
            const challenge = await Pkce.challenge(String(body["code_verifier"]));
            if (
                code === undefined ||
                code.challenge !== challenge ||
                code.redirectUri !== body["redirect_uri"]
            ) {
                return TestAuthorizationServer.#error("invalid_grant");
            }
        } else if (grant === "refresh_token") {
            if (!this.#refreshTokens.delete(String(body["refresh_token"]))) {
                return TestAuthorizationServer.#error("invalid_grant");
            }
        } else if (grant === "urn:ietf:params:oauth:grant-type:device_code") {
            const device = this.#devices.get(String(body["device_code"]));
            if (device?.isApproved !== true) {
                return TestAuthorizationServer.#error("authorization_pending");
            }
            this.#devices.delete(String(body["device_code"]));
        }

        // issue a numbered access token with a refresh token for a person's grant
        this.#issued += 1;
        const isPersonal =
            grant === "authorization_code" ||
            grant === "refresh_token" ||
            grant?.endsWith("device_code") === true;
        const refreshToken = isPersonal ? `refresh-${this.#issued}` : undefined;
        if (refreshToken !== undefined) {
            this.#refreshTokens.add(refreshToken);
        }

        return Response.json({
            access_token: `access-${this.#issued}`,
            token_type: "bearer",
            expires_in: TOKEN_SECONDS,
            ...(refreshToken === undefined ? {} : { refresh_token: refreshToken }),
        });
    }

    /** Answer an OAuth error (RFC 6749 5.2). */
    static #error(error: string): Response {
        return Response.json({ error }, { status: 400 });
    }
}

/** What a fixture opens with. */
export interface ConnectionFixtureOptions {
    /** The issuer the fixture serves as an issuing installation in its serving space. */
    readonly issuer?: typeof testIssuer;
    /** The tables of packages sharing the database, which read the connections as replicas. */
    readonly tables?: readonly Table[];
    /** The home of other scopes' integrations and the space replicating them, as a space has. */
    readonly home?: {
        readonly home: (scope: string, signer?: Subject) => ClientOf<typeof connectionPackage>;
        /** Call the home's connections as the space's connection installation. */
        readonly at: ServerContext["at"];
        readonly space: string;
        /** The account enclosing the space, where its home placed it. */
        readonly enclosing: ObjectReference;
    };
    /** The tables that sharing packages replicate, such as an installation's integrations. */
    readonly replicated?: readonly Table[];
}

/** Connections served over a database with a scope, owner, vault and test server. */
export class ConnectionFixture {
    /** The database. */
    readonly database: DatabaseConnection;
    /** The object store keeping the connections. */
    readonly objects: ObjectStore;
    /** The connections' tokens. */
    readonly tokens: TokenSource;
    /** The authorization server. */
    readonly server: TestAuthorizationServer;
    /** The scope the connections live in. */
    readonly scope: ObjectReference;
    /** The vault in the scope. */
    readonly vaultReference: ObjectReference;
    /** The connections served in process to clients sending their principal. */
    readonly http: Server;
    /** Close the database. */
    readonly #close: () => Promise<void>;

    /** Keep the open connections. */
    private constructor(
        fields: Omit<
            ConnectionFixture,
            | "close"
            | "context"
            | "control"
            | "connections"
            | "dump"
            | "follow"
            | "home"
            | "keepIssuer"
            | "as"
            | "borrow"
            | "lend"
            | "place"
            | "secrets"
            | "staticIntegration"
        > & { readonly close: () => Promise<void> },
    ) {
        // keep the store, its vault, server and scope
        this.database = fields.database;
        this.objects = fields.objects;
        this.tokens = fields.tokens;
        this.server = fields.server;
        this.scope = fields.scope;
        this.vaultReference = fields.vaultReference;
        this.http = fields.http;
        this.#close = fields.close;
    }

    /** Serve connections over a fresh database. */
    static async open(options: ConnectionFixtureOptions = {}): Promise<ConnectionFixture> {
        // keep the database and the authorization server
        const storage = await TestDatabase.create("sqlite", ConnectionFixture.#declare(options), {
            isMigrated: true,
        });
        const { database } = storage;
        const server = new TestAuthorizationServer();
        const home = options.home;
        const scope = await ConnectionFixture.#scope(database, home);
        // reach the served issuer as the connection installation, and other installations at the home
        let issuing: Server | undefined;
        const consumer = ConnectionFixture.#installation(scope);
        const issuer = Egress.address(ISSUER_INSTALLATION, consumer.scope);
        const tokens = new TokenSource({
            consumer,
            fetch: server.fetch,
            at: (address) =>
                address === issuer && options.issuer !== undefined
                    ? {
                          url: ORIGIN,
                          fetch: (request) =>
                              ConnectionFixture.#sent(
                                  present(issuing, "the issuing package's server"),
                                  request,
                                  consumer,
                                  ISSUING.id,
                              ),
                      }
                    : home === undefined
                      ? ConnectionFixture.#unreached(address)
                      : home.at(address),
        });

        // serve the connections, the vaults, and the integrations at their home
        const events = await testEvents(database);
        const objects = ConnectionFixture.#objects(database, tokens, options, scope, events);
        const http = ConnectionFixture.#serve(objects, connectionPackage);
        issuing =
            options.issuer === undefined
                ? undefined
                : ConnectionFixture.#serve(objects, ConnectionFixture.#issuing(options.issuer));

        return new ConnectionFixture({
            database,
            objects,
            tokens,
            server,
            scope,
            vaultReference: await ConnectionFixture.#vault(database, scope),
            http,
            close: async () => {
                await issuing?.close();
                await http.close();
                await storage.close();
            },
        });
    }

    /** Keep a vault in a scope. */
    static async #vault(
        database: DatabaseConnection,
        scope: ObjectReference,
    ): Promise<ObjectReference> {
        // record the vault
        const id = vault.generateId();
        const now = Date.now();
        await database.insert(vault.table).values({
            id,
            scope: scope.id,
            name: "credentials",
            definitionPackageId: vault.package.id,
            definitionVersion: vault.package.version,
            definitionName: "credentials",
            spec: {},
            createdAt: now,
            updatedAt: now,
        });

        return vault.reference(scope.id, id);
    }

    /** Declare the database: connections, and integrations at home or replicated in a space. */
    static #declare(options: ConnectionFixtureOptions) {
        // keep the integrations at their home, or replicate them as a space
        const isHome = options.home === undefined;
        const replicated = [
            ...(isHome ? [] : integrationTables),
            ...SPACE_REPLICATED,
            ...(options.replicated ?? []),
        ];
        const tables = [
            ...connectionTables,
            ...vaultTables,
            placementTable,
            ...callTables,
            ...(isHome ? integrationTables : []),
            ...(options.issuer === undefined ? [] : options.issuer.tables),
            ...(options.tables ?? []),
        ];

        return defineDatabase({
            name: "connection",
            tables: tables.filter((table) => !replicated.includes(table)),
            replicated,
        });
    }

    /** Serve connections and vaults, and integrations at their home, else follow their replicas. */
    static #objects(
        database: DatabaseConnection,
        tokens: TokenSource,
        options: ConnectionFixtureOptions,
        scope: ObjectReference,
        events: EventStore,
    ): ObjectStore {
        const { home } = options;

        return new ObjectStore({
            clock: Clock.system,
            principal: ConnectionFixture.#installation(scope),
            servers: [
                defineServer(connectionPackage, {
                    objects: [
                        implementConnection(tokens),
                        authorizationRequest,
                        ...(home === undefined ? [implementIntegration(tokens)] : []),
                    ],
                    ...(home === undefined ? {} : { follows: [integrationFollow] }),
                }),
                defineServer(vaultPackage, {
                    objects: [implementVault(), implementSecret(), implementSecretVersion()],
                }),
                ...(options.issuer === undefined
                    ? []
                    : [
                          defineServer(ConnectionFixture.#issuing(options.issuer), {
                              objects: [options.issuer],
                          }),
                      ]),
            ],
            policies: [account, space, placement],
            database,
            callKey: testCallKey,
            events,
            origin: { package: Package.of(connectionPackage) },
            // keep the chain above the serving space, which lives in the account, as its placed home does
            chain: async () => ({
                kind: "home",
                parent: account.is(scope) ? scope.id : scope.scope,
            }),
            ...(home === undefined
                ? {}
                : {
                      publisher: (followed: string) =>
                          Uplink.of((signer) => home.home(followed, signer).replica),
                  }),
        });
    }

    /** Answer clients sending their principal, leaving the controllers to the test. */
    static #serve(objects: ObjectStore, handle: PackageHandle): Server {
        return Server.start({
            clock: Clock.system,
            ...objects.package(handle),
            controllers: [],
            drainTimeout: 1000,
            health: new Health("connection"),
            resources: new ResourceContext(),
            authorizeMachine: async () => {},
            authenticate: async (request) => ConnectionFixture.#authenticate(request, handle.id),
        });
    }

    /** Declare the issuing package's handle with its issuer type. */
    static #issuing(issuer: ObjectType) {
        return PackageHandle.of(ISSUING, { objects: { issuer } });
    }

    /** Keep the home's account, or a space in an account, with their rows as chains keep them. */
    static async #scope(
        database: DatabaseConnection,
        home: ConnectionFixtureOptions["home"],
    ): Promise<ObjectReference> {
        // keep the owner's account with its row: the home's own scope, or the one a space lives in
        const owner = principal.user.reference(Scope.universe.id, OWNER);
        const access = new AccessFixture(database);
        const enclosing = home?.enclosing;
        const holder = enclosing ?? account.reference(OWNER, Identifier.create("account"));
        await access.replicateScope(owner);
        await access.replicateScope(holder);
        await database.insert(account.table).values({
            id: schema.identifier("account").parse(holder.id),
            scope: OWNER,
            handle: "owner",
            name: "Owner",
            residencyId: "eu",
            kind: "personal",
            createdAt: 0,
            updatedAt: 0,
        });

        // keep the home's account as its scope, or a space in the account, with the serving space's row
        const isHome = home === undefined;
        const scope = isHome ? holder : space.reference(holder.id, home.space);
        const serving = space.reference(holder.id, ConnectionFixture.#principal(scope).id);
        await access.replicateScope(serving);
        await database.insert(space.table).values({
            id: schema.identifier("space").parse(serving.id),
            scope: schema.identifier("account").parse(holder.id),
            name: isHome ? "home" : "connections",
            stack: STACKS.space,
            createdAt: 0,
            updatedAt: 0,
        });

        // let the person own the scope, and the space serving it and its connection installation act as its owners
        await access.bindOwner(scope, owner);
        await Transaction.run(database, async (transaction) => {
            await Role.bindOwner(
                transaction,
                scope,
                ConnectionFixture.#principal(scope),
                Date.now(),
            );
            await Role.bindOwner(
                transaction,
                scope,
                ConnectionFixture.#installation(scope),
                Date.now(),
            );
        });

        return scope;
    }

    /** Read the connection installation serving a scope's connections, in its serving space. */
    static #installation(scope: ObjectReference): Subject {
        return principal.installation.reference(
            ConnectionFixture.#principal(scope).id,
            INSTALLATION,
        );
    }

    /** Refuse calling another space from a home, which keeps its own integrations. */
    static #unreached(address: string): never {
        throw new TypeError(`a home calls no other connections, not ${address}`);
    }

    /** Read the space serving a scope: an account's home, or the space itself. */
    static #principal(scope: ObjectReference): Subject {
        const id = account.is(scope) ? Account.home(scope.id) : scope.id;

        return principal.space.reference(Scope.universe.id, id);
    }

    /** Place a space in the fixture's account, answering the account. */
    async place(spaceId: string): Promise<ObjectReference> {
        // place the space, and keep its row and scope in the account as the directory replica does
        const id = schema.identifier("space").parse(spaceId);
        const accountId = schema.identifier("account").parse(this.scope.id);
        await this.database.insert(placementTable).values({ id, scope: accountId });
        await new AccessFixture(this.database).replicateScope(space.reference(accountId, id));
        await this.database.insert(space.table).values({
            id,
            scope: accountId,
            name: "connections",
            stack: STACKS.space,
            createdAt: 0,
            updatedAt: 0,
        });

        return this.scope;
    }

    /** Close the database. */
    close(): Promise<void> {
        return this.#close();
    }

    /** Build the context of the owner calling in a session. */
    context(session = "session-one"): RequestContext {
        const subject = principal.user.reference(Scope.universe.id, OWNER);
        const now = Date.now();

        return new RequestContext(new Request("https://test.local"), {
            clock: Clock.system,
            audience: connectionPackage.id,
            scope: this.scope.id,
            authentication: {
                caller: Caller.of(subject),
                credential: {
                    kind: "session",
                    id: session,
                    audience: connectionPackage.id,
                    verifiedAt: now,
                    expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                },
            },
            resources: new ResourceContext(),
        });
    }

    /** Connect to these connections as the home of every scope's integrations. */
    home(spaceId: string): Pick<NonNullable<ConnectionFixtureOptions["home"]>, "home" | "at"> {
        // reach the home's connections as the space's connection installation, at the home's address
        const connections = principal.installation.reference(spaceId, INSTALLATION);
        const address = Egress.address(
            CONNECTION_INSTALLATION,
            ConnectionFixture.#principal(this.scope).id,
        );

        return {
            home: (_scope, signer) =>
                this.as(signer ?? principal.space.reference(Scope.universe.id, spaceId)),
            at: (called) => {
                if (called !== address) {
                    throw new TypeError(`no connections at ${called}, only at ${address}`);
                }

                return {
                    url: ORIGIN,
                    fetch: (request) => ConnectionFixture.#sent(this.http, request, connections),
                };
            },
        };
    }

    /** Connect to the connections as a subject sending its principal in the bearer. */
    as(subject: Subject): ClientOf<typeof connectionPackage> {
        return createClient(connectionPackage, {
            url: ORIGIN,
            headers: { authorization: `Bearer ${Subject.key(subject)}` },
            fetch: (request: Request) => this.http.fetch(request),
        });
    }

    /** Send a request to the mounted connections with a subject's principal in the bearer. */
    static #sent(
        http: Server,
        request: Request,
        subject: Subject,
        packageId: PackageId = connectionPackage.id,
    ): Promise<Response> {
        // route the package's mount as the gateway does
        const mount = PackageMount.of(request);
        if (mount?.packageId !== packageId) {
            throw new TypeError(`no connections at ${request.url}`);
        }

        // send the request below the mount with the subject's bearer
        const headers = new Headers(mount.request.headers);
        headers.set("authorization", `Bearer ${Subject.key(subject)}`);

        return http.fetch(new Request(mount.request, { headers }));
    }

    /** Lend a connection to an installation of its serving space, bound to it first. */
    async lend(reference: ObjectReference, request: Request) {
        // bind the borrowing installation as the owner, then borrow as it
        const borrower = principal.installation.reference(
            ConnectionFixture.#principal(this.scope).id,
            "installation-borrower",
        );
        await this.objects.mutate(this.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("bindConsumer", {
                    ...connection.target({ scope: reference.scope, id: reference.id }),
                    consumer: borrower,
                }),
            ],
        });

        return this.borrow(reference, request, borrower);
    }

    /** Borrow a connection as a consumer bound to it, applying its credential to a request. */
    borrow(reference: ObjectReference, request: Request, consumer: Subject) {
        return ConnectionRequest.borrow(this.as(consumer), reference, request, Date.now());
    }

    /** Run the connection controller over every connection once, answering its waits. */
    async control(): Promise<(number | undefined)[]> {
        // find the controller and reconcile each key once
        const controller = this.objects.controllers().find((each) => each.name === connection.name);
        if (controller === undefined) {
            throw new TypeError("the connection package controls its connections");
        }
        const waits: (number | undefined)[] = [];
        for (const key of await controller.list()) {
            waits.push(await controller.reconcile(key, reconciliation()));
        }

        return waits;
    }

    /** Follow the store's replicas in a control loop until the test ends. */
    follow(): void {
        // run the replica controller, keeping every failure
        const controller = new AbortController();
        const failures: unknown[] = [];
        const loop = new ControlLoop(
            this.database,
            this.objects.controllers().filter((each) => each.name === "replica"),
            { clock: Clock.system, report: (_controller, _key, error) => failures.push(error) },
        );
        const running = loop.run(controller.signal);
        onTestFinished(async () => {
            controller.abort();
            await running;
            expect(failures).toEqual([]);
        });
    }

    /** Read every row of the package's tables, the calls included, by table. */
    async dump(): Promise<Readonly<Record<string, unknown[]>>> {
        const rows: Record<string, unknown[]> = {};
        for (const table of [...connectionTables, ...integrationTables, ...callTables]) {
            rows[table[TABLE].name] = await this.database.select().from(table);
        }

        return rows;
    }

    /** Read the live secret values of the fixture's vault, as their bound consumer. */
    async secrets(): Promise<string[]> {
        // read every secret of the vault in the order it was kept
        const rows = await this.database
            .select({ id: secret.table.id })
            .from(secret.table)
            .where(
                eq(secret.table.parentId, schema.identifier("vault").parse(this.vaultReference.id)),
            )
            .orderBy(secret.table.createdAt);
        const values: string[] = [];
        for (const row of rows) {
            const value = await this.tokens.readSecret(
                Runner.of(this.objects),
                this.vaultReference,
                row.id,
            );
            values.push(present(value, `the value of secret ${row.id}`));
        }

        return values;
    }

    /** Read the connections as their table keeps them. */
    connections() {
        return this.database.select().from(connection.table);
    }

    /** Register the test server's static confidential client, public or not, returning it. */
    async staticIntegration(
        options: { readonly isPublic?: boolean } = {},
    ): Promise<ObjectReference> {
        const created = await this.objects
            .mutate(this.context().access(), {
                id: RequestId.create(Date.now()),
                calls: [
                    integration.call("create", {
                        scope: this.scope.id,
                        issuer: { kind: "server", url: ISSUER },
                        endpoints: {
                            authorization: `${ISSUER}/authorize`,
                            token: `${ISSUER}/token`,
                            revocation: `${ISSUER}/revoke`,
                            deviceAuthorization: `${ISSUER}/device`,
                            isIssuerReturned: true,
                        },
                        clientId: "static-client",
                        authentication: "client_secret_basic",
                        redirectUris: ["https://app.test/callback"],
                        vault: this.vaultReference,
                        secret: this.server.clientSecret,
                        isPublic: options.isPublic ?? false,
                    }),
                ],
            })
            .then(([written]) => written);

        return integration.reference(this.scope.id, created.id);
    }

    /** Keep the served issuer in the serving space, its consumer the connection installation. */
    async keepIssuer(): Promise<IntegrationIssuer> {
        // create the issuer as the serving space
        const serving = ConnectionFixture.#principal(this.scope);
        const [created] = await this.objects.mutate(
            AccessContext.of(Caller.of(serving), Date.now()),
            {
                id: RequestId.create(Date.now()),
                calls: [testIssuer.call("create", testIssuer.in(serving.id))],
            },
        );
        const reference = testIssuer.reference(serving.id, present(created, "the issuer").id);

        // let the connection installation call it
        const consumer = ConnectionFixture.#installation(this.scope);
        await new AccessFixture(this.database).replicateRelationship(
            serving.id,
            reference,
            "consumer",
            consumer,
        );

        return {
            kind: "installation",
            package: ISSUING,
            name: ISSUER_INSTALLATION,
            issuer: reference,
        };
    }

    /** Authenticate a request to a package as the principal in its bearer token. */
    static #authenticate(request: Request, audience: PackageId): Authentication {
        const subject = Subject.read(TestBearer.read(request));

        return TestBearer.authenticate(subject, audience, Date.now());
    }
}
