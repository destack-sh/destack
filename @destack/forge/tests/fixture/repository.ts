import objectPackage from "@destack/object/package";
import accountPackage from "@destack/account/package";
import { AccessFixture } from "@destack/access/test";
import { Secret } from "@destack/vault";
import { PackageMount, SpaceMount } from "@destack/object";
import {
    Health,
    implementPackage,
    ObjectStore,
    RemoteStore,
    Server,
    type PackageHandler,
} from "@destack/object/server";
import connectionPackage from "@destack/connection/package";
import { integration, type IntegrationIssuer } from "@destack/connection";
import { test } from "@destack/test";
import { Package } from "@destack/package";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import vaultServer from "@destack/vault/server";
import { AccessContext, Caller, principal } from "@destack/access";
import * as accountTest from "@destack/account/test";
import { testEvents, testCallKey } from "@destack/object/test";
import { LocalBucket } from "@destack/bucket/bun";
import { PackageStore } from "@destack/build/store";
import { type DatabaseConnection, eq, type Select, Snapshot } from "@destack/db";
import { Account, claim, Identity } from "@destack/account/object";
import { TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, present, schema, zip } from "@destack/schema";
import { RequestId, type Publisher, Scope, Subject, Uplink } from "@destack/sync";
import { INSTALLATION_KEY, TokenVerifier } from "@destack/access/token";
import { createClient } from "@destack/object/client";
import forgePackage, { FORGE_INSTALLATION } from "../../src/package.ts";
import { GitHubApp } from "../../src/github/index.ts";
import { LocalGitStorage } from "../../src/local/index.ts";
import { issuer, reference, repository } from "../../src/object/index.ts";
import { Forge } from "../../src/server/index.ts";
import {
    bearHome,
    homeReplicas,
    homeDatabase,
    provisionVault,
    readHome,
    RESIDENCY,
    type ResidencyHome,
} from "../../src/test/home.ts";
import { ConnectionSpace } from "./space.ts";
import { APP_ID, GITHUB_API, GitHubStandIn, WEBHOOK_SECRET } from "./github.ts";
import { githubPrivateKey } from "./key.ts";
import { Clock } from "@destack/environment";

/** The identifiers the fixture's accounts, users and machines take. */
export const ids = {
    account: accountTest.ids.account,
    other: accountTest.ids.other,
    owner: accountTest.ids.owner,
    machine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000008"),
    otherMachine: schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-000000000009"),
};

/** The controllers the fixture forge runs, leaving the others to each test. */
const RUNNING_CONTROLLERS: ReadonlySet<string> = new Set(["replica", "github"]);

/** The origin clients call the fixture forge at. */
const ORIGIN = "https://forge.test";

/** The GitHub instance the fixture App is registered at. */
const GITHUB_ORIGIN = "https://github.com";

/** The longest a test waits for the forge's replicas, in milliseconds. */
const REPLICA_MILLISECONDS = 10_000;

/** The origin the platform's cloud machine answers the spaces it runs at. */
const SPACES_ORIGIN = "https://spaces.test";

/** The setup URL GitHub returns to after an installation. */
const SETUP_URL = "https://accounts.test/github/setup";

/** The installation of the fixture GitHub App in the owner's GitHub organisation. */
export const INSTALLATION = 4242;

/** The installation of the fixture GitHub App in another account's GitHub organisation. */
export const OTHER_INSTALLATION = 5151;

/** The owner's accounts whose home spaces connect the App's installations, in order. */
const CONNECTED_ACCOUNTS = [ids.account, ids.other];

/** The installations of the fixture GitHub App the owner's accounts connect, in order. */
const INSTALLATIONS = [INSTALLATION, OTHER_INSTALLATION];

/** An account of one test with its owner, a reader of its repositories and a stranger to it. */
export interface Tenant {
    /** The account. */
    readonly accountId: Identifier<"account">;
    /** The account's owner. */
    readonly owner: Identifier<"user">;
    /** A user who reads and pulls the account's repositories. */
    readonly reader: Identifier<"user">;
    /** A user without access to the account. */
    readonly stranger: Identifier<"user">;
}

/** A forge installation serving accounts' repositories. */
export class RepositoryFixture implements AsyncDisposable {
    /** The account package with the accounts and the directory. */
    readonly accounts: accountTest.AccountFixture;
    /** The forge's database of repositories and references. */
    readonly database: DatabaseConnection;
    /** The connections of the owner's accounts to their GitHub App installations, in order. */
    readonly connections: Identifier<"connection">[];
    /** The forge's platform storage. */
    readonly storage: LocalGitStorage;
    /** The GitHub REST API stand-in. */
    readonly github: GitHubStandIn;
    /** The residency home's store the forge extends. */
    readonly objects: ObjectStore;
    /** The HTTP server serving the forge. */
    readonly http: Server;
    /** The failures of committed work to settle, in order. */
    readonly reported: unknown[];
    /** Aborted with the controllers' first failure, such as a refused replica, failing settles. */
    readonly failure: AbortSignal;
    /** The forge installation's space, below which it stores its replicas. */
    readonly space: string;
    /** Wait until every replica the forge lists has a complete snapshot. */
    readonly ready: (signal: AbortSignal) => Promise<void>;
    /** The databases, servers and directories closed with the fixture. */
    readonly #disposal: AsyncDisposableStack;

    /** Take a started forge. */
    private constructor(
        fields: Omit<
            RepositoryFixture,
            | "connect"
            | "connectMachine"
            | "connectSpace"
            | "createSpace"
            | "settle"
            | "identified"
            | "tenant"
            | typeof Symbol.asyncDispose
        >,
        disposal: AsyncDisposableStack,
    ) {
        this.accounts = fields.accounts;
        this.database = fields.database;
        this.connections = fields.connections;
        this.storage = fields.storage;
        this.github = fields.github;
        this.objects = fields.objects;
        this.http = fields.http;
        this.reported = fields.reported;
        this.failure = fields.failure;
        this.space = fields.space;
        this.ready = fields.ready;
        this.#disposal = disposal;
    }

    /** Start a forge on sqlite with the owner's accounts connected to GitHub. */
    static async open(): Promise<RepositoryFixture> {
        // serve the account package beside the forge's database
        const disposal = new AsyncDisposableStack();
        const accounts = disposal.use(
            await accountTest.AccountFixture.open({
                dialect: "sqlite",
                policies: [repository.policy, reference.policy],
            }),
        );
        const forgeStorage = await TestDatabase.create("sqlite", homeDatabase, {
            isMigrated: true,
        });
        disposal.defer(() => forgeStorage.close());

        // install the forge in the residency's home, following and calling the account package as the installation, and read the home
        const installation = await accounts.install([], RESIDENCY);
        const client = accounts.installation(installation);
        const home = await readHome(accounts, installation.scope);

        // serve the home space of each of the owner's accounts and connect its installation of the App
        let issuing: Server | undefined;
        const endpoint = {
            url: ORIGIN,
            issue: (request: Request, from: Subject) => {
                // send the installation's call to the forge served at its origin, below no package mount
                const headers = new Headers(request.headers);
                headers.set("x-installation", Subject.key(from));
                const url = request.url.replace(PackageMount.path(forgePackage.id), "");

                return present(issuing, "the forge's server").fetch(
                    new Request(url, { method: request.method, headers, body: request.body }),
                );
            },
        };
        const spaces = await RepositoryFixture.#homes(accounts, installation, endpoint);
        spaces.forEach((space) => disposal.use(space));

        // serve the repositories with local storage and the GitHub App against the stand-in, recording calls
        const github = new GitHubStandIn();
        const reported: unknown[] = [];
        const database = forgeStorage.database;
        const implementation = await RepositoryFixture.#implement({
            database,
            directory: Snapshot.live(accounts.database),
            client,
            installation,
            home,
            spaces,
            github,
            reported,
            disposal,
        });

        // serve the forge, replica the home's account chain, then give birth to the home below it
        const failure = new AbortController();
        const tokens = new TokenVerifier({
            issuer: accountTest.ISSUER,
            keys: await Identity.keys(Snapshot.live(accounts.database), Scope.universe.id),
            authority: { kind: "universe" },
            audience: forgePackage.id,
        });
        const http = RepositoryFixture.#serve(implementation.implementation, failure, tokens);
        issuing = http;
        disposal.defer(() => http.close());
        await accounts.settle(implementation.objects, failure.signal);
        await bearHome(implementation.objects, home);
        await accounts.settle(implementation.objects, failure.signal);

        // create the App's issuer in the home, then connect each account's installation through it
        const issued = await RepositoryFixture.#createIssuer(
            implementation.objects,
            home.id,
            spaces,
        );
        const connections = await RepositoryFixture.#connect(
            spaces,
            issued,
            implementation.objects.principal,
        );
        await accounts.settle(implementation.objects, failure.signal);

        return new RepositoryFixture(
            {
                accounts,
                database,
                connections,
                storage: implementation.storage,
                github,
                objects: implementation.objects,
                http,
                reported,
                failure: failure.signal,
                space: installation.scope,
                ready: present(implementation.implementation.ready, "the forge's readiness"),
            },
            disposal,
        );
    }

    /** Create an account of the owner with a reader and a stranger, once replicated. */
    async tenant(): Promise<Tenant> {
        // create the account as the owner
        const owner = this.accounts.user(ids.owner, accountPackage);
        const created = await owner.account.create({
            scope: ids.owner,
            requestId: RequestId.create(Date.now()),
            handle: `tenant-${Identifier.create("account").slice(-12)}`,
            name: "Tenant",
            kind: "shared",
            residencyId: RESIDENCY,
        });

        // let a reader read and pull its repositories
        const reader = schema.identifier("user").parse(Identifier.create("user"));
        const access = this.accounts.user(ids.owner, objectPackage);
        const role = await access.role.create({
            scope: created.id,
            requestId: RequestId.create(Date.now()),
            name: "reader",
            description: "Reads and pulls repositories",
            permissions: [repository.permission("read"), repository.permission("pull")],
        });
        await owner.account.grant({
            scope: ids.owner,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            role: role.id,
            subject: principal.user.reference(Scope.universe.id, reader),
        });

        // wait for the forge's replicas of both
        await this.settle();
        const stranger = schema.identifier("user").parse(Identifier.create("user"));

        return { accountId: created.id, owner: ids.owner, reader, stranger };
    }

    /** Close the forge, its servers and databases, and remove its directories. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposal.disposeAsync();
    }

    /** Serve the home spaces of the owner's accounts on the platform's cloud machine. */
    static async #homes(
        accounts: accountTest.AccountFixture,
        installation: Subject,
        endpoint: Parameters<typeof ConnectionSpace.open>[3],
    ): Promise<ConnectionSpace[]> {
        // serve the home spaces with their owner and the installation
        const spaces = await Promise.all(
            CONNECTED_ACCOUNTS.map(async (id) =>
                ConnectionSpace.open(
                    { id, owner: ids.owner },
                    Account.home(id),
                    installation,
                    endpoint,
                ),
            ),
        );

        // place each on the cloud machine
        await accountTest.TestDirectory.publish(
            accounts.database,
            accountTest.ids.machine,
            accounts.platform,
            SPACES_ORIGIN,
        );
        for (const space of spaces) {
            await accountTest.TestDirectory.place(accounts.database, {
                id: space.id,
                scope: space.accountId,
                machine: accountTest.ids.machine,
                epoch: 1,
            });
        }

        return spaces;
    }

    /** Connect each account's App installation as the owner, and bind it to the forge. */
    static async #connect(
        spaces: readonly ConnectionSpace[],
        issued: IntegrationIssuer,
        forge: Subject,
    ): Promise<Identifier<"connection">[]> {
        const connections: Identifier<"connection">[] = [];
        for (const [space, installed] of zip(spaces, INSTALLATIONS)) {
            // register the App and create its installation connection
            const owner = space.connect(ids.owner);
            const accountId = space.accountId;
            const app = await owner.integration.create({
                scope: accountId,
                requestId: RequestId.create(Date.now()),
                issuer: issued,
                endpoints: {
                    installation: `${GITHUB_ORIGIN}/apps/fixture/installations/new`,
                    isIssuerReturned: false,
                },
                clientId: APP_ID,
                authentication: "none",
                redirectUris: [SETUP_URL],
            });
            const created = await owner.connection.create({
                scope: accountId,
                requestId: RequestId.create(Date.now()),
                protocol: { kind: "installation" },
                integration: integration.reference(accountId, app.id),
                vault: space.vault,
            });

            // authorize it and complete GitHub's setup redirect
            const { url } = await owner.connection.authorize({
                scope: accountId,
                id: created.id,
                requestId: RequestId.create(Date.now()),
                redirectUri: SETUP_URL,
            });
            const state = present(
                new URL(url).searchParams.get("state"),
                "the installation's state",
            );
            await owner.connection.complete({
                scope: accountId,
                id: created.id,
                requestId: RequestId.create(Date.now()),
                parameters: { installation_id: String(installed), setup_action: "install", state },
            });

            // bind the connection to the forge as its consumer, as the owner does before connecting repositories
            await owner.connection.bindConsumer({
                scope: accountId,
                id: created.id,
                requestId: RequestId.create(Date.now()),
                consumer: forge,
            });
            connections.push(created.id);
        }

        return connections;
    }

    /** Create the GitHub App's issuer in the residency home, for the spaces' connections. */
    static async #createIssuer(
        objects: ObjectStore,
        home: string,
        spaces: readonly ConnectionSpace[],
    ): Promise<IntegrationIssuer> {
        // create the issuer as the home
        const owner = principal.space.reference(Scope.universe.id, home);
        const [created] = await objects.mutate(AccessContext.of(Caller.of(owner), Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [issuer.call("create", issuer.in(home))],
        });
        const appIssuer = issuer.reference(home, present(created, "the App's issuer").id);

        // let each space's connection installation call it
        const access = new AccessFixture(objects.database);
        for (const space of spaces) {
            await access.replicateRelationship(home, appIssuer, "consumer", space.consumer);
        }

        return {
            kind: "installation",
            package: Package.of(forgePackage),
            name: FORGE_INSTALLATION,
            issuer: appIssuer,
        };
    }

    /** Implement the forge over local storage and a bucket, calling home spaces as itself. */
    static async #implement(context: {
        readonly database: DatabaseConnection;
        readonly directory: Snapshot;
        readonly client: ReturnType<accountTest.AccountFixture["installation"]>;
        readonly installation: Subject;
        readonly home: ResidencyHome;
        readonly spaces: readonly ConnectionSpace[];
        readonly github: GitHubStandIn;
        readonly reported: unknown[];
        readonly disposal: AsyncDisposableStack;
    }): Promise<{
        readonly objects: ObjectStore;
        readonly implementation: PackageHandler;
        readonly storage: LocalGitStorage;
    }> {
        // store the platform repositories and the builds in a temporary directory
        const { client, github, spaces, reported, disposal } = context;
        const directory = await mkdtemp(join(tmpdir(), "destack-forge-"));
        disposal.defer(() => rm(directory, { recursive: true, force: true }));
        const storage = new LocalGitStorage(join(directory, "storage"));
        const bucket = disposal.use(
            await LocalBucket.open(join(directory, "bucket"), "forge-test", Clock.system),
        );

        // extend the residency home's store with the forge and the GitHub App against the stand-in
        const uplink = Uplink.of(() => client.store);
        const forge = new Forge({
            home: context.home.id,
            store: new PackageStore(bucket),
            endpoint: new URL(ORIGIN),
            storage,
            github: new GitHubApp({
                clock: Clock.system,
                id: APP_ID,
                key: Secret.of(githubPrivateKey()),
                secret: Secret.of(WEBHOOK_SECRET),
                api: GITHUB_API,
                fetch: github.fetch,
            }),
            fetch: github.fetch,
            vault: await provisionVault(context.database, context.home.id, context.installation),
            consumer: context.installation,
        });

        // follow and call the account package as the installation, beside the home's vaults and replicas
        const { follows, ...placed } = homeReplicas(context.home, RESIDENCY, uplink);
        const definition = forge.definition();
        const store = new ObjectStore({
            servers: [
                implementPackage(forgePackage, {
                    ...definition,
                    follows: [...(definition.follows ?? []), ...follows],
                }),
                vaultServer,
            ],
            clock: Clock.system,
            principal: context.installation,
            database: context.database,
            callKey: testCallKey,
            events: await testEvents(context.database),
            origin: { package: Package.of(forgePackage) },
            claims: {
                object: claim,
                home: RemoteStore.of(client.store, Scope.universe.id),
            },
            uplink,
            ...placed,
            // follow the serving machine as itself, the universe and the home's chain through the uplink, an account's connections at its home
            publisher: (scope) => {
                // reach the serving machine's chain as the machine
                if (scope === context.home.machine.id) {
                    return context.home.machine.publisher;
                }
                // reach the universe and the home's chain through the uplink
                else if (scope === Scope.universe.id || scope === context.home.id) {
                    return uplink;
                }

                return connectionHome(spaces, scope);
            },
            report: (error) => reported.push(error),
            machine: ids.machine,
        });

        return {
            objects: store,
            implementation: store.package(forgePackage),
            storage,
        };
    }

    /** Wait until the forge's replicas have the account package's writes so far. */
    async settle(): Promise<void> {
        // replicate the account package's writes, then sync every listed replica, failing once a controller fails
        await this.accounts.settle(this.objects, this.failure);
        const signal = AbortSignal.any([AbortSignal.timeout(REPLICA_MILLISECONDS), this.failure]);
        await this.ready(signal);
        this.failure.throwIfAborted();
        if (signal.aborted) {
            throw new Error("the forge's replicas did not sync");
        }
    }

    /** Wait until the forge records a repository's GitHub identifier, and read it. */
    async identified(id: Identifier<"repository">): Promise<Select<typeof repository.table>> {
        // read the repository at each commit until it has the identifier
        const table = repository.table;
        const read = async () => {
            const [row] = await this.database.select().from(table).where(eq(table.id, id));

            return present(row, `repository ${id}`);
        };
        const isIdentified = await this.database.log.until(
            async () => (await read()).providerRepositoryId !== null,
            AbortSignal.timeout(REPLICA_MILLISECONDS),
        );
        if (!isIdentified) {
            throw new Error(`the forge did not identify ${id}`);
        }

        return read();
    }

    /** Connect to the forge as a user. */
    connect(userId: string) {
        return createClient(forgePackage, {
            url: ORIGIN,
            headers: { "x-user": userId },
            fetch: (request) => this.http.fetch(request),
        });
    }

    /** Connect to the forge as a machine of the account. */
    connectMachine(machine: string) {
        return createClient(forgePackage, {
            url: ORIGIN,
            headers: { "x-machine": machine },
            fetch: (request) => this.http.fetch(request),
        });
    }

    /** Create a space in an account through the account package, with its identity started. */
    async createSpace(accountId: Identifier<"account">): Promise<Identifier<"space">> {
        const id = schema.identifier("space").parse(Identifier.create("space"));
        await this.accounts.user(ids.owner, accountPackage).space.create({
            accountId,
            id,
            requestId: RequestId.create(Date.now()),
            name: `space-${id.slice(-12)}`,
        });
        await this.accounts.startIdentity(
            id,
            this.accounts.caller(await this.accounts.serving(id)),
        );

        return id;
    }

    /** Connect to the forge as a space, with the tokens the universe issues for its assertions. */
    connectSpace(space: Identifier<"space">) {
        const subject = principal.space.reference(Scope.universe.id, space);
        const route = (request: Request) =>
            request.url.startsWith(accountTest.ISSUER)
                ? this.accounts.fetch(request)
                : this.http.fetch(request);

        return createClient(forgePackage, {
            url: ORIGIN,
            fetch: this.accounts.keystore
                .source(this.accounts.database, subject, {
                    issuer: accountTest.ISSUER,
                    send: route,
                })
                .fetchGranted(forgePackage.id),
        });
    }

    /** Serve the forge to the user, machine or space its x- header has, or anonymously. */
    static #serve(
        implementation: PackageHandler,
        failure: AbortController,
        tokens: TokenVerifier,
    ): Server {
        // run the replicas and the identification of GitHub repositories, and settle the rest by hand when each test chooses
        const audience = implementation.package.id;
        const http = Server.start({
            clock: Clock.system,
            ...implementation,
            controllers: (implementation.controllers ?? []).filter((controller) =>
                RUNNING_CONTROLLERS.has(controller.name),
            ),
            resources: new ResourceContext(),
            health: new Health("forge"),
            drainTimeout: 1000,
            report: (error) => failure.abort(error),
            authorizeMachine: async () => {},
            authenticate: async (request) => {
                // verify a bearer token the universe issued, as a space's calls carry
                if (request.headers.has("authorization")) {
                    return tokens.authenticate(request, Date.now());
                }

                // serve anonymous requests without a credential
                const caller = RepositoryFixture.#caller(request);
                if (caller === undefined) {
                    return undefined;
                }

                // verify the caller for a minute
                const { subject, kind } = caller;
                const now = Date.now();

                return {
                    caller: Caller.of(subject),
                    credential: {
                        kind,
                        id: subject.id,
                        audience,
                        verifiedAt: now,
                        expiresAt: now + 60_000,
                    },
                };
            },
        });

        return http;
    }

    /** Read the machine, user or space of a fixture request, with its credential kind. */
    static #caller(
        request: Request,
    ): { readonly subject: Subject; readonly kind: string } | undefined {
        const machine = request.headers.get("x-machine");
        const userId = request.headers.get("x-user");
        const installation = request.headers.get("x-installation");

        // name the machine by its key
        if (machine !== null) {
            return {
                subject: principal.machine.reference(Scope.universe.id, machine),
                kind: "machine-key",
            };
        }
        // name the installation by its key
        else if (installation !== null) {
            return { subject: Subject.read(installation), kind: INSTALLATION_KEY };
        }
        // name the user
        else if (userId !== null) {
            return {
                subject: principal.user.reference(Scope.universe.id, userId),
                kind: "session",
            };
        }
        // serve a request of nobody anonymously, as GitHub's deliveries come
        else {
            return undefined;
        }
    }
}

/** The tests of one file, sharing one forge they each create their own accounts in. */
export const repositoryTest = test.extend<{ forge: RepositoryFixture }>({
    forge: [
        async ({ task: _task }, use) => {
            await using forge = await RepositoryFixture.open();
            await use(forge);
        },
        { scope: "file" },
    ],
});

/** Stream an account's connection replicas from the stand-in of its home. */
function connectionHome(spaces: readonly ConnectionSpace[], accountId: string): Publisher {
    const client = createClient(connectionPackage, {
        url: `${SpaceMount.url(SPACES_ORIGIN, Account.home(accountId))}${PackageMount.path(connectionPackage.id)}`,
        fetch: ConnectionSpace.fetch(spaces),
    });

    return {
        async *stream(subscription, signal) {
            yield* await client.store.stream(subscription, { signal });
        },
        confirm: async (subscription, watermark) => {
            await client.store.confirm({ subscription, watermark });
        },
    };
}
