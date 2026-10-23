import { anyone, type Authentication, Caller, principal, universe } from "@destack/access";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { AccessFixture } from "@destack/access/test";
import { type DatabaseConnection, defineDatabase, TABLE, type Table } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { TestDatabase } from "@destack/db/test";
import { Clock } from "@destack/environment";
import { Package, type PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, schema } from "@destack/schema";
import { type ObjectReference, Scope, Subject } from "@destack/sync";
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
import { ObjectStore } from "../server/store.ts";
import { tablesOf } from "../stack/db.ts";
import objectPackage from "../package.ts";
import { TestBearer } from "./bearer.ts";
import { TestCaller, type TestServed } from "./caller.ts";
import { testCallKey } from "./context.ts";
import { testEvents } from "./events.ts";

/** The most settling rounds, each a hop of one package's controllers waking another's: 20 hops. */
const SETTLE_ROUNDS = 20;

/** How long a closing test server drains its requests, in milliseconds: far above a test call's. */
const DRAIN_MILLISECONDS = 1000;

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

/** What a test store serves: its packages, its host's tables, its clock and its space. */
export interface TestObjectStoreOptions {
    /** The packages it serves, as declared or as implemented. */
    readonly packages: readonly (ObjectPackage | PackageImplementation)[];
    /** The tables its host stores beside them. */
    readonly tables?: readonly Table[];
    /** The clock it runs at, the system's by default. */
    readonly clock?: Clock;
    /** The scope it serves, a new space by default. */
    readonly scope?: ObjectReference;
    /** Whether anyone may read the scope, public by default. */
    readonly visibility?: "public" | "private";
    /** The subjects owning the scope beside itself, as its creation binds them. */
    readonly owners?: readonly Subject[];
    /** The groups each user's credential vouches for, by user. */
    readonly groups?: ReadonlyMap<string, readonly Subject[]>;
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
    /** The failures the servers reported, such as a controller's, which settling throws. */
    readonly #failures: unknown[];
    /** The first failure a server reported, rejected once one does. */
    readonly #failed: Promise<never>;
    /** The object types the store serves by name, each with its package. */
    readonly #types: ReadonlyMap<
        string,
        { readonly type: ObjectType; readonly handle: PackageHandle }
    >;

    /** Create a test store over its database and servers. */
    private constructor(
        store: ObjectStore,
        servers: ReadonlyMap<PackageId, Server>,
        credential: TestCredential,
        disposables: AsyncDisposableStack,
        failures: unknown[],
        failed: Promise<never>,
    ) {
        // store the store, its scope, its servers, its callers' credential and what disposal closes
        this.store = store;
        this.scope = store.scope;
        this.#servers = servers;
        this.#credential = credential;
        this.#disposables = disposables;
        this.#failures = failures;
        this.#failed = failed;

        // index the served object types by name, refusing a name two packages share
        const served = store.packages.flatMap((handler) =>
            Object.values(handler.package.objects)
                .filter((declared): declared is ObjectType => declared instanceof ObjectType)
                .map((type) => ({ type, handle: handler.package })),
        );
        const types = new Map<
            string,
            { readonly type: ObjectType; readonly handle: PackageHandle }
        >();
        for (const each of served) {
            if (types.has(each.type.name)) {
                throw new TypeError(`two packages the test store serves declare ${each.type.name}`);
            }
            types.set(each.type.name, each);
        }
        this.#types = types;
    }

    /** Serve packages in a scope, a new space by default, over a database derived from them. */
    static async open(options: TestObjectStoreOptions): Promise<TestObjectStore> {
        // serve each package as implemented, or as declared without code
        const implementations = options.packages.map((each) =>
            "definition" in each ? each : implementPackage(each, {}),
        );

        // derive and migrate the database from the core and served packages and their follows
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
            { isMigrated: true },
        );

        // take the store's scope
        const scope =
            options.scope ??
            principal.space.reference(Scope.universe.id, Identifier.create("space"));

        // collect the failures the servers report, which settling and serving throw
        const failures: unknown[] = [];
        const failed = TestObjectStore.#failure();

        // store the served ephemeral types in memory, as a host does, failing the test on a lost write
        const clock = options.clock ?? Clock.system;
        const ephemeral = await EphemeralStorage.open(
            implementations.flatMap((each) => Object.values(each.package.objects)),
            EphemeralStorage.channel(storage.database),
            (tables) => connectBunSqlite(":memory:", tables),
            {
                clock,
                report: (error) => {
                    failures.push(error);
                    failed.reject(error);
                },
            },
        );

        // serve the packages through one store as the scope, or as the universe a user's scope lives at
        const store = new ObjectStore({
            servers: implementations,
            chain: async () => undefined,
            origin: { package: Package.of(objectPackage) },
            principal: principal.user.is(scope)
                ? universe.reference(Scope.universe.id, Scope.universe.id)
                : Subject.parse(scope),
            callKey: testCallKey,
            events: await testEvents(storage.database),
            database: storage.database,
            ...(ephemeral === undefined ? {} : { ephemeral }),
            clock,
        });

        // open the store's scope in the partition its policy declares
        await TestObjectStore.#openScope(
            store,
            scope,
            options.visibility ?? "public",
            options.owners ?? [],
        );

        // serve it, closing the ephemeral and durable databases after the servers
        const disposables = new AsyncDisposableStack();
        disposables.defer(() => storage.close());
        if (ephemeral !== undefined) {
            disposables.use(ephemeral);
        }

        return TestObjectStore.#serve(
            store,
            store.packages,
            TestObjectStore.#bearers(store, options.groups ?? new Map()),
            { disposables, failures, failed },
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
        } = {},
    ): TestObjectStore {
        const groups = options.groups ?? new Map();

        return TestObjectStore.#serve(
            store,
            options.packages ?? store.packages,
            options.credential ?? TestObjectStore.#bearers(store, groups),
            {
                disposables: new AsyncDisposableStack(),
                failures: [],
                failed: TestObjectStore.#failure(),
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
            readonly failures: unknown[];
            readonly failed: PromiseWithResolvers<never>;
        },
    ): TestObjectStore {
        // start one server per package, closed first on disposal, collecting what they report
        const { disposables, failures, failed } = run;
        const servers = new Map(
            packages.map((handler) => [
                handler.package.id,
                Server.start({
                    clock: store.clock,
                    ...handler,
                    // serve the store's one scope, or every scope as the universe's store does
                    ...(store.scope === Scope.universe.id ? {} : { scope: store.scope }),
                    resources: new ResourceContext(),
                    health: new Health(handler.package.name),
                    drainTimeout: DRAIN_MILLISECONDS,
                    report: (error) => {
                        failures.push(error);
                        failed.reject(error);
                    },
                    // add no host requirement, as a host serving no machines or installations
                    authorizeMachine: async () => {},
                    authenticate: (request) => credential.authenticate(request, handler.package.id),
                }),
            ]),
        );
        for (const server of servers.values()) {
            disposables.defer(() => server.close());
        }

        return new TestObjectStore(
            store,
            servers,
            credential,
            disposables,
            failures,
            failed.promise,
        );
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

    /** Open the failure that serving and settling race, leaving it to those waiting for it. */
    static #failure(): PromiseWithResolvers<never> {
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => undefined);

        return failed;
    }

    /** The store's database. */
    get database(): DatabaseConnection {
        return this.store.database;
    }

    /** Call the store as a subject, or by an identifier whose type names its principal kind. */
    as(caller: string | Subject): TestCaller {
        return new TestCaller(this, typeof caller === "string" ? this.#subject(caller) : caller);
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
    type(name: string): { readonly type: ObjectType; readonly handle: PackageHandle } | undefined {
        return this.#types.get(name);
    }

    /** Wait until every server serves, its readiness met, failing on the first reported failure. */
    async serving(): Promise<void> {
        const servers = [...this.#servers.values()];
        await Promise.race([Promise.all(servers.map((server) => server.serving())), this.#failed]);
    }

    /** Settle once no controller of any package has work left after a round committing nothing. */
    async settle(): Promise<void> {
        // settle every server again while a round commits, as one package's work wakes another's
        let before = await this.database.log.position();
        for (let round = 0; round < SETTLE_ROUNDS; round += 1) {
            await Promise.all([...this.#servers.values()].map((server) => server.idle()));
            const after = await this.database.log.position();
            if (after.epoch === before.epoch && after.sequence === before.sequence) {
                this.#throwFailures();

                return;
            }
            before = after;
        }

        // refuse controllers still committing, naming the objects their last round wrote
        const page = await this.database.log.read({
            tables: this.store.objects.map((object) => object.table),
            after: before.sequence,
        });
        const written = [...new Set(page.changes.map((change) => change.table[TABLE].sqlName))];
        throw new Error(
            `the test store's controllers kept committing after ${SETTLE_ROUNDS} rounds, last to ${written.join(", ")}`,
        );
    }

    /** Throw the failures the servers reported, once. */
    #throwFailures(): void {
        const failures = this.#failures.splice(0);
        if (failures.length > 0) {
            throw new AggregateError(failures, "the test store's servers failed");
        }
    }

    /** Close the servers, then the databases the test store opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
        this.#throwFailures();
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

    /** Replicate a scope as its universe would, with the store's principal as owner. */
    static async #openScope(
        store: ObjectStore,
        object: ObjectReference,
        visibility: "public" | "private",
        owners: readonly Subject[],
    ): Promise<void> {
        // record the scope in its own partition or its parent's, as its policy declares, and bind its owners
        const fixture = new AccessFixture(store.database);
        const isPartition = store.authorizer.policy(object).definition.partition === true;
        await fixture.replicateScope(object, isPartition ? "self" : "parent");
        for (const owner of [Subject.parse(object), ...owners]) {
            await fixture.bindOwner(object, owner, 0);
        }

        // relate anyone to a public space as its reader
        if (visibility === "public") {
            await fixture.replicateRelationship(object, "reader", anyone.reference("*", "*"), 0);
        }
    }
}
