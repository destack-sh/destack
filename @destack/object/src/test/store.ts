import {
    accessRelationship,
    anyone,
    type Authentication,
    Caller,
    principal,
    Relationship,
} from "@destack/access";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { AccessFixture } from "@destack/access/test";
import { type DatabaseConnection, defineDatabase, type Table } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { TestDatabase } from "@destack/db/test";
import { Clock } from "@destack/environment";
import { Package, type PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, schema } from "@destack/schema";
import { type ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import type { Example } from "@destack/package/declare";
import { ObjectType } from "../object/object.ts";
import { type ClientOptions, type ClientTarget, createClient } from "../client/package.ts";
import type { Client, ProcedureRouter } from "../procedure/index.ts";
import type { PackageHandle } from "../declare/package.ts";
import { Health } from "../server/health.ts";
import { Server } from "../server/http.ts";
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
import { testCallKey } from "./context.ts";
import { testEvents } from "./events.ts";

/** How long a closing test server drains its requests, in milliseconds: far above a test call's. */
const DRAIN_MILLISECONDS = 1000;

/** The origin test clients address the servers at, which no request leaves the process for. */
const TEST_ORIGIN = "https://objects.test";

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
    /** The groups each user's credential vouches for, by user. */
    readonly groups?: ReadonlyMap<string, readonly Subject[]>;
}

/** An object store serving packages in one space over a test database, called as its users. */
export class TestObjectStore implements AsyncDisposable {
    /** The store. */
    readonly store: ObjectStore;
    /** The scope it serves. */
    readonly scope: string;
    /** The server of each package. */
    readonly #servers: ReadonlyMap<PackageId, Server>;
    /** What disposal closes: the servers, and the databases the test store opened. */
    readonly #disposables: AsyncDisposableStack;
    /** The object types the store serves by name, each with its package. */
    readonly #types: ReadonlyMap<
        string,
        { readonly type: ObjectType; readonly handle: PackageHandle }
    >;

    /** Create a test store over its database and servers. */
    private constructor(
        store: ObjectStore,
        servers: ReadonlyMap<PackageId, Server>,
        disposables: AsyncDisposableStack,
    ) {
        // store the store, its scope, its servers and what disposal closes
        this.store = store;
        this.scope = store.scope;
        this.#servers = servers;
        this.#disposables = disposables;

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

    /** Serve packages in a new space over a database derived from them, of the project's target. */
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

        // open the store's scope
        const scope =
            options.scope ??
            principal.space.reference(Scope.universe.id, Identifier.create("space"));
        await TestObjectStore.#openScope(storage.database, scope, options.visibility ?? "public");

        // store the served ephemeral types in memory, as a host does, failing the test on a lost write
        const clock = options.clock ?? Clock.system;
        const ephemeral = await EphemeralStorage.open(
            implementations.flatMap((each) => Object.values(each.package.objects)),
            EphemeralStorage.channel(storage.database),
            (tables) => connectBunSqlite(":memory:", tables),
            {
                clock,
                report: (error) => {
                    throw error;
                },
            },
        );

        // serve the packages through one store as the space
        const store = new ObjectStore({
            servers: implementations,
            chain: async () => undefined,
            origin: { package: Package.of(objectPackage) },
            principal: principal.space.reference(Scope.universe.id, scope.id),
            callKey: testCallKey,
            events: await testEvents(storage.database),
            database: storage.database,
            ...(ephemeral === undefined ? {} : { ephemeral }),
            clock,
        });

        // serve it, closing the ephemeral and durable databases after the servers
        const disposables = new AsyncDisposableStack();
        disposables.defer(() => storage.close());
        if (ephemeral !== undefined) {
            disposables.use(ephemeral);
        }

        return TestObjectStore.#serve(store, options.groups ?? new Map(), disposables);
    }

    /** Serve an existing store's packages over HTTP to bearer users, leaving its databases open. */
    static serve(
        store: ObjectStore,
        options: {
            /** The groups each user's credential vouches for, by user. */
            readonly groups?: ReadonlyMap<string, readonly Subject[]>;
        } = {},
    ): TestObjectStore {
        return TestObjectStore.#serve(
            store,
            options.groups ?? new Map(),
            new AsyncDisposableStack(),
        );
    }

    /** Serve a store's packages to the users its requests name, running its controllers. */
    static #serve(
        store: ObjectStore,
        groups: ReadonlyMap<string, readonly Subject[]>,
        disposables: AsyncDisposableStack,
    ): TestObjectStore {
        // start one server per package, closed first on disposal
        const servers = new Map(
            store.packages.map((handler) => [
                handler.package.id,
                Server.start({
                    clock: store.clock,
                    ...handler,
                    scope: store.scope,
                    resources: new ResourceContext(),
                    health: new Health(handler.package.name),
                    drainTimeout: DRAIN_MILLISECONDS,
                    // add no host requirement, as a host serving no machines or installations
                    authorizeMachine: async () => {},
                    authenticate: async (request) =>
                        TestObjectStore.#authenticate(request, {
                            audience: handler.package.id,
                            scope: store.scope,
                            groups,
                            now: store.clock.now(),
                        }),
                }),
            ]),
        );
        for (const server of servers.values()) {
            disposables.defer(() => server.close());
        }

        return new TestObjectStore(store, servers, disposables);
    }

    /** The store's database. */
    get database(): DatabaseConnection {
        return this.store.database;
    }

    /** Call a package the store serves as a user, through its typed client. */
    client<Definition extends ProcedureRouter>(
        handle: PackageHandle & ClientTarget<Definition>,
        user: Identifier<"user">,
    ): Client<Definition> {
        return createClient(handle, this.endpoint(handle, user));
    }

    /** Address a package the store serves as a user. */
    endpoint(handle: PackageHandle, user: Identifier<"user">): ClientOptions {
        const server = this.server(handle);

        return {
            url: TEST_ORIGIN,
            headers: { authorization: `Bearer ${user}` },
            fetch: (request: Request) => server.fetch(request),
        };
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

    /** Make an example's calls in the store's space as a user, each through the user's client. */
    async given(example: Pick<Example, "objects">, user: Identifier<"user">): Promise<void> {
        for (const [scope, calls] of Object.entries(example.objects)) {
            // refuse calls in a scope the store does not serve
            if (scope !== "space") {
                throw new TypeError(
                    `the test store serves its space alone, not the example's ${scope}`,
                );
            }

            // call each method in the store's scope as the user, through the package's client
            for (const { object, method, input } of calls) {
                const served = this.#types.get(object.name);
                if (served === undefined) {
                    throw new TypeError(`the test store serves no object type ${object.name}`);
                }
                const { type, handle } = served;
                const procedure: unknown = Reflect.get(
                    Reflect.get(this.client(handle, user), type.name),
                    method,
                );
                if (typeof procedure !== "function") {
                    throw new TypeError(`${type.name} has no method ${method}`);
                }
                await Reflect.apply(procedure, undefined, [
                    {
                        ...type.in(this.scope),
                        ...(type.method(method).mutates
                            ? { requestId: RequestId.create(this.store.clock.now()) }
                            : {}),
                        ...input,
                    },
                ]);
            }
        }
    }

    /** Settle once no controller of any package has a key due or reconciling. */
    async settle(): Promise<void> {
        await Promise.all([...this.#servers.values()].map((server) => server.idle()));
    }

    /** Close the servers, then the databases the test store opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
    }

    /** Authenticate a bearer: the store's scope as the space, else a user in its groups. */
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

        // sign the store's scope in as the space
        const bearer = TestBearer.read(request);
        if (bearer === served.scope) {
            const space = principal.space.reference(Scope.universe.id, bearer);

            return TestBearer.authenticate(space, served.audience, served.now);
        }

        // sign any other bearer's user in to the scope for a token's lifetime
        const user = schema.identifier("user").parse(bearer);
        const subject = principal.user.reference(Scope.universe.id, user);

        return {
            caller: { ...Caller.of(subject), groups: served.groups.get(user) ?? [] },
            credential: {
                kind: "session",
                id: user,
                audience: served.audience,
                scope: served.scope,
                verifiedAt: served.now,
                expiresAt: served.now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
    }

    /** Replicate a scope as its universe would, with the store's principal as owner. */
    static async #openScope(
        database: DatabaseConnection,
        object: ObjectReference,
        visibility: "public" | "private",
    ): Promise<void> {
        // record the scope, with the store's principal as owner as its creation binds it
        const fixture = new AccessFixture(database);
        await fixture.replicateScope(object, "self");
        await fixture.bindOwner(object, principal.space.reference(Scope.universe.id, object.id), 0);

        // relate anyone to a public space as its reader
        if (visibility === "public") {
            await database.insert(accessRelationship).values(
                Relationship.encode(
                    {
                        id: Identifier.create("relationship"),
                        object,
                        relation: "reader",
                        subject: anyone.reference("*", "*"),
                        createdAt: 0,
                        expiresAt: null,
                    },
                    object.id,
                ),
            );
        }
    }
}
