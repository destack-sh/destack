import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucketHost } from "@destack/bucket/bun";
import type { Controller } from "@destack/runtime/control";
import type { BuildReader } from "@destack/package/manifest";
import { RequestId } from "@destack/runtime/request";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { Call, type Publisher, Scope, type Subject, type Uplink } from "@destack/sync";
import { reconciliation, testCallKey } from "@destack/runtime/test";
import { expect, onTestFinished } from "@destack/test";
import {
    AccessContext,
    Authentication,
    Authorization,
    Authorizer,
    Caller,
    principal,
    Role,
    universe,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, claim, machine, placement, space } from "@destack/account/object";
import { IdentityKeystore, type IdentityRegistry, identityKey } from "@destack/account/identity";
import { AccountFixture, STACKS, TestDirectory, TestIdentityRegistry } from "@destack/account/test";
import accountPackage from "@destack/account/package";
import { Provisioned } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import {
    Change,
    type DatabaseConnection,
    defineDatabase,
    type Dialect,
    type LogPosition,
    Snapshot,
    Transaction,
} from "@destack/db";
import { MemoryContentStore, TestDatabase, TestTemplate } from "@destack/db/test";
import { connectBunSqlite } from "@destack/db/bun";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { canonicalize, Identifier, type JsonObject, present, schema } from "@destack/schema";
import { Health } from "@destack/runtime/health";
import { type ClientOptions, createClient } from "@destack/runtime/client";
import { Server, type PackageImplementation } from "@destack/runtime/server";
import { until } from "@destack/runtime/timer";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/runtime/authentication";
import type { StackClient } from "../client/index.ts";
import {
    type BuildUpload,
    Database,
    Installation,
    MAIN_DATABASE,
    SPACE_PROVIDER,
} from "../object/index.ts";
import { BuildStore } from "../server/installation/build.ts";
import { StackDefinition } from "../declare/stack.ts";
import { Handoff } from "../server/host/handoff.ts";
import * as object from "../object/index.ts";
import {
    SocketRegistry,
    ConsumerController,
    serveSpace,
    type SpaceAccounts,
    type SpaceOptions,
} from "../server/index.ts";
import { spaceReplicated, stackTables } from "../stack/index.ts";
import stackPackage from "../package.ts";
import type { EventStore } from "@destack/event";
import { testEvents, serversOf } from "@destack/object/test";
import { entitlement } from "@destack/account/object";
import { TestRegistry } from "./build.ts";
import { TestRuntime } from "./runtime.ts";
import { insertBuildScope, insertRelease } from "./install.ts";
import { BunGraphStorage } from "../bun/graph.ts";
import { Clock } from "@destack/environment";

/** The main database of a space a test machine serves without other modules. */
export const spaceDatabase = defineDatabase({
    name: "main",
    tables: [...stackTables, identityKey],
    replicated: [...spaceReplicated, entitlement.table],
});

/** The fixture's space on its cloud machine, owned by the owner, with its main database. */
export const SPACE_TEMPLATE = new TestTemplate({
    name: "stack.space",
    tables: spaceDatabase,
    value: schema.null(),
    write: async ({ database }) => {
        // keep the account's scope and the space within it, owned by the owner and itself
        const now = Date.now();
        await replicateAccount(database);
        await keepSpace(database);

        // keep the replica of the cloud machine serving the residency and of the space placed in it
        await database.insert(machine.table).values({
            id: ids.cloud,
            scope: ids.platform,
            name: "cloud",
            kind: "cloud",
            residencyId: "eu",
            providerCode: "cloudflare",
            location: "eu",
            createdAt: now,
            updatedAt: now,
        });
        await placeSpace(database);

        // keep the space's main database, as its birth gives it
        await database.insert(object.database.table).values({
            id: Identifier.create("database"),
            scope: ids.space,
            name: MAIN_DATABASE,
            definitionPackageId: spaceDatabase.package.id,
            definitionVersion: spaceDatabase.package.version,
            definitionName: spaceDatabase.name,
            spec: spaceDatabase.spec,
            placement: { provider: SPACE_PROVIDER },
            createdAt: now,
            updatedAt: now,
        });

        return null;
    },
});

/** Record a test space's call events in its database, their segments in memory. */
export function openEvents(database: DatabaseConnection): Promise<EventStore> {
    return testEvents(database);
}

/** The origin a test reaches its served space at. */
const SPACE_URL = "http://space.test";

/** The name of a test machine's replica of the universe's space rows. */
const SPACE_REPLICA = "spaces";

/** The most rounds of controller writes a settle follows. */
const SETTLE_ROUNDS = 20;

/** The assurance of a person who signed in with several factors. */
const AAL2 = 2;

/** How long a test waits for the log to reach a state. */
export const WAIT_MILLISECONDS = 5000;

/** The S3 credentials of the fixture's buckets. */
const BUCKET_CREDENTIALS = {
    accessKeyId: "AKIDSTACK",
    secretAccessKey: "fixture-secret-access-key",
};

/** The identities the scenarios name. */
export const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    cloud: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000003"),
    platform: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-0000000000a0"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    stackPackage: PackageId.parse("package-01996ab0-0000-7000-8000-00000000000a"),
    notes: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000005"),
    run: schema.identifier("run").parse("run-01996ab0-0000-7000-8000-000000000006"),
    machine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000007"),
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    home: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000009"),
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000b1"),
};

/** A machine's database closed after its test. */
export class StackFixture implements AsyncDisposable {
    /** The test database. */
    readonly #test: TestDatabase;
    /** The connection the machine's database declares its tables on. */
    readonly #database: DatabaseConnection;
    /** The closing of the machine's database, absent while open. */
    #closed: Promise<void> | undefined;
    /** The machine's content store, absent until opened. */
    #store: MemoryContentStore | undefined;
    /** The machine's bucket host, opened once. */
    #buckets: Promise<LocalBucketHost> | undefined;
    /** The machine's build store, opened once. */
    #builds: Promise<BuildStore> | undefined;
    /** The index of the builds, opened once. */
    #index: Promise<BunGraphStorage> | undefined;
    /** The registry the space's releases come from. */
    readonly registry = new TestRegistry();
    /** The replica of the universe's space rows running now, one at a time. */
    #replicating: Promise<void> = Promise.resolve();

    /** Keep an opened test database. */
    private constructor(test: TestDatabase, database: DatabaseConnection = test.database) {
        this.#test = test;
        this.#database = database;
    }

    /** The machine's database. */
    get database(): DatabaseConnection {
        return this.#database;
    }

    /** Open the fixture's cloud machine keeping the fixture's space, copied from its template. */
    static async open(options: StackFixtureOptions = {}): Promise<StackFixture> {
        // copy the space's template
        const { database = spaceDatabase, dialect = "sqlite" } = options;
        const { storage } = await TestDatabase.copy(dialect, SPACE_TEMPLATE);
        if (database === spaceDatabase) {
            return StackFixture.#closing(storage);
        }

        // migrate it to the test's own tables beside the stack's, connecting with them
        await storage.database.migrate(database.tables, {
            replicated: database.replicated,
            others: "keep",
        });

        return StackFixture.#closing(storage, await storage.connect(database));
    }

    /** Open a machine keeping no space, such as the target of a move. */
    static async empty(options: StackFixtureOptions = {}): Promise<StackFixture> {
        const { database = spaceDatabase, dialect = "sqlite" } = options;

        return StackFixture.#closing(
            await TestDatabase.create(dialect, database, { isMigrated: true }),
        );
    }

    /** Keep a machine's database, closed after the test. */
    static #closing(test: TestDatabase, database?: DatabaseConnection): StackFixture {
        const fixture = new StackFixture(test, database);
        onTestFinished(() => fixture[Symbol.asyncDispose]());

        return fixture;
    }

    /** Fill the stack package's options over the machine's database, following nothing. */
    async options(
        options: FixtureOptions &
            (
                | { readonly universe?: undefined; readonly identities?: undefined }
                | { readonly universe: DatabaseConnection; readonly identities: IdentityRegistry }
            ) = {},
    ): Promise<SpaceOptions> {
        // open a universe unless the test gives one, with its identity registry
        const { universe: given, identities: givenIdentities, ...overrides } = options;
        const opened = given === undefined ? await openUniverse() : undefined;
        const universeDatabase = given ?? present(opened, "the opened universe").universe;
        const identities = givenIdentities ?? present(opened, "the opened universe").identities;
        const claims =
            overrides.claims ??
            opened?.claims ??
            (await TestDirectory.claims(universeDatabase, Clock.system));
        const sockets = overrides.sockets ?? (await this.sockets());

        return {
            principal: principal.space.reference(Scope.universe.id, ids.space),
            database: this.database,
            callKey: testCallKey,
            ...idle,
            publisher: await this.universe(universeDatabase, overrides.machine ?? ids.cloud),
            accounts: StackFixture.accounts(),
            claims,
            clock: Clock.system,
            machine: ids.cloud,
            buildRunner: ids.cloud,
            providers: [],
            main: spaceDatabase,
            sandboxes: [],
            store: options.store ?? this.store(),
            keystore: new IdentityKeystore(
                await LocalKeyring.open(new MemoryKeychain(), ids.cloud),
                identities,
                Clock.system,
            ),
            servers: [],
            registry: this.registry,
            buckets: options.buckets ?? (await this.buckets()),
            builds: options.builds ?? (await this.builds()),
            bindables: [],
            runtimes: [new TestRuntime()],
            events: overrides.events ?? (await openEvents(this.database)),
            sockets,
            isEnforcingEntitlements: false,
            publisherAt: null,
            sweepCache: null,
            repositories: null,
            follows: [],
            external: await this.index(),
            ...overrides,
        };
    }

    /** Control a space as its machine does, serving its installed revisions at each change. */
    host(store: ObjectStore, spaceId: Identifier<"space">): Controller {
        const watches = [object.installation.table, object.binding.table, object.database.table];

        return {
            name: "host",
            watches,
            keys: (change) => (watches.some((table) => Change.of(change, table)) ? [spaceId] : []),
            list: async () => [spaceId],
            reconcile: async () => {
                await this.serveInstalled(store, spaceId);

                return undefined;
            },
        };
    }

    /** Serve a space's installed revisions through its store, as its machine's start does. */
    async serveInstalled(
        store: Pick<ObjectStore, "database" | "mutate" | "principal" | "clock">,
        spaceId: Identifier<"space">,
    ): Promise<void> {
        // read the revisions main applied, as the machine's start serves them
        const builds = await this.builds();
        const installed = await Database.installed(
            Snapshot.live(this.database),
            spaceId,
            (manifest) => builds.packages.contents(manifest),
        );
        const revisions = new Map(
            installed.map((entry) => [entry.installation.id, entry.revisionId]),
        );

        // record what the space serves
        await Installation.serve(store, spaceId, revisions);
    }

    /** Call the account objects machines read, with every person at home in one space. */
    static accounts(): SpaceAccounts {
        // keep every person in the fixture's home, serving the residency on no cloud machine
        return {
            residency: { machines: async () => [] },
            user: { home: async () => ids.home },
        };
    }

    /** Stand in for the universe, replicating the spaces it changes back to the machine. */
    async universe(
        database: DatabaseConnection,
        machineId: Identifier<"machine"> = ids.cloud,
    ): Promise<Uplink> {
        // keep the spaces at their home and run the machine's placement calls over the universe's database
        const home = await StackFixture.home(database);
        const directory = await TestDirectory.uplink(database, machineId);

        return {
            ...idlePublisher,
            receive: async (mutation, signer) => {
                const isPlacement = mutation.calls.every((each) => each.type.type === "placement");

                // run a placement call at the directory
                if (isPlacement) {
                    return directory.receive(mutation);
                }
                // run a space call at its home and replicate the space back
                else {
                    const results = await home.uplink.receive(mutation, signer);
                    await this.replicate(home);

                    return results;
                }
            },
            call: (call, scope, signer) => home.uplink.call(call, scope, signer),
        };
    }

    /** Serve the spaces at their home over the universe's database, as the universe does. */
    static async home(database: DatabaseConnection): Promise<ObjectStore> {
        return new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            servers: serversOf([space]),
            database,
            claims: { object: claim },
            callKey: testCallKey,
            events: await testEvents(database),
            origin: { package: accountPackage },
        });
    }

    /** Replicate every space row a home keeps into the machine's database, up to its position. */
    replicate(home: ObjectStore): Promise<void> {
        // replicate after the replica running, failing alone
        const replicating = this.#replicating.then(() => replicateSpaces(home, this.database));
        this.#replicating = Promise.allSettled([replicating]).then(() => undefined);

        return replicating;
    }

    /** Keep the machine's content store in memory, opened once. */
    store(): MemoryContentStore {
        this.#store ??= new MemoryContentStore();

        return this.#store;
    }

    /** Keep the machine's buckets in a temporary directory until the test ends, opened once. */
    buckets(): Promise<LocalBucketHost> {
        this.#buckets ??= (async () => {
            // keep the buckets in a directory removed with the host once the test ends
            const directory = await mkdtemp(join(tmpdir(), "destack-stack-buckets-"));
            const host = new LocalBucketHost({
                directory,
                endpoint: new URL("http://s3.localhost"),
                region: "local",
                credentials: async () => BUCKET_CREDENTIALS,
            });
            onTestFinished(async () => {
                await host[Symbol.asyncDispose]();
                await rm(directory, { recursive: true, force: true });
            });

            return host;
        })();

        return this.#buckets;
    }

    /** Keep the machine's builds in a bucket of the fixture's host, opened once. */
    builds(): Promise<BuildStore> {
        this.#builds ??= (async () =>
            BuildStore.of(await this.buckets(), {
                scope: ids.cloud,
                bucketId: Identifier.create("bucket"),
            }))();

        return this.#builds;
    }

    /** Index the space's builds in a temporary directory until the test ends. */
    index(): Promise<BunGraphStorage> {
        this.#index ??= (async () => {
            // keep the index files in a directory removed with the index once the test ends
            const directory = await mkdtemp(join(tmpdir(), "destack-stack-index-"));
            const builds = await this.builds();
            const index = new BunGraphStorage(
                this.database,
                async (_space, manifest) => builds.packages.contents(manifest),
                directory,
            );
            onTestFinished(async () => {
                await index[Symbol.asyncDispose]();
                await rm(directory, { recursive: true, force: true });
            });

            return index;
        })();

        return this.#index;
    }

    /** Store a running build's files as its runner. */
    async storeBuild(
        runner: StackClient,
        target: { readonly spaceId: Identifier<"space">; readonly id: Identifier<"build"> },
        reader: BuildReader,
    ): Promise<readonly BuildUpload[]> {
        // report the files, then keep them as the runner's uploads do
        const files = [...reader.references(), ...(await reader.files())].map(
            ({ digest, size, mediaType }) => ({ digest, size, mediaType }),
        );
        const { uploads } = await runner.build.progress({
            ...target,
            requestId: RequestId.create(Date.now()),
            progress: { phase: "storing", files },
        });
        await (await this.builds()).packages.put(reader);

        // finish with the manifest
        await runner.build.finish({
            ...target,
            requestId: RequestId.create(Date.now()),
            end: { kind: "stored", manifest: reader.manifest },
        });

        return uploads;
    }

    /** Keep a build in the build store and record it succeeded now. */
    async recordBuild(
        reader: BuildReader,
    ): Promise<{ readonly kind: "build"; readonly build: Identifier<"build"> }> {
        // keep the build in the build store
        const { database } = this;
        const manifest = await (await this.builds()).packages.put(reader);

        // record the build succeeded now
        const id = Identifier.create("build");
        const now = Date.now();
        await database.insert(object.build.table).values({
            id,
            scope: ids.space,
            source: {
                kind: "checkout",
                machine: ids.machine,
                checkout: ids.checkout,
                directory: ".",
            },
            outputs: ["bun"],
            state: "succeeded",
            manifest,
            startedAt: now,
            finishedAt: now,
            createdAt: now,
            updatedAt: now,
        });

        // keep the build's own scope inside its space's, as its creation does
        await insertBuildScope(database, ids.space, id);

        return { kind: "build", build: id };
    }

    /** Pin each declared installation to its release's manifest in the registry. */
    async pin(definition: JsonObject): Promise<JsonObject> {
        const { installations } = StackDefinition.collections(definition);
        const pinned = await Promise.all(
            Object.entries(installations).map(async ([key, installed]) => {
                // find the manifest of the installed release
                const { id, version } = installed.package;
                const manifest = await this.registry.find(id, version);

                return [
                    key,
                    { ...installed, package: { ...installed.package, manifest } },
                ] as const;
            }),
        );

        return { ...definition, installations: Object.fromEntries(pinned) };
    }

    /** Keep a package's fixture release in the build store, as the space's build of it. */
    async release(
        packageId: PackageId,
        version: string,
    ): Promise<{ readonly id: Identifier<"build">; readonly manifest: string }> {
        // replicate the release the registry keeps into the build store, and keep it as the space's build
        const manifest = await this.registry.find(packageId, version);
        const reader = await (await this.builds()).open(this.registry, packageId, manifest);
        const release = { package: packageId, version, manifest };

        return insertRelease(
            this.database,
            ids.space,
            release,
            Object.keys(reader.manifest.outputs),
        );
    }

    /** Keep the machine's live sockets in memory until the test ends. */
    async sockets(): Promise<SocketRegistry> {
        // collect the store's failures
        const reported: unknown[] = [];
        const sockets = await SocketRegistry.open({
            objects: [],
            clock: Clock.system,
            channel: this.database.channel("sockets"),
            memory: (tables) => connectBunSqlite(":memory:", tables),
            report: (error) => reported.push(error),
        });

        // close the store and require no failure once the test ends
        onTestFinished(async () => {
            await sockets[Symbol.asyncDispose]();
            expect(reported).toEqual([]);
        });

        return sockets;
    }

    /** Serve the machine to each request's user, machine or installation; return a client each. */
    async serve(
        options: FixtureOptions & {
            /** Whether the store runs its controllers, true by default. */
            readonly isControlled?: boolean;
        } = {},
    ): Promise<
        (caller: string, connection?: { readonly isAuthenticatedNow?: boolean }) => StackClient
    > {
        const { server } = await this.server(options);

        return (caller, connection = {}) =>
            createClient(stackPackage, StackFixture.endpoint(server, caller, connection));
    }

    /** Serve the machine with its store to each request's user, machine or installation. */
    async server(
        options: FixtureOptions & {
            /** Whether the store runs its controllers, true by default. */
            readonly isControlled?: boolean;
        } = {},
    ): Promise<{ readonly server: Server; readonly store: ObjectStore }> {
        // serve the space's objects and authenticate each request as the caller its header names
        const { isControlled = true, ...given } = options;
        const served = await this.options(given);

        // serve the stack package at once, as the fixture writes the replicas itself and no snapshot ever arrives
        const { store, packages } = serveSpace(served);
        const stack = present(
            packages.find((implementation) => implementation.package.id === stackPackage.id),
            "the stack package",
        );
        const server = StackFixture.#start(stack, isControlled);

        return { server, store };
    }

    /** Start a served package, authenticating the callers headers name after the package's own. */
    static #start(implementation: PackageImplementation, isControlled: boolean): Server {
        // serve without waiting for readiness, running controllers when asked
        const { ready: _ready, ...served } = implementation;
        const server = Server.start({
            clock: Clock.system,
            ...served,
            ...(isControlled ? {} : { controllers: [] }),
            resources: new ResourceContext(),
            health: new Health("space"),
            authenticate: async (request) => {
                // act as the caller the header names
                const isAuthenticatedNow = request.headers.has("x-authenticated-now");

                return subjectAuthentication(namedSubject(request), isAuthenticatedNow);
            },
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });
        onTestFinished(() => server.close());

        return server;
    }

    /** Call a served machine as the user, machine or installation an identifier refers to. */
    static endpoint(
        server: Pick<Server, "fetch">,
        caller: string,
        connection: { readonly isAuthenticatedNow?: boolean } = {},
    ): ClientOptions {
        return {
            url: SPACE_URL,
            headers: {
                [callerHeader(caller)]: caller,
                ...(connection.isAuthenticatedNow === true ? { "x-authenticated-now": "1" } : {}),
            },
            fetch: (request) => server.fetch(request),
        };
    }

    /** Connect to the handoff a server serves, as a caller. */
    static handoff(server: Pick<Server, "fetch">, caller: string): Handoff {
        const endpoint = StackFixture.endpoint(server, caller);

        return Handoff.connect(SPACE_URL, async (request) => {
            // send the request with the caller's headers
            const sent = new Request(request);
            for (const [name, value] of Object.entries(endpoint.headers ?? {})) {
                sent.headers.set(name, String(value));
            }

            return server.fetch(sent);
        });
    }

    /** Close the machine's database once. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.#closed ??= this.#test.close();
        await this.#closed;
    }
}

/** How a fixture opens a machine's database. */
export interface StackFixtureOptions {
    /** The machine's database: the stack package's, or one with other packages' tables too. */
    readonly database?: ReturnType<typeof defineDatabase>;
    /** The database's dialect, SQLite by default. */
    readonly dialect?: Dialect;
}

/** Read the subject a request's header sends: the universe, a machine, an installation, a user. */
function namedSubject(request: Request): Subject {
    // read the universe, machine and installation headers
    const isUniverse = request.headers.has("x-universe");
    const machineId = request.headers.get("x-machine");
    const installationId = request.headers.get("x-installation");

    // take the universe
    if (isUniverse) {
        return universe.reference(Scope.universe.id, Scope.universe.id);
    }
    // take the machine
    else if (machineId !== null) {
        return principal.machine.reference(Scope.universe.id, machineId);
    }
    // take the installation
    else if (installationId !== null) {
        return principal.installation.reference(ids.space, installationId);
    }
    // take the user otherwise
    else {
        return principal.user.reference(
            "universe",
            present(request.headers.get("x-user"), "the x-user header"),
        );
    }
}

/** Name the header a caller is sent under by the kind its identifier names. */
function callerHeader(caller: string): string {
    // send the universe
    if (caller === Scope.universe.id) {
        return "x-universe";
    }
    // send a machine
    else if (caller.startsWith("machine-")) {
        return "x-machine";
    }
    // send an installation
    else if (caller.startsWith("installation-")) {
        return "x-installation";
    }
    // send a user otherwise
    else {
        return "x-user";
    }
}

/** Keep the account's scope and row, as its home and the machines replicating its chain do. */
export async function replicateAccount(database: DatabaseConnection): Promise<void> {
    const now = Date.now();
    await new AccessFixture(database).replicateScope(account.reference("universe", ids.account));
    await database.insert(account.table).values({
        id: ids.account,
        scope: "universe",
        handle: "acme",
        name: "Acme",
        kind: "shared",
        residencyId: "eu",
        createdAt: now,
        updatedAt: now,
    });
}

/** Keep a space of the fixture's account owned by the owner and by itself, as its creation does. */
export async function keepSpace(
    database: DatabaseConnection,
    spaceId: Identifier<"space"> = ids.space,
    name = "personal",
): Promise<void> {
    // keep the space's row
    const now = Date.now();
    await database.insert(space.table).values({
        id: spaceId,
        scope: ids.account,
        name,
        stack: STACKS.space,
        createdAt: now,
        updatedAt: now,
    });

    // make the owner and the space itself the space's owners, as the space's creation binds them
    const owner = principal.user.reference("universe", ids.owner);
    const created = space.reference(ids.account, spaceId);
    await Authorization.run(
        new Authorizer([space.policy], [space.mapping]),
        database,
        () => AccessContext.of(Caller.of(owner), now),
        (authorization) => authorization.create(created, { owner }),
    );
    await Transaction.run(database, (transaction) =>
        Role.bindOwner(
            transaction,
            created,
            principal.space.reference(Scope.universe.id, spaceId),
            0,
        ),
    );
}

/** Replicate every space row a home keeps into a machine's database, up to its position now. */
async function replicateSpaces(home: ObjectStore, database: DatabaseConnection): Promise<void> {
    // replicate the universe's space rows under a replica of the machine's own, which no request drops
    const subscription = Authorizer.universeSubscription({
        name: SPACE_REPLICA,
        scope: Scope.universe.id,
        below: ids.cloud,
        parameters: {
            rows: [{ type: space.typeReference, where: {}, isEverywhere: true }],
        },
    });
    const replica = home.source.buildReplica(subscription);
    const options = { clock: Clock.system };
    await replica.register(database, options);

    // leave a replica reflecting the home's position already
    const target = await home.database.log.position();
    const replicated = await replica.position(database);
    if (replicated?.epoch === target.epoch && replicated.sequence >= target.sequence) {
        return;
    }

    // apply the home's pages until one reaches the position
    const stopped = new AbortController();
    const pages = home.uplink.stream(
        { ...subscription, ...(await replica.resume(database)) },
        stopped.signal,
    );
    for await (const page of replica.apply(database, pages, options)) {
        // stop once a complete page reaches it
        if (page.complete && page.position.sequence >= target.sequence) {
            stopped.abort();
            break;
        }
    }
}

/** Keep the replica of a space placing it in a machine at the first epoch. */
export async function placeSpace(
    database: DatabaseConnection,
    spaceId: Identifier<"space"> = ids.space,
    machineId: Identifier<"machine"> = ids.cloud,
): Promise<void> {
    const now = Date.now();
    await database
        .insert(placement.table)
        .values({
            id: spaceId,
            scope: ids.account,
            machineId,
            epoch: 1,
            createdAt: now,
            updatedAt: now,
        })
        .onConflictDoUpdate({ target: placement.table.id, set: { machineId } });
}

/** Reconcile one key of a served object type's controller once. */
export async function reconcile(
    store: Pick<ObjectStore, "controllers" | "database">,
    name: string,
    key: Readonly<Record<string, unknown>>,
): Promise<number | undefined> {
    // find the controller
    const controller = controllerOf(store, name);

    // see the key changed at each commit after the reconciliation waits
    const { log } = store.database;
    const settled = new AbortController();
    const changed = async () => {
        const { sequence } = await log.position();
        await log.until(async () => (await log.position()).sequence > sequence, settled.signal);
    };
    const signal = new AbortController().signal;
    try {
        return await controller.reconcile(canonicalize(key), { signal, changed });
    } finally {
        settled.abort();
    }
}

/** Reconcile every resource of a space as the control loops do once it changes. */
export async function reconcileResources(
    store: Pick<
        ObjectStore,
        "controllers" | "database" | "objects" | "serving" | "mutate" | "clock"
    >,
    spaceId: string,
): Promise<void> {
    // reconcile each served kind's resources in the space
    const kinds = store.objects.filter((type) => Provisioned.is(type));
    const consumers = new ConsumerController(store, kinds, spaceDatabase);
    for (const kind of kinds) {
        const rows = await store.database
            .select()
            .from(kind.table)
            .where(kind.inScope(schema.identifier("space").parse(spaceId)));
        for (const row of rows) {
            const id = schema.string().parse(row["id"]);
            await consumers.reconcile(id);
            await reconcile(store, kind.name, { id });
            await consumers.reconcile(id);
        }
    }
}

/** Reconcile each key the changes since a log position select, round by round. */
export async function settle(
    database: DatabaseConnection,
    controllers: readonly Controller[],
    after: LogPosition,
): Promise<void> {
    // follow the tables any of the controllers watch from the position
    const tables = [...new Set(controllers.flatMap((controller) => controller.watches ?? []))];
    let sequence = after.sequence;
    for (let round = 0; round < SETTLE_ROUNDS; round += 1) {
        // read the changes the controllers watch since the last round
        const page = await database.log.read({ tables, after: sequence });
        if (page.changes.length === 0) {
            return;
        }
        sequence = page.sequence;

        // reconcile each key a watched change selects, once per controller
        for (const controller of controllers) {
            const keys = new Set<string>();
            const watched = page.changes.filter((change) =>
                (controller.watches ?? []).includes(change.table),
            );
            for (const change of watched) {
                for (const key of (await controller.keys?.(change)) ?? []) {
                    keys.add(key);
                }
            }
            for (const key of keys) {
                await controller.reconcile(key, reconciliation(new AbortController().signal));
            }
        }
    }

    // refuse controllers that keep writing
    throw new TypeError(`the controllers kept writing after ${SETTLE_ROUNDS} rounds`);
}

/** Find a served object type's controller by name, refusing one the fixture does not serve. */
function controllerOf(store: Pick<ObjectStore, "controllers">, name: string): Controller {
    const controller = store.controllers().find((each) => each.name === name);
    if (controller === undefined) {
        throw new TypeError(`the fixture serves no controller ${name}`);
    }

    return controller;
}

/** A publisher streaming nothing until each stream's signal aborts. */
export const idlePublisher: Publisher = { stream: (_subscription, signal) => stream(signal) };

/** The options a test gives the fixture's space. */
export type FixtureOptions = Partial<SpaceOptions>;

/** Replicate nothing of the universe, and call no peer machine. */
const idle: Pick<SpaceOptions, "publisher" | "handoff"> = {
    publisher: {
        ...idlePublisher,
        receive: async (mutation) => {
            throw new TypeError(
                `the fixture's universe receives no ${mutation.calls.map((entry) => Call.label(entry)).join(", ")}`,
            );
        },
        call: async (call) => {
            throw new TypeError(`the fixture's universe answers no ${Call.label(call)}`);
        },
    },
    handoff: async (source) => {
        throw new TypeError(`the fixture knows no handoff of ${source}`);
    },
};

/** Yield nothing until a signal aborts. */
async function* stream(signal: AbortSignal): AsyncGenerator<never> {
    await until(signal);
    yield* [];
}

/** Open the universe's database in a database of its own, with its claims. */
export async function openUniverse(
    overrides: Partial<Pick<SpaceOptions, "publisher" | "handoff">> = {},
): Promise<
    Pick<SpaceOptions, "handoff" | "claims"> &
        Partial<Pick<SpaceOptions, "publisher">> & {
            readonly universe: DatabaseConnection;
            readonly identities: IdentityRegistry;
        }
> {
    // keep the universe's tables in a database of their own, with the fixture's account and space at home in it
    const storage = await AccountFixture.storage(undefined, "sqlite");
    onTestFinished(() => storage.close());
    const { database } = storage;
    const owner = principal.user.reference(Scope.universe.id, ids.owner);
    await Authorization.run(
        new Authorizer([account.policy], [account.mapping]),
        database,
        () => AccessContext.of(Caller.of(owner), Date.now()),
        (authorization) =>
            authorization.create(account.reference(Scope.universe.id, ids.account), { owner }),
    );
    await keepSpace(database);

    // leave the publisher to the fixture's stand-in universe unless the test names one
    const { publisher: _publisher, ...rest } = idle;

    return {
        ...rest,
        ...overrides,
        universe: database,
        identities: (await TestIdentityRegistry.open(database, Clock.system, ids.cloud)).registry,
        claims: await TestDirectory.claims(database, Clock.system),
    };
}

/** Authenticate a subject, at the highest assurance when it authenticated just now. */
export function subjectAuthentication(
    subject: Subject,
    isAuthenticatedNow = false,
): Authentication {
    const now = Date.now();

    return {
        caller: Caller.of(subject),
        credential: {
            kind: "fixture",
            id: "fixture-1",
            audience: stackPackage.id,
            verifiedAt: now,
            expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            ...(isAuthenticatedNow ? { assurance: { level: AAL2, authenticatedAt: now } } : {}),
        },
    };
}
