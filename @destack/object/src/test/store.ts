import { testSimulation } from "@destack/test/world";
import { MemoryBlobStorage } from "@destack/resource";
import { type DatabaseConnection, defineDatabase, type Table, Transaction } from "@destack/db";
import {
    AccessContext,
    anyone,
    type Authentication,
    Authorization,
    Caller,
    principal,
    universe,
} from "@destack/access";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { TestAccess } from "@destack/access/test";
import { ServerContext } from "../workload/workload.ts";
import { connectBunSqlite } from "@destack/db/bun";
import { TestDatabase } from "@destack/db/test";
import { Clock, type Environment } from "@destack/environment";
import { Simulation, SimulatedScheduler } from "@destack/environment/simulation";
import { Package, type PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, present, schema } from "@destack/schema";
import {
    type ObjectReference,
    ObjectTypeReference,
    type Publisher,
    Scope,
    Subject,
} from "@destack/sync";
import { ObjectType } from "../object/object.ts";
import type { PackageHandle } from "../declare/package.ts";
import { Health } from "../server/health.ts";
import { type PackageHandler, Server } from "../server/http.ts";
import {
    implementPackage,
    type PackageImplementation,
    type ObjectPackage,
} from "../server/implementation.ts";
import { EphemeralStorage } from "../server/ephemeral.ts";
import { ObjectStore, type ObjectStoreOptions } from "../server/store.ts";
import { Installation } from "../installation/installation.ts";
import { tablesOf } from "../stack/db.ts";
import objectPackage from "../package.ts";
import { TestBearer } from "./bearer.ts";
import { TestCaller, type TestServed } from "./caller.ts";
import { testCallKey } from "./context.ts";
import { testEvents } from "./events.ts";

/** How long a closing test server drains its requests, in milliseconds: far above a test call's. */
const DRAIN_MILLISECONDS = 1000;

/** The node the test store runs as in its world unless its test names another. */
const WORLD_NODE = "store";

/** Authenticate a request to a served package, absent for no one. */
export type TestAuthenticate = (
    request: Request,
    audience: PackageId,
) => Promise<Authentication | undefined>;

/** A test caller of a served package: who calls, which package, and whether just stepped up. */
export interface TestSigner {
    /** The calling subject. */
    readonly subject: Subject;
    /** The package called. */
    readonly audience: PackageId;
    /** Whether the subject authenticated with a second factor just before the call. */
    readonly isSteppedUp: boolean;
}

/** How a test host's callers prove who they are: the requests subjects send, and their reading. */
export interface TestCredential {
    /** Sign a request as a caller of a package. */
    readonly sign: (request: Request, signer: TestSigner) => Promise<Request>;
    /** Authenticate a request to a served package, absent for no one. */
    readonly authenticate: TestAuthenticate;
}

/** A test store's options: a store's, its principal given and its blobs in memory. */
export type TestStoreOptions = (ObjectStoreOptions extends infer Options
    ? Options extends unknown
        ? Omit<Options, "blobs">
        : never
    : never) & { readonly principal: Subject };

/** What a test store serves: its packages, its host's tables, its world and its space. */
export interface TestObjectStoreOptions {
    /** The packages it serves, as declared or as implemented. */
    readonly packages: readonly (ObjectPackage | PackageImplementation)[];
    /** The tables its host stores beside them. */
    readonly tables?: readonly Table[];
    /** The simulated world it runs in and drives, its own paused at a fixed start by default. */
    readonly simulation?: Simulation;
    /** The node it runs as in its world, `store` by default. */
    readonly node?: string;
    /** The scope it serves, a new space by default. */
    readonly scope?: ObjectReference;
    /** The further scopes it serves at the universe, each its own owner, such as people's. */
    readonly scopes?: readonly ObjectReference[];
    /** Whether anyone may read the scope, public by default. */
    readonly visibility?: "public" | "private";
    /** The subjects owning the scope beside itself, as its creation binds them. */
    readonly owners?: readonly Subject[];
    /** The groups each user's credential vouches for, by user. */
    readonly groups?: ReadonlyMap<string, readonly Subject[]>;
    /** Open the publisher toward a followed scope's home, none by default. */
    readonly publisher?: (scope: string) => Publisher;
}

/** An object store serving packages in one space over a test database, called as its users. */
export class TestObjectStore implements TestServed, AsyncDisposable {
    /** The store. */
    readonly store: ObjectStore;
    /** The scope it serves. */
    readonly scope: string;
    /** The server of each package. */
    readonly #servers: ReadonlyMap<PackageId, Server>;
    /** How its callers prove who they are. */
    readonly #credential: TestCredential;
    /** What disposal closes: the servers, and the databases the test store opened. */
    readonly #disposables: AsyncDisposableStack;
    /** The object types the store serves by their reference's key, each with its package. */
    readonly #types: ReadonlyMap<
        string,
        { readonly type: ObjectType; readonly handle: PackageHandle }
    >;

    /** The world it runs in, its store's environment. */
    get environment(): Environment {
        return this.store.environment;
    }

    /** The simulation running its world. */
    get simulation(): Simulation {
        return present(SimulatedScheduler.of(this.environment).world, "the store's world");
    }

    /** Create a test store over its database and servers. */
    private constructor(
        store: ObjectStore,
        servers: ReadonlyMap<PackageId, Server>,
        credential: TestCredential,
        disposables: AsyncDisposableStack,
    ) {
        // store the store, its scope, its world, its servers, its callers' credential and what disposal closes
        this.store = store;
        this.scope = store.scope;
        this.#servers = servers;
        this.#credential = credential;
        this.#disposables = disposables;

        // index the served object types by their reference, each with its package
        const served = store.packages.flatMap((handler) =>
            Object.values(handler.package.objects)
                .filter((declared): declared is ObjectType => declared instanceof ObjectType)
                .map((type) => ({ type, handle: handler.package })),
        );
        this.#types = new Map(
            served.map((each) => [ObjectTypeReference.key(each.type.typeReference), each]),
        );
    }

    /** Serve packages at their base installations in a scope, its blobs in memory. */
    static async at(scope: ObjectReference, options: TestStoreOptions): Promise<ObjectStore> {
        // serve each package at its base installation in the scope
        const { environment } = options;
        const servers = await Promise.all(
            options.servers.map(async (server) =>
                server.installation === undefined
                    ? server.at(await Installation.base(scope, server.package, environment.crypto))
                    : server,
            ),
        );

        return new ObjectStore({
            ...options,
            blobs: new MemoryBlobStorage(environment.crypto),
            servers,
        });
    }

    /** Open a store of packages in a scope born with their base installations. */
    static async receive(scope: ObjectReference, options: TestStoreOptions): Promise<ObjectStore> {
        // serve the packages at their base installations, homing a partition scope's partition
        const types = options.servers.flatMap((server) => Object.values(server.package.objects));
        const policies = [
            principal.space,
            ...(options.policies ?? []).map((each) => ("policy" in each ? each.policy : each)),
            ...types.flatMap((type) => [type.policy, ...type.scopes.map((each) => each.policy)]),
        ];
        const isPartition =
            scope.id !== Scope.universe.id &&
            policies.some((policy) => policy.is(scope) && policy.definition.partition === true);
        const store = await TestObjectStore.at(scope, {
            ...(isPartition ? { partitions: [scope.id] } : {}),
            ...options,
        });

        // install the base installations of the packages served at one
        await TestObjectStore.install(store, scope, options.servers);

        return store;
    }

    /** Install the base installations a store serves, each bound to its role. */
    static async install(
        store: ObjectStore,
        scope: ObjectReference,
        servers: readonly PackageImplementation[],
    ): Promise<void> {
        // receive the base-served packages, each installation bound to its role
        const { environment } = store;
        const ancestors = scope.scope === Scope.universe.id ? [] : [scope.scope];
        const bases = servers.filter(
            (server) =>
                server.installation === undefined ||
                !ServerContext.isInstalled(server.installation),
        );
        const packages = [...new Set(bases.map((server) => server.package))];
        await Transaction.run(store.database, async (transaction) => {
            const authorization = new Authorization(
                store.authorizer,
                transaction,
                () => AccessContext.of(Caller.of(store.principal), environment.clock.now()),
                environment,
            );
            await Installation.receive(authorization, scope, ancestors, packages);
        });
    }

    /** Serve packages in a scope, a new space by default, over a database derived from them. */
    static async open(options: TestObjectStoreOptions): Promise<TestObjectStore> {
        // take the test's world, else open one paused at a fixed start
        const simulation = options.simulation ?? testSimulation();
        const disposables = new AsyncDisposableStack();
        const environment = simulation.environment(options.node ?? WORLD_NODE);
        try {
            // open the store in its world
            return await TestObjectStore.#open(options, environment, disposables);
        } catch (error) {
            await disposables.disposeAsync();
            throw error;
        }
    }

    /** Serve packages in a world, closing with a lifetime. */
    static async #open(
        options: TestObjectStoreOptions,
        environment: Environment,
        disposables: AsyncDisposableStack,
    ): Promise<TestObjectStore> {
        // serve each package as implemented, or as declared without code
        const implementations = options.packages.map((each) =>
            "definition" in each ? each : implementPackage(each, {}),
        );

        // derive and migrate the database from the core and served packages and their follows, in the store's world
        const follows = implementations.flatMap((each) => each.definition(undefined).follows ?? []);
        const storage = await TestDatabase.create(
            defineDatabase({
                name: "main",
                ...tablesOf({
                    packages: [objectPackage, ...implementations.map((each) => each.package)],
                    follows,
                    tables: options.tables ?? [],
                }),
            }),
            { isMigrated: true, environment },
        );

        // take the store's scope
        const scope =
            options.scope ??
            principal.space.reference(Scope.universe.id, Identifier.create("space", environment));

        // store the served ephemeral types in memory, as a host does, failing the test on a lost write
        const ephemeral = await EphemeralStorage.open(
            implementations.flatMap((each) => Object.values(each.package.objects)),
            storage.database.tracker,
            (tables) => connectBunSqlite(":memory:", tables, { environment }),
            {
                report: (error) => SimulatedScheduler.of(environment).fail(error),
            },
        );

        // serve the packages through one store as the scope, or as the universe a user's scope lives at, the scope born with their bases
        const store = await TestObjectStore.receive(scope, {
            servers: implementations,
            chain: async () => undefined,
            origin: { package: Package.of(objectPackage) },
            principal: principal.user.is(scope)
                ? universe.reference(Scope.universe.id, Scope.universe.id)
                : Subject.parse(scope),
            callKey: testCallKey,
            events: await testEvents(storage.database, environment),
            database: storage.database,
            ...(ephemeral === undefined ? {} : { ephemeral }),
            ...(options.publisher === undefined ? {} : { publisher: options.publisher }),
            environment,
        });

        // open the store's born scope to its owners, and the further scopes as their own owners
        await TestObjectStore.#openScope(
            store,
            scope,
            options.visibility ?? "public",
            options.owners ?? [],
            "born",
        );
        for (const further of options.scopes ?? []) {
            await TestObjectStore.#openScope(store, further, "private", [], "replicated");
        }

        // serve it, closing the ephemeral and durable databases after the servers
        disposables.defer(() => storage.close());
        if (ephemeral !== undefined) {
            disposables.use(ephemeral);
        }

        return TestObjectStore.#serve(
            store,
            store.packages,
            TestObjectStore.#bearers(store, options.groups ?? new Map()),
            { disposables },
        );
    }

    /** Serve an existing store's packages over HTTP to bearer users, leaving its databases open. */
    static serve(
        store: ObjectStore,
        options: {
            /** The groups each user's credential vouches for, by user. */
            readonly groups?: ReadonlyMap<string, readonly Subject[]>;
            /** The handlers to serve, such as a host's with verifiers, the store's by default. */
            readonly packages?: readonly PackageHandler[];
            /** How callers prove who they are, as a host checks it, test bearers by default. */
            readonly credential?: TestCredential;
            /** The scopes the servers serve, its own first, the store's alone by default. */
            readonly scopes?: readonly string[];
            /** Take what the servers report, failing the world by default. */
            readonly report?: (error: unknown) => void;
        } = {},
    ): TestObjectStore {
        const groups = options.groups ?? new Map();

        return TestObjectStore.#serve(
            store,
            options.packages ?? store.packages,
            options.credential ?? TestObjectStore.#bearers(store, groups),
            {
                disposables: new AsyncDisposableStack(),
                ...(options.scopes === undefined ? {} : { scopes: options.scopes }),
                ...(options.report === undefined ? {} : { report: options.report }),
            },
        );
    }

    /** Serve a store's packages to the users its requests name, running its controllers. */
    static #serve(
        store: ObjectStore,
        packages: readonly PackageHandler[],
        credential: TestCredential,
        run: {
            readonly disposables: AsyncDisposableStack;
            readonly scopes?: readonly string[];
            readonly report?: (error: unknown) => void;
        },
    ): TestObjectStore {
        // start one server per package, closed first on disposal, failing the world on what they report
        const { disposables } = run;
        const report =
            run.report ?? ((error) => SimulatedScheduler.of(store.environment).fail(error));
        const scopes =
            run.scopes ?? (store.scope === Scope.universe.id ? undefined : [store.scope]);
        const servers = new Map(
            packages.map((handler) => [
                handler.package.id,
                Server.start({
                    environment: store.environment,
                    ...handler,
                    // serve the store's scopes, or every scope as the universe's store does
                    ...(scopes === undefined ? {} : { scopes }),
                    resources: new ResourceContext(),
                    health: new Health(handler.package.name),
                    drainTimeout: DRAIN_MILLISECONDS,
                    report,
                    // add no host requirement, as a host serving no machines or installations
                    authorizeMachine: async () => {},
                    authenticate: (request) => credential.authenticate(request, handler.package.id),
                }),
            ]),
        );
        for (const server of servers.values()) {
            disposables.defer(() => server.close());
        }

        return new TestObjectStore(store, servers, credential, disposables);
    }

    /** Sign callers with test bearers, authenticated toward a store's packages in their groups. */
    static #bearers(
        store: ObjectStore,
        groups: ReadonlyMap<string, readonly Subject[]>,
    ): TestCredential {
        return {
            sign: async (request, { subject, isSteppedUp }) => {
                const headers = new Headers(request.headers);
                for (const [name, value] of Object.entries(
                    TestBearer.headers(subject, isSteppedUp),
                )) {
                    headers.set(name, value);
                }

                return new Request(request, { headers });
            },
            authenticate: async (request, audience) =>
                TestObjectStore.#authenticate(request, {
                    audience,
                    scope: store.scope,
                    groups,
                    now: store.clock.now(),
                }),
        };
    }

    /** Sign a request as a caller of a package with the callers' credential. */
    sign(request: Request, signer: TestSigner): Promise<Request> {
        return this.#credential.sign(request, signer);
    }

    /** The store's database. */
    get database(): DatabaseConnection {
        return this.store.database;
    }

    /** Call the store as a subject, or by an identifier whose type names its principal kind. */
    as(caller: string | Subject): TestCaller {
        const subject = typeof caller === "string" ? this.#subject(caller) : caller;

        return new TestCaller(this, subject);
    }

    /** Read an identifier's principal by its kind, such as a machine, space or user. */
    #subject(id: string): Subject {
        // take the universe
        if (id === Scope.universe.id) {
            return universe.reference(Scope.universe.id, Scope.universe.id);
        }

        // take the principal type whose identifiers it is: a machine, space or user at the universe, an installation in the served scope
        const kind = [
            principal.machine,
            principal.space,
            principal.installation,
            principal.user,
        ].find((policy) => schema.identifier(policy.name).safeParse(id).success);
        if (kind !== undefined) {
            return kind.reference(
                kind === principal.installation ? this.scope : Scope.universe.id,
                id,
            );
        }

        // refuse an identifier of no principal kind
        throw new TypeError(`${id} names no principal kind: pass its subject`);
    }

    /** The clock the store runs at. */
    get clock(): Clock {
        return this.store.clock;
    }

    /** Read the server of a package the store serves. */
    server(handle: PackageHandle): Server {
        // refuse a package the store serves no server of
        const server = this.#servers.get(handle.id);
        if (server === undefined) {
            throw new TypeError(`the test store serves no package ${handle.name}`);
        }

        return server;
    }

    /** Read a served object type with its package, absent for a type served nowhere. */
    type(
        reference: ObjectTypeReference,
    ): { readonly type: ObjectType; readonly handle: PackageHandle } | undefined {
        return this.#types.get(ObjectTypeReference.key(reference));
    }

    /** Read the one served object type of a name with its package, refusing a shared name. */
    named(name: string): { readonly type: ObjectType; readonly handle: PackageHandle } | undefined {
        const found = [...this.#types.values()].filter((each) => each.type.name === name);
        if (found.length > 1) {
            throw new TypeError(`two packages the test store serves declare ${name}`);
        }

        return found[0];
    }

    /** Wait until every server serves, its readiness met, failing on the first reported failure. */
    async ready(): Promise<void> {
        const servers = [...this.#servers.values()];
        const ready = Promise.all(servers.map((server) => server.ready()));
        await Promise.race([ready, SimulatedScheduler.of(this.environment).failed]);
    }

    /** Stop every server's controllers at once, while it serves its requests until it drains. */
    async stopControllers(): Promise<void> {
        await Promise.all([...this.#servers.values()].map((server) => server.stopControllers()));
    }

    /** Run a cleanup once the store closes, before its servers and databases close. */
    defer(cleanup: () => Promise<void>): void {
        this.#disposables.defer(cleanup);
    }

    /** Close the servers, then the databases the test store opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
    }

    /** Authenticate a bearer's subject: a user in the scope with their groups, else by its kind. */
    static #authenticate(
        request: Request,
        served: {
            readonly audience: PackageId;
            readonly scope: string;
            readonly groups: ReadonlyMap<string, readonly Subject[]>;
            readonly now: number;
        },
    ): Authentication | undefined {
        // serve a request without a bearer as no one
        if (request.headers.get("authorization") === null) {
            return undefined;
        }

        // sign in a subject other than a user by its kind's credential, at its assurance
        const subject = TestBearer.subject(request);
        const assurance = TestBearer.assurance(request, served.now);
        if (!principal.user.is(subject)) {
            return TestBearer.authenticate(subject, served.audience, served.now, assurance);
        }

        // sign a user in, bound to a space store's scope and unbound at the universe, in their groups
        return {
            caller: { ...Caller.of(subject), groups: served.groups.get(subject.id) ?? [] },
            credential: {
                kind: "session",
                id: subject.id,
                audience: served.audience,
                ...(served.scope === Scope.universe.id ? {} : { scope: served.scope }),
                verifiedAt: served.now,
                expiresAt: served.now + TOKEN_LIFETIME_MILLISECONDS,
                ...(assurance === undefined ? {} : { assurance }),
            },
        };
    }

    /** Open a scope to its owners, replicating it as its universe would unless born here. */
    static async #openScope(
        store: ObjectStore,
        object: ObjectReference,
        visibility: "public" | "private",
        owners: readonly Subject[],
        origin: "born" | "replicated",
    ): Promise<void> {
        // record a replicated scope in its own partition or its parent's, as its policy declares, and bind its owners
        const fixture = new TestAccess(store.database, store.environment);
        if (origin === "replicated") {
            await fixture.replicateScope(object, store.authorizer.policy(object));
        }
        for (const owner of [Subject.parse(object), ...owners]) {
            await fixture.replicateOwner(object, owner, 0);
        }

        // relate anyone to a public space as its reader
        if (visibility === "public") {
            await fixture.replicateRelationship(object, "reader", anyone.reference("*", "*"), 0);
        }
    }
}
