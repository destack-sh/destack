import { Package, type PackageId } from "@destack/package";
import {
    ControlLoop,
    type DatabaseConnection,
    defineDatabase,
    defineTable,
    eq,
    Snapshot,
    TABLE,
    type Table,
    text,
} from "@destack/db";
import { MemoryBlobStorage } from "@destack/resource";
import { TOKEN_LIFETIME_MILLISECONDS, Pkce } from "@destack/access/token";
import { Account, account, space } from "@destack/account/object";
import { AccessContext, Authentication, Caller, principal, relation } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { STACKS } from "@destack/account/test";
import { TestDatabase, reconcileOnce } from "@destack/db/test";
import {
    defineObject,
    type ObjectType,
    Egress,
    PackageHandle,
    PackageMount,
    Installation,
} from "@destack/object";
import {
    Health,
    implementPackage,
    ObjectStore,
    Server,
    RequestContext,
} from "@destack/object/server";
import { testEvents, TestBearer, testCallKey, testServerContext } from "@destack/object/test";
import type { EventStore } from "@destack/event";
import { Identifier, present, schema, Uuid } from "@destack/schema";
import { binding, capture, deployment } from "@destack/stack/object";
import { secret, vault } from "@destack/vault/object";
import { implementSecret, implementSecretVersion, implementVault } from "@destack/vault/server";
import { vaultTables } from "@destack/vault/stack";
import vaultPackage from "@destack/vault/package";
import { ResourceContext } from "@destack/resource/context";
import { RequestId, type ObjectReference, Scope, Subject, Uplink } from "@destack/sync";
import { expect, onTestFinished } from "@destack/test";
import { createClient, type ClientOf } from "@destack/object/client";
import {
    authorizationRequest,
    integration,
    connection,
    type IntegrationIssuer,
} from "../object/index.ts";
import { ISSUING, testIssuer } from "./issuer.ts";
import type { Fetch } from "@destack/object";
import {
    integrationFollow,
    implementIntegration,
    implementConnection,
    Runner,
    TokenSource,
} from "../server/index.ts";
import connectionPackage, { CONNECTION_INSTALLATION } from "../package.ts";
import { ConnectionRequest } from "../credential/request.ts";
import type { InstalledServerContext } from "@destack/object/workload";
import { integrationTables, connectionTables } from "../stack/index.ts";
import type { Environment } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { tablesOf } from "@destack/object";
import objectPackage from "@destack/object/package";

/** The issuer of the test authorization server. */
export const ISSUER = "https://auth.test";

/** The name of the installation serving the fixture's issuer. */
const ISSUER_NAME = "issuer";

/** The serving space's issuing installation. */
const ISSUER_INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01a1134b-0000-7000-8000-00000000155e");

/** A protected resource whose metadata lists the test authorization server. */
export const RESOURCE = "https://mcp.test/mcp";

/** How long the test server's access tokens last, in seconds. */
const TOKEN_SECONDS = 3600;

/** The origin clients call a fixture's connections at, in process. */
const ORIGIN = "https://connection.test";

/** The seed of the simulated world a fixture opens when its test gives none. */
const WORLD_SEED = 1;

/** The moment the world a fixture opens starts at, paused there: 2026-10-01T00:00:00Z. */
const WORLD_START = Date.UTC(2026, 9, 1);

/** The node the connections run as in a world a fixture opens. */
const WORLD_NODE = "connection";

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
    binding.table,
    capture.table,
    deployment.table,
];

/** The spaces placed in the scopes of a primary, as the directory places them. */
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
    mapping: {
        table: placementTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { self: { column: "id", scope: Scope.universe.id } },
    },
    relations: { self: { subjects: [principal.space], grantedBy: null } },
    permissions: { read: relation("self") },
});

/** A database of connections and their vaults, as a space stores it. */
export const connectionDatabase = defineDatabase({
    name: "connection",
    ...tablesOf({
        packages: [objectPackage],
        follows: [],
        tables: [...connectionTables, ...integrationTables, ...vaultTables],
    }),
    replicated: SPACE_REPLICATED,
});

/** How a test server answers a device authorization's poll: approved, or an RFC 8628 3.5 error. */
export type DeviceAnswer =
    | "approved"
    | "authorization_pending"
    | "slow_down"
    | "access_denied"
    | "expired_token"
    | "invalid_grant";

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
    /** The device codes, with how the server answers their next poll (RFC 8628 3.5). */
    readonly #devices = new Map<string, { readonly userCode: string; answer: DeviceAnswer }>();
    /** The statuses the token endpoint fails its next requests with, in turn. */
    readonly #failures: number[] = [];
    /** The tokens issued so far, numbering the next. */
    #issued = 0;
    /** Answer requests to other origins, such as an installation issuer's API. */
    other: Fetch = async () => new Response(null, { status: 404 });
    /** The effects checking the codes' challenges and drawing the issued codes. */
    readonly #environment: Pick<Environment, "clock" | "crypto" | "random">;

    /** Serve the test client's grants in an environment. */
    constructor(environment: Pick<Environment, "clock" | "crypto" | "random">) {
        this.#environment = environment;
    }

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
                client_id: `dynamic-${Uuid.v7(this.#environment)}`,
                token_endpoint_auth_method: "none",
            });
        } else if (path === `${ISSUER}/device`) {
            const deviceCode = `device-${Uuid.v7(this.#environment)}`;
            const userCode = `USER-${this.#devices.size + 1}`;
            this.#devices.set(deviceCode, { userCode, answer: "authorization_pending" });

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
        const code = `code-${Uuid.v7(this.#environment)}`;
        this.#codes.set(code, {
            challenge: String(url.searchParams.get("code_challenge")),
            redirectUri: String(url.searchParams.get("redirect_uri")),
        });

        return { code, state: String(url.searchParams.get("state")), iss: ISSUER };
    }

    /** Approve a device authorization by user code, as a person does on the verification page. */
    approveDevice(userCode: string): void {
        this.answerDevice(userCode, "approved");
    }

    /** Answer a device authorization's next poll as a person or the server decided. */
    answerDevice(userCode: string, answer: DeviceAnswer): void {
        for (const device of this.#devices.values()) {
            if (device.userCode === userCode) {
                device.answer = answer;
            }
        }
    }

    /** Fail the token endpoint's next requests with some server failure statuses, in turn. */
    failTokens(statuses: readonly number[]): void {
        this.#failures.push(...statuses);
    }

    /** Answer the token endpoint for a grant. */
    async #token(request: Request, body: Readonly<Record<string, string>>): Promise<Response> {
        // fail as scripted
        const failure = this.#failures.shift();
        if (failure !== undefined) {
            return new Response(null, { status: failure });
        }

        // refuse a confidential client presenting another secret
        const basic = request.headers.get("authorization");
        if (basic !== null && basic !== `Basic ${btoa(`static-client:${this.clientSecret}`)}`) {
            return TestAuthorizationServer.#error("invalid_client");
        }

        // read the grant
        const grant = body["grant_type"];

        // check the code against its challenge and redirect
        if (grant === "authorization_code") {
            const code = this.#codes.get(String(body["code"]));
            this.#codes.delete(String(body["code"]));
            const challenge = await Pkce.challenge(
                String(body["code_verifier"]),
                this.#environment.crypto,
            );
            if (
                code === undefined ||
                code.challenge !== challenge ||
                code.redirectUri !== body["redirect_uri"]
            ) {
                return TestAuthorizationServer.#error("invalid_grant");
            }
        }
        // spend the refresh token
        else if (grant === "refresh_token") {
            if (!this.#refreshTokens.delete(String(body["refresh_token"]))) {
                return TestAuthorizationServer.#error("invalid_grant");
            }
        }
        // answer a slow-down once, then pending again, and any other refusal until approved
        else if (grant === "urn:ietf:params:oauth:grant-type:device_code") {
            const device = this.#devices.get(String(body["device_code"]));
            if (device === undefined) {
                return TestAuthorizationServer.#error("invalid_grant");
            } else if (device.answer !== "approved") {
                const { answer } = device;
                device.answer = answer === "slow_down" ? "authorization_pending" : answer;

                return TestAuthorizationServer.#error(answer);
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
    /** The primary of other scopes' integrations and the space replicating them, as a space has. */
    readonly primary?: {
        /** Open a client of the primary of a scope's integrations, signed as a subject. */
        readonly primary: (scope: string, signer?: Subject) => ClientOf<typeof connectionPackage>;
        /** Call the primary's connections as the space's connection installation. */
        readonly at: InstalledServerContext["at"];
        /** The space replicating the primary's integrations. */
        readonly space: string;
        /** The account enclosing the space, where its primary placed it. */
        readonly enclosing: ObjectReference;
    };
    /** The tables that sharing packages replicate, such as an installation's integrations. */
    readonly replicated?: readonly Table[];
    /** The simulated world the test's kits share, its own paused at a fixed start by default. */
    readonly simulation?: Simulation;
}

/** Connections served over a database with a scope, owner, vault and test server. */
export class ConnectionFixture {
    /** The database. */
    readonly database: DatabaseConnection;
    /** The object store with the connections. */
    readonly objects: ObjectStore;
    /** The store of the serving space's issuing installation, absent without an issuer. */
    readonly issuing: ObjectStore | undefined;
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
    /** The simulated world the connections run in, which drives their calls. */
    readonly simulation: Simulation;
    /** The world the connections run in, as their node of the simulation. */
    readonly environment: Environment;
    /** Close the database. */
    readonly #close: () => Promise<void>;

    /** Take the open connections. */
    private constructor(
        fields: Omit<
            ConnectionFixture,
            | "close"
            | "context"
            | "control"
            | "connections"
            | "dump"
            | "follow"
            | "primary"
            | "createIssuer"
            | "as"
            | "lease"
            | "bindAndLease"
            | "place"
            | "secrets"
            | "staticIntegration"
        > & { readonly close: () => Promise<void> },
    ) {
        // take the store, its vault, server and scope
        this.database = fields.database;
        this.objects = fields.objects;
        this.issuing = fields.issuing;
        this.tokens = fields.tokens;
        this.server = fields.server;
        this.scope = fields.scope;
        this.vaultReference = fields.vaultReference;
        this.http = fields.http;
        this.simulation = fields.simulation;
        this.environment = fields.environment;
        this.#close = fields.close;
    }

    /** Serve connections over a fresh database. */
    static async open(options: ConnectionFixtureOptions = {}): Promise<ConnectionFixture> {
        // take the test's world, else open one paused at a fixed start, and serve driven through it
        const simulation =
            options.simulation ?? new Simulation({ seed: WORLD_SEED, start: WORLD_START });

        return ConnectionFixture.#open(options, simulation);
    }

    /** Serve connections over a fresh database in a world. */
    static async #open(
        options: ConnectionFixtureOptions,
        simulation: Simulation,
    ): Promise<ConnectionFixture> {
        // take the world's node, the database and the authorization server
        const environment = simulation.environment(WORLD_NODE);
        const storage = await TestDatabase.create(ConnectionFixture.#declare(options), {
            environment,
            isMigrated: true,
        });
        const { database } = storage;
        const server = new TestAuthorizationServer(environment);
        const primary = options.primary;
        const scope = await ConnectionFixture.#scope(database, primary, environment);
        // serve the issuing package beside the fixture when it has one
        const events = await testEvents(database, environment);
        const issuingObjects =
            options.issuer === undefined
                ? undefined
                : await ConnectionFixture.#issuingObjects(
                      database,
                      options.issuer,
                      scope,
                      events,
                      environment,
                  );
        const issuing =
            issuingObjects === undefined || options.issuer === undefined
                ? undefined
                : ConnectionFixture.#serve(
                      issuingObjects,
                      ConnectionFixture.#issuing(options.issuer),
                  );

        // reach the served issuer as the connection installation, and other installations at the primary
        const consumer = ConnectionFixture.#installation(scope);
        const issuer = Egress.address(ISSUER_NAME, consumer.scope);
        const tokens = new TokenSource({
            environment,
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
                    : primary === undefined
                      ? ConnectionFixture.#unreached(address)
                      : primary.at(address),
        });

        // serve the connections, the vaults, and the integrations at their primary
        const objects = await ConnectionFixture.#objects(
            database,
            tokens,
            options,
            scope,
            events,
            environment,
        );
        const http = ConnectionFixture.#serve(objects, connectionPackage);

        return new ConnectionFixture({
            database,
            objects,
            issuing: issuingObjects,
            tokens,
            server,
            scope,
            vaultReference: await ConnectionFixture.#vault(database, scope, environment),
            http,
            simulation,
            environment,
            close: async () => {
                // stop serving, then close the database
                await issuing?.close();
                await http.close();
                await storage.close();
            },
        });
    }

    /** Create a vault in a scope. */
    static async #vault(
        database: DatabaseConnection,
        scope: ObjectReference,
        environment: Environment,
    ): Promise<ObjectReference> {
        // record the vault
        const id = vault.generateId(environment);
        const now = environment.clock.now();
        await database.transaction(
            async (transaction) =>
                await transaction.insert(vault.table).values({
                    id,
                    scope: scope.id,
                    name: "credentials",
                    definitionPackageId: vault.package.id,
                    definitionVersion: vault.package.version,
                    definitionName: "credentials",
                    spec: {},
                    createdAt: now,
                    updatedAt: now,
                }),
        );

        return vault.reference(scope.id, id);
    }

    /** Declare the database: connections, and integrations at the primary or replicated. */
    static #declare(options: ConnectionFixtureOptions) {
        // store the integrations at their primary, or replicate them as a space
        const isPrimary = options.primary === undefined;
        const replicated = [
            ...(isPrimary ? [] : integrationTables),
            ...SPACE_REPLICATED,
            ...(options.replicated ?? []),
        ];
        const tables = tablesOf({
            packages: [objectPackage],
            follows: [],
            tables: [
                ...connectionTables,
                ...vaultTables,
                placementTable,
                ...(isPrimary ? integrationTables : []),
                ...(options.issuer === undefined ? [] : options.issuer.tables),
                ...(options.tables ?? []),
            ],
        }).tables;

        return defineDatabase({
            name: "connection",
            tables: tables.filter((table) => !replicated.includes(table)),
            replicated,
        });
    }

    /** Serve connections and vaults, and integrations at their primary, else their replicas. */
    static async #objects(
        database: DatabaseConnection,
        tokens: TokenSource,
        options: ConnectionFixtureOptions,
        scope: ObjectReference,
        events: EventStore,
        environment: Environment,
    ): Promise<ObjectStore> {
        // read the space's primary
        const { primary } = options;

        // serve the connections as the space's connection installation, and the vaults at their base installation
        const serving = principal.space.reference(
            Scope.universe.id,
            ConnectionFixture.#principal(scope).id,
        );
        const installed = {
            ...(await testServerContext(Package.of(connectionPackage), serving.id, environment)),
            installation: INSTALLATION,
        };
        const vaults = await Installation.base(serving, vaultPackage, environment.crypto);

        // primary the account's integrations at the universe, where an account's inside lives, else serve the space
        return new ObjectStore({
            blobs: new MemoryBlobStorage(environment.crypto),
            environment,
            principal: ConnectionFixture.#installation(scope),
            ...(primary === undefined ? { partitions: [Scope.universe.id] } : {}),
            servers: [
                implementPackage(connectionPackage, {
                    objects: [
                        implementConnection(tokens),
                        authorizationRequest,
                        ...(primary === undefined ? [implementIntegration(tokens)] : []),
                    ],
                    ...(primary === undefined ? {} : { follows: [integrationFollow] }),
                }).at(installed),
                implementPackage(vaultPackage, {
                    objects: [implementVault(), implementSecret(), implementSecretVersion()],
                }).at(vaults),
            ],
            policies: [account, space, placement],
            database,
            callKey: testCallKey,
            events,
            origin: { package: Package.of(connectionPackage) },
            // store the chain above the serving space, which lives in the account, as its placed primary does
            chain: async () => ({
                kind: "home",
                parent: account.is(scope) ? scope.id : scope.scope,
            }),
            ...(primary === undefined
                ? {}
                : {
                      publisher: (followed: string) =>
                          Uplink.of((signer) => primary.primary(followed, signer).store),
                  }),
        });
    }

    /** Serve an issuer as the serving space's issuing installation, a store of the space. */
    static async #issuingObjects(
        database: DatabaseConnection,
        issuer: ObjectType,
        scope: ObjectReference,
        events: EventStore,
        environment: Environment,
    ): Promise<ObjectStore> {
        const serving = ConnectionFixture.#principal(scope);
        const issuing = {
            ...(await testServerContext(Package.of(ISSUING), serving.id, environment)),
            installation: ISSUER_INSTALLATION,
        };

        return new ObjectStore({
            blobs: new MemoryBlobStorage(environment.crypto),
            environment,
            principal: principal.installation.reference(serving.id, ISSUER_INSTALLATION),
            servers: [
                implementPackage(ConnectionFixture.#issuing(issuer), { objects: [issuer] }).at(
                    issuing,
                ),
            ],
            policies: [account, space, placement],
            database,
            callKey: testCallKey,
            events,
            origin: { package: ISSUING },
            chain: async () => ({ kind: "home", parent: serving.scope }),
        });
    }

    /** Answer clients sending their principal, leaving the controllers to the test. */
    static #serve(objects: ObjectStore, handle: PackageHandle): Server {
        return Server.start({
            environment: objects.environment,
            ...objects.package(handle),
            controllers: [],
            drainTimeout: 1000,
            health: new Health("connection"),
            resources: new ResourceContext(),
            authorizeMachine: async () => {},
            authenticate: async (request) =>
                ConnectionFixture.#authenticate(request, handle.id, objects.clock.now()),
        });
    }

    /** Declare the issuing package's handle with its issuer type. */
    static #issuing(issuer: ObjectType) {
        return PackageHandle.of(ISSUING, { objects: { issuer } });
    }

    /** Store the primary's account, or a space in an account, with their rows. */
    static async #scope(
        database: DatabaseConnection,
        primary: ConnectionFixtureOptions["primary"],
        environment: Environment,
    ): Promise<ObjectReference> {
        // store the owner's account with its row: the primary's own scope, or the one a space lives in
        const owner = principal.user.reference(Scope.universe.id, OWNER);
        const access = new AccessFixture(database, environment);
        const enclosing = primary?.enclosing;
        const holder =
            enclosing ?? account.reference(OWNER, Identifier.create("account", environment));
        await access.replicateScope(owner, principal.user);
        await access.replicateScope(holder, account.policy);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(account.table).values({
                    id: schema.identifier("account").parse(holder.id),
                    scope: OWNER,
                    handle: "owner",
                    name: "Owner",
                    residencyId: "eu",
                    kind: "personal",
                    createdAt: 0,
                    updatedAt: 0,
                }),
        );

        // store the primary's account as its scope, or a space in the account, with the serving space's row
        const isPrimary = primary === undefined;
        const scope = isPrimary ? holder : space.reference(holder.id, primary.space);
        const serving = space.reference(holder.id, ConnectionFixture.#principal(scope).id);
        await access.replicateScope(serving, space.policy);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(space.table).values({
                    id: schema.identifier("space").parse(serving.id),
                    scope: schema.identifier("account").parse(holder.id),
                    name: isPrimary ? "home" : "connections",
                    stack: STACKS.space,
                    createdAt: 0,
                    updatedAt: 0,
                }),
        );

        // bind the primary's owners here, and receive a placed space's from its primary's replica
        if (isPrimary) {
            await ConnectionFixture.#bindOwners(database, scope, environment);
        }

        return scope;
    }

    /** Let the person own a scope, and the space and its connection installation act as owners. */
    static async #bindOwners(
        database: DatabaseConnection,
        scope: ObjectReference,
        environment: Environment,
    ): Promise<void> {
        // bind the owner, the space and the connection installation in one transaction
        const owner = principal.user.reference(Scope.universe.id, OWNER);
        const now = environment.clock.now();
        const subjects = [
            owner,
            ConnectionFixture.#principal(scope),
            ConnectionFixture.#installation(scope),
        ];
        const [container] = await Scope.read(Snapshot.live(database), [scope.scope]);
        const partition =
            scope.scope === Scope.universe.id ? Scope.universe.id : container?.partition;
        const access = new AccessFixture(database, environment, partition);
        for (const subject of subjects) {
            await access.bindOwner(scope, subject, now);
        }
    }

    /** Read the connection installation serving a scope's connections, in its serving space. */
    static #installation(scope: ObjectReference): Subject {
        return principal.installation.reference(
            ConnectionFixture.#principal(scope).id,
            INSTALLATION,
        );
    }

    /** Refuse calling another space from a primary, which stores its own integrations. */
    static #unreached(address: string): never {
        throw new TypeError(`a primary calls no other connections, not ${address}`);
    }

    /** Read the space serving a scope: an account's primary, or the space itself. */
    static #principal(scope: ObjectReference): Subject {
        const id = account.is(scope) ? Account.home(scope.id) : scope.id;

        return principal.space.reference(Scope.universe.id, id);
    }

    /** Place a space in the fixture's account, answering the account. */
    async place(spaceId: string): Promise<ObjectReference> {
        // place the space, and store its row and scope in the account as the directory replica does
        const id = schema.identifier("space").parse(spaceId);
        const accountId = schema.identifier("account").parse(this.scope.id);
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(placementTable).values({ id, scope: accountId }),
        );
        const placed = space.reference(accountId, id);
        await new AccessFixture(this.database, this.environment).replicateScope(
            placed,
            space.policy,
        );
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(space.table).values({
                    id,
                    scope: accountId,
                    name: "connections",
                    stack: STACKS.space,
                    createdAt: 0,
                    updatedAt: 0,
                }),
        );

        // bind its owners where its row lives
        await ConnectionFixture.#bindOwners(this.database, placed, this.environment);

        return this.scope;
    }

    /** Close the database. */
    close(): Promise<void> {
        return this.#close();
    }

    /** Build the context of the owner calling in a session. */
    context(session = "session-one"): RequestContext {
        const subject = principal.user.reference(Scope.universe.id, OWNER);
        const now = this.environment.clock.now();

        return new RequestContext(new Request("https://test.local"), {
            environment: this.environment,
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

    /** Connect to these connections as the primary of every scope's integrations. */
    primary(
        spaceId: string,
    ): Pick<NonNullable<ConnectionFixtureOptions["primary"]>, "primary" | "at"> {
        // reach the primary's connections as the space's connection installation, at the primary's address
        const connections = principal.installation.reference(spaceId, INSTALLATION);
        const address = Egress.address(
            CONNECTION_INSTALLATION,
            ConnectionFixture.#principal(this.scope).id,
        );

        return {
            primary: (_scope, signer) =>
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
        return createClient(
            connectionPackage,
            {
                url: ORIGIN,
                headers: { authorization: `Bearer ${Subject.key(subject)}` },
                fetch: (request: Request) => this.http.fetch(request),
            },
            this.environment,
        );
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

    /** Bind a connection to an installation of its serving space, then lease it to a request. */
    async bindAndLease(reference: ObjectReference, request: Request) {
        // bind the consuming installation as the owner, then lease as it
        const consumer = principal.installation.reference(
            ConnectionFixture.#principal(this.scope).id,
            "installation-consumer",
        );
        await this.objects.mutate(this.context().access(), {
            id: RequestId.create(this.environment),
            calls: [
                connection.call("bindConsumer", {
                    ...connection.target({ scope: reference.scope, id: reference.id }),
                    consumer,
                }),
            ],
        });

        return this.lease(reference, request, consumer);
    }

    /** Lease a connection as a consumer bound to it, applying its credential to a request. */
    lease(reference: ObjectReference, request: Request, consumer: Subject) {
        return ConnectionRequest.lease(
            this.as(consumer),
            reference,
            request,
            this.environment.clock.now(),
            this.environment.crypto,
        );
    }

    /** Run the connection controller over every connection once, answering its waits. */
    async control(): Promise<(number | undefined)[]> {
        // find the controller and reconcile each key once
        const controller = this.objects.controllers().find((each) => each.name === connection.name);
        if (controller === undefined) {
            throw new TypeError("the connection package controls its connections");
        }

        return reconcileOnce(controller, this.objects.database);
    }

    /** Follow the store's replicas in a control loop until the test ends. */
    follow(): void {
        // run the replica controller, collecting every failure
        const controller = new AbortController();
        const failures: unknown[] = [];
        const loop = new ControlLoop(
            this.database,
            this.objects.controllers().filter((each) => each.name === "replica"),
            {
                clock: this.environment.clock,
                report: (_controller, _key, error) => failures.push(error),
            },
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
        for (const table of tablesOf({
            packages: [objectPackage],
            follows: [],
            tables: [...connectionTables, ...integrationTables],
        }).tables) {
            rows[table[TABLE].name] = await this.database.select().from(table);
        }

        return rows;
    }

    /** Read the live secret values of the fixture's vault, as their bound consumer. */
    async secrets(): Promise<string[]> {
        // read every secret of the vault by its owner, in the order it was stored
        const rows = await this.database
            .select({ name: secret.table.name })
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
                row.name,
            );
            values.push(present(value, `the value of secret ${row.name}`));
        }

        return values;
    }

    /** Read the connections as their table stores them. */
    connections() {
        return this.database.select().from(connection.table);
    }

    /** Register the test server's static confidential client, public or not, returning it. */
    async staticIntegration(
        options: { readonly isPublic?: boolean } = {},
    ): Promise<ObjectReference> {
        const created = await this.objects
            .mutate(this.context().access(), {
                id: RequestId.create(this.environment),
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

    /** Create the served issuer in the serving space, its consumer the connection installation. */
    async createIssuer(): Promise<Extract<IntegrationIssuer, { readonly kind: "installation" }>> {
        // create the issuer as the serving space
        const serving = ConnectionFixture.#principal(this.scope);
        const issuing = present(this.issuing, "the fixture's issuing installation");
        const [created] = await issuing.mutate(
            AccessContext.of(Caller.of(serving), this.environment.clock.now()),
            {
                id: RequestId.create(this.environment),
                calls: [testIssuer.call("create", testIssuer.in(serving.id))],
            },
        );
        const reference = testIssuer.reference(serving.id, present(created, "the issuer").id);

        // let the connection installation call it
        const consumer = ConnectionFixture.#installation(this.scope);
        await new AccessFixture(this.database, this.environment).replicateRelationship(
            reference,
            "consumer",
            consumer,
        );

        return {
            kind: "installation",
            package: ISSUING,
            name: ISSUER_NAME,
            issuer: reference,
        };
    }

    /** Authenticate a request to a package as the principal in its bearer token, at a time. */
    static #authenticate(request: Request, audience: PackageId, now: number): Authentication {
        const subject = Subject.read(TestBearer.read(request));

        return TestBearer.authenticate(subject, audience, now);
    }
}
