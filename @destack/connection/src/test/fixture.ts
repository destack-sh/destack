import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/service/authentication";
import { space } from "@destack/account/object";
import { type Caller, principal, relation, Role } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { journal } from "@destack/audit/stack";
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
import { LocalKeyring } from "@destack/identity";
import { defineObject, INTRINSIC } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import type {} from "@destack/package/import-meta";
import { present, schema, Clock } from "@destack/schema";
import { binding, capture, deployment, installation } from "@destack/stack/object";
import { VaultKeystore } from "@destack/vault/key";
import { ConsumerSecret, secret, serveSecrets, vault } from "@destack/vault/server";
import { vaultTables } from "@destack/vault/stack";
import { ResourceContext } from "@destack/resource/context";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server, ServiceContext } from "@destack/service/server";
import { reconciliation, TestBearer, testCallKey } from "@destack/service/test";
import { type ObjectReference, Scope, Subject } from "@destack/sync";
import { v7 } from "uuid";
import { createClient, type ServiceClient } from "@destack/service/client";
import { type Integration, integration, connection } from "../object/index.ts";
import type { InstallationIssuer } from "../installation/installation.ts";
import { Pkce } from "@destack/service/authentication";
import type { Fetch } from "@destack/service";
import {
    IntegrationCopy,
    integrationShape,
    Lender,
    serveIntegrations,
    serveConnections,
    TokenSource,
} from "../server/index.ts";
import { connectionService } from "../service/index.ts";
import { integrationTables, connectionTables } from "../stack/index.ts";
import { TestVault } from "@destack/vault/test";

/** The issuer of the test authorization server. */
export const ISSUER = "https://auth.test";

/** A protected resource whose metadata lists the test authorization server. */
export const RESOURCE = "https://mcp.test/mcp";

/** How long the test server's access tokens last, in seconds. */
const TOKEN_SECONDS = 3600;

/** The origin clients call a fixture's connection service at, in process. */
const ORIGIN = "https://connection.test";

/** The machine serving the space, consuming its credentials and keeping its vault keys. */
const MACHINE = schema.identifier("machine").parse("machine-01a1134b-0000-7000-8000-00000000c4ed");

/** The bytes of a vault keyring's root key. */
const ROOT_KEY_BYTES = 32;

/** The tables a vault's checks read, which a space's database copies. */
const SPACE_COPIES: readonly Table[] = [
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
}).methods((method) => ({ get: method.get("read") }));

/** A database of connections and their vaults, as a space or the account service keeps it. */
export const connectionDatabase = defineDatabase({
    name: "connection",
    tables: [...connectionTables, ...integrationTables, ...vaultTables, journal],
    copies: SPACE_COPIES,
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
    /** The installation issuers of the test server's integrations. */
    readonly issuers?: (fetch: Fetch) => readonly InstallationIssuer[];
    /** The tables of services sharing the database, which read the connections as their copies. */
    readonly tables?: readonly Table[];
    /** The home of other scopes' integrations and the space copying them, as a space has. */
    readonly home?: {
        readonly home: (scope: string, signer?: Subject) => ServiceClient<typeof connectionService>;
        readonly space: string;
        /** The home scope enclosing the space, such as its account, absent in the universe. */
        readonly enclosing?: ObjectReference;
    };
    /** The tables sharing services copy, such as an installation's integration copies. */
    readonly copies?: readonly Table[];
}

/** A connection service over a database with a scope, owner, vault and test server. */
export class ConnectionFixture {
    /** The database. */
    readonly database: DatabaseConnection;
    /** The object store keeping the connections. */
    readonly objects: ObjectStore;
    /** The connections' tokens. */
    readonly tokens: TokenSource;
    /** The lender of the connections. */
    readonly lender: Lender;
    /** The authorization server. */
    readonly server: TestAuthorizationServer;
    /** The scope the connections live in. */
    readonly scope: ObjectReference;
    /** The vault in the scope. */
    readonly vaultReference: ObjectReference;
    /** The connection service served in process to clients sending their principal. */
    readonly http: Server;
    /** Close the database. */
    readonly #close: () => Promise<void>;

    /** Hold an open service. */
    private constructor(
        fields: Omit<
            ConnectionFixture,
            | "close"
            | "context"
            | "control"
            | "connections"
            | "dump"
            | "home"
            | "place"
            | "secrets"
            | "staticIntegration"
        > & { readonly close: () => Promise<void> },
    ) {
        // keep the service, its vault, server and scope
        this.database = fields.database;
        this.objects = fields.objects;
        this.tokens = fields.tokens;
        this.lender = fields.lender;
        this.server = fields.server;
        this.scope = fields.scope;
        this.vaultReference = fields.vaultReference;
        this.http = fields.http;
        this.#close = fields.close;
    }

    /** Open a connection service over a fresh database. */
    static async open(options: ConnectionFixtureOptions = {}): Promise<ConnectionFixture> {
        // keep the database and the authorization server
        const storage = await TestDatabase.create("sqlite", ConnectionFixture.#declare(options), {
            isMigrated: true,
        });
        const { database } = storage;
        const server = new TestAuthorizationServer();
        const issuers = options.issuers ?? (() => []);
        const home = options.home;
        const tokens = new TokenSource({
            consumer: principal.machine.reference(Scope.universe.id, MACHINE),
            fetch: server.fetch,
            issuers: issuers(server.fetch),
            ...(home === undefined ? {} : { home: home.home }),
        });

        // serve the connections, the vaults, and the integrations at their home
        const root = crypto.getRandomValues(new Uint8Array(ROOT_KEY_BYTES));
        const keyring = await LocalKeyring.import(1, new Map([[1, root]]));
        const keystore = new VaultKeystore(database, keyring, MACHINE);
        const scope = await ConnectionFixture.#scope(database, home?.enclosing);
        const objects = ConnectionFixture.#objects(database, tokens, keystore, home, scope);
        const http = ConnectionFixture.#serve(objects);

        return new ConnectionFixture({
            database,
            objects,
            tokens,
            lender: new Lender(objects, tokens),
            server,
            scope,
            vaultReference: await ConnectionFixture.#vault(database, keystore, scope),
            http,
            close: async () => {
                await http.close();
                await storage.close();
            },
        });
    }

    /** Keep a vault with its key in a scope. */
    static async #vault(
        database: DatabaseConnection,
        keystore: VaultKeystore,
        scope: ObjectReference,
    ): Promise<ObjectReference> {
        // record the vault
        const id = schema.identifier("vault").parse(vault.generateId());
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

        // start its key, as its creation does
        await TestVault.startKey(database, keystore, { id, scope: scope.id }, now);

        return vault.reference(scope.id, id);
    }

    /** Declare the database: connections, and integrations at their home or copied in a space. */
    static #declare(options: ConnectionFixtureOptions) {
        // keep the integrations at their home, or copy them as a space
        const isHome = options.home === undefined;
        const copies = [
            ...(isHome ? [] : integrationTables),
            ...SPACE_COPIES,
            ...(options.copies ?? []),
        ];
        const tables = [
            ...connectionTables,
            ...vaultTables,
            placementTable,
            journal,
            ...(isHome ? integrationTables : []),
            ...(options.tables ?? []),
        ];

        return defineDatabase({
            name: "connection",
            tables: tables.filter((table) => !copies.includes(table)),
            copies,
        });
    }

    /** Serve connections and vaults, and integrations at their home, else follow their copies. */
    static #objects(
        database: DatabaseConnection,
        tokens: TokenSource,
        keystore: VaultKeystore,
        home: ConnectionFixtureOptions["home"],
        scope: ObjectReference,
    ): ObjectStore {
        return new ObjectStore({
            clock: Clock.system,
            principal: principal.space.reference(Scope.universe.id, scope.id),
            objects: {
                ...serveConnections(tokens),
                vault,
                ...serveSecrets(keystore),
                ...(home === undefined ? { integration: serveIntegrations(tokens) } : {}),
            },
            policies: [space, placement],
            database,
            callKey: testCallKey,
            origin: { package: connectionService.package, service: connectionService.name },
            shapes: [integrationShape],
            ...(home === undefined
                ? {}
                : {
                      subscriber: {
                          watches: [connection.table],
                          subscriptions: () =>
                              IntegrationCopy.subscriptions(database, home.space, home.home),
                      },
                  }),
        });
    }

    /** Answer clients sending their principal, leaving the controllers to the test. */
    static #serve(objects: ObjectStore): Server {
        return Server.start({
            clock: Clock.system,
            ...objects.implement(connectionService, objects.routed, objects.controllers(), {}),
            controllers: [],
            audience: connectionService.package.id,
            drainTimeout: 1000,
            health: new Health("connection"),
            resources: new ResourceContext(),
            authorizeMachine: async () => {},
            authenticate: async (request) => ConnectionFixture.#authenticate(request),
        });
    }

    /** Keep a scope with a space's name below its parent, owned by the person and the space. */
    static async #scope(
        database: DatabaseConnection,
        enclosing: ObjectReference | undefined,
    ): Promise<ObjectReference> {
        // record the scope below its enclosing scope with the person owning it
        const scope = space.reference(enclosing?.id ?? Scope.universe.id, `space-${v7()}`);
        const access = new AccessFixture(database);
        if (enclosing !== undefined) {
            await access.copyScope(enclosing);
        }
        await access.copyScope(scope);
        await access.copyOwner(scope, principal.user.reference(Scope.universe.id, "user-owner"));
        await Transaction.run(database, (transaction) =>
            Role.keepOwner(
                transaction,
                scope,
                principal.space.reference(Scope.universe.id, scope.id),
                Date.now(),
            ),
        );

        return scope;
    }

    /** Place a space in the fixture's scope, as the directory places the spaces of an account. */
    async place(spaceId: string): Promise<void> {
        await this.database.insert(placementTable).values({ id: spaceId, scope: this.scope.id });
    }

    /** Close the database. */
    close(): Promise<void> {
        return this.#close();
    }

    /** Build the context of the owner calling in a session. */
    context(session = "session-one"): ServiceContext {
        const subject = principal.user.reference(Scope.universe.id, "user-owner");
        const now = Date.now();

        return new ServiceContext(new Request("https://test.local"), {
            clock: Clock.system,
            audience: connectionService.package.id,
            scope: this.scope.id,
            caller: {
                subject,
                subjects: [subject],
                credential: {
                    kind: "session",
                    id: session,
                    audience: connectionService.package.id,
                    verifiedAt: now,
                    expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                },
            },
            resources: new ResourceContext(),
        });
    }

    /** Connect to this service as the home of every scope's integrations. */
    home(
        spaceId: string,
    ): (scope: string, signer?: Subject) => ServiceClient<typeof connectionService> {
        return (_scope, signer) => {
            // send the signer, or else the space, in the bearer
            const subject = signer ?? principal.space.reference(Scope.universe.id, spaceId);

            return createClient(connectionService, {
                url: ORIGIN,
                headers: { authorization: `Bearer ${Subject.key(subject)}` },
                fetch: (request: Request) => this.http.fetch(request),
            });
        };
    }

    /** Run the connection controller over every connection once, answering its waits. */
    async control(): Promise<(number | undefined)[]> {
        // find the controller and reconcile each key once
        const controller = this.objects.controllers().find((each) => each.name === connection.name);
        if (controller === undefined) {
            throw new TypeError("the connection service controls its connections");
        }
        const waits: (number | undefined)[] = [];
        for (const key of await controller.list()) {
            waits.push(await controller.reconcile(key, reconciliation()));
        }

        return waits;
    }

    /** Read every row of the service's tables, the journal included, by table. */
    async dump(): Promise<Readonly<Record<string, unknown[]>>> {
        const rows: Record<string, unknown[]> = {};
        for (const table of [...connectionTables, ...integrationTables, journal]) {
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
        const { consumer } = this.tokens.options;
        const values: string[] = [];
        for (const row of rows) {
            const value = await ConsumerSecret.read(
                this.objects,
                this.vaultReference,
                row.id,
                consumer,
            );
            values.push(present(value, `the value of secret ${row.id}`));
        }

        return values;
    }

    /** Read the connections as their table keeps them. */
    connections() {
        return this.database.select().from(connection.table);
    }

    /** Register the test server's static confidential client with a visibility, returning it. */
    async staticIntegration(
        visibility: Integration["visibility"] = "private",
    ): Promise<ObjectReference> {
        const created = await this.objects
            .mutate(this.context().caller(), {
                id: RequestId.create(Date.now()),
                calls: [
                    integration.call("create", {
                        scope: this.scope.id,
                        issuer: ISSUER,
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
                        visibility,
                    }),
                ],
            })
            .then(([written]) => written);

        return integration.reference(this.scope.id, created.id);
    }

    /** Authenticate a request as the principal in its bearer token. */
    static #authenticate(request: Request): Caller {
        const subject = Subject.read(TestBearer.read(request));

        return TestBearer.authenticate(subject, connectionService.package.id, Date.now());
    }
}
