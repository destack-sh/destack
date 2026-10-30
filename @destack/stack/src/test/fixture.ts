import { copyRequest, Crypto } from "@destack/environment";
import { InstallationDefinition, StackDefinition } from "../declare/index.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { type Connector, Plan } from "@destack/resource";
import { TestSpaceDatabaseHost } from "./space.ts";
import { dirname, join } from "node:path";
import { LocalBlobStorage, LocalBucketHost } from "@destack/bucket/bun";
import {
    BlobStore,
    Change,
    type Controller,
    ControlLoop,
    type DatabaseConnection,
    defineDatabase,
    eq,
    type LogPosition,
    type ReconcileController,
    Snapshot,
    type Table,
    TABLE,
    Transaction,
} from "@destack/db";
import type { BuildReader } from "@destack/package/manifest";
import { RequestId, Scope } from "@destack/sync";
import {
    TestBearer,
    TestCaller,
    type TestCredential,
    testEvents,
    TestObjectStore,
    type TestServed,
} from "@destack/object/test";
import { TokenVerifier } from "@destack/access/token";
import { reconciliation, TestDatabase, TestTemplate, type TemplateStorage } from "@destack/db/test";
import { expect, onTestFinished, single } from "@destack/test";
import { Failure } from "@destack/error";
import { SettingValue } from "@destack/setting/object";
import { AccessContext, Authorization, Caller, CALLER_HEADER, principal } from "@destack/access";
import { Exchange } from "@destack/access/token";
import {
    Account,
    space,
    entitlement,
    Identity,
    placement,
    Residency,
    type ResidencyCode,
} from "@destack/account/object";
import { Jwks, type KeyLookup } from "@destack/identity";
import { identityKey } from "@destack/account/identity";
import { OPERATOR_EMAIL, STACKS } from "@destack/account/test";
import accountPackage from "@destack/account/package";
import {
    Egress,
    type Follow,
    HookSandbox,
    InstallationMount,
    ObjectType,
    type PackageHandle,
    PackageMount,
    Provisioned,
    Installation,
    installation,
} from "@destack/object";
import { ObjectStore, type PackageImplementation } from "@destack/object/server";
import { connectBunSqlite } from "@destack/db/bun";
import { Package, PackageId, type PackageRelease } from "@destack/package";
import settingPackage from "@destack/setting/package";
import { describeSetting } from "@destack/setting/inspect";
import { buildRetention } from "../setting/index.ts";
import {
    canonicalize,
    Digest,
    Identifier,
    type JsonObject,
    type JsonValue,
    nth,
    present,
    schema,
} from "@destack/schema";
import objectPackage from "@destack/object/package";
import { ServerContext } from "@destack/object/workload";
import { Simulation } from "@destack/environment/simulation";
import { Database, InstallationRevision, Rollout } from "../object/index.ts";
import { BuildCache, BuildStore } from "../server/installation/build.ts";
import { REFUSAL_REASONS } from "../server/installation/application.ts";
import { StackDocument, type StackTypes } from "../declare/stack.ts";
import { Handoff } from "../server/host/handoff.ts";
import * as object from "../object/index.ts";
import {
    SocketRegistry,
    ConsumerController,
    serveSpace,
    SpaceHost,
    type SpaceMachine,
    type SpaceOptions,
    SPACE_KERNEL,
    type StoreProvider,
    type SpaceProvider,
} from "../server/index.ts";
import { universeDatabase } from "@destack/universe";
import {
    MachineWorld,
    TestMachine,
    TestUniverse,
    type TestUniverseOptions,
    UniverseWorld,
    UNIVERSE_NODE,
} from "@destack/universe/test";
import type { TestPerson } from "@destack/account/test";
import type { ClientOf } from "@destack/object/client";
import { spaceReplicated, spaceTables } from "../stack/index.ts";
import stackPackage, { PUSH_PATH } from "../package.ts";
import type { EventStore } from "@destack/event";
import { TestRegistry } from "./build.ts";
import { TestGateway } from "./gateway.ts";
import { TestRuntime } from "./runtime.ts";
import { EgressCaller, ProcessRuntime } from "../runtime/index.ts";
import { insertBuildScope, insertRelease } from "./install.ts";
import { BunGraphStorage } from "../bun/graph.ts";
import { PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import type { StackSource } from "@destack/account/object";
import { PackageClient, PackageStore } from "@destack/build/store";
import { TestPackageStore } from "@destack/build/test";

/** The main database of a space a test machine serves without other modules. */
export const spaceDatabase = defineDatabase({
    name: "main",
    tables: spaceTables({ tables: [identityKey] }).tables,
    replicated: [...spaceReplicated, entitlement.table],
});

/** Run a cleanup once a fixture's test or template write ends. */
type Defer = (cleanup: () => Promise<void> | void) => void;

/** The releases a test machine ships with the fixture's space, built in its world. */
export interface StackReleases {
    /** The fixture space's stack: the empty stack of a space without apps. */
    readonly space: BuildReader;
    /** The stack package's release, declaring its settings. */
    readonly stack: BuildReader;
    /** The stack of the fixture's space: the empty stack's release. */
    readonly source: StackSource;
    /** The digest of the stack package's release. */
    readonly manifest: Digest;
}

/** The residency a test universe declares, whose home a residency fixture hosts. */
const RESIDENCY: ResidencyCode = "eu";

/** The options a template's world leaves unset: those making another space or universe. */
const WORLD_OPTIONS = [
    "world",
    "owner",
    "machine",
    "account",
    "id",
    "name",
    "stack",
    "releases",
] as const satisfies readonly (keyof TestSpaceOptions)[];

/** Describe a space world's state outside its databases: keys, files, stack and a kit's world. */
function spaceWorldOf<World extends JsonValue>(world: schema.Schema<World>) {
    return schema.object({
        /** The universe's keys and its people's sign-ins. */
        universe: UniverseWorld,
        /** The machine's keyring secrets. */
        machine: MachineWorld,
        /** The directory of the machine's buckets and the spaces' databases, copied per test. */
        files: schema.string(),
        /** The spaces the world hosts, the born space first. */
        spaces: schema.array(
            schema.object({
                /** The space's database in the template. */
                key: schema.string(),
                /** The space's identifier. */
                id: schema.identifier("space"),
                /** The account the space belongs to. */
                account: schema.identifier("account"),
                /** The email of the space's owner, the operator for a residency's home. */
                owner: schema.string(),
                /** The stack the space declares, as the write last applied it. */
                stack: schema.jsonObject(),
                /** The releases the registry published, published again on restore. */
                releases: schema.array(Digest),
                /** The release of each shipped server's package the write installed, by package. */
                bundled: schema.array(schema.object({ packageId: PackageId, manifest: Digest })),
            }),
        ),
        /** What the kit's write returned. */
        world,
    });
}

/** A space world's state in addition to its databases, with a kit's world. */
type SpaceWorld<World extends JsonValue> = schema.Infer<ReturnType<typeof spaceWorldOf<World>>>;

/** The born space alone, without a kit's world. */
export const BORN_SPACE: SpaceWorldWrite<null> = {
    name: "space",
    value: schema.null(),
    write: async () => null,
};

/** The world of a residency's home hosted on its cloud machine, its kit writing nothing. */
export const RESIDENCY_HOME: SpaceWorldWrite<null> = {
    name: "residency",
    value: schema.null(),
    spaces: [{ residency: RESIDENCY }],
    write: async () => null,
};

/** A kit's world a space template writes on its born space, and its copies share. */
export interface SpaceWorldWrite<World extends JsonValue> {
    /** The world's name, which is also its template's name. */
    readonly name: string;
    /** What the world's copies share, as its JSON form crosses to the tests. */
    readonly value: schema.Schema<World>;
    /** The spaces the world hosts on one machine, the born space first. */
    readonly spaces?: readonly WorldSpace[];
    /** Write the world on the born space and each space by id, closing what it serves. */
    readonly write: (
        space: TestSpace,
        directory: string,
        spaces: ReadonlyMap<Identifier<"space">, TestSpace>,
    ) => Promise<World>;
}

/** A space an owner creates in an account: its identifier, the account and its name. */
export type HostedSpace = Required<Pick<TestSpaceOptions, "id" | "account" | "name">>;

/** A space a world hosts: an owner's space, a person's home by email, or a residency's home. */
export type WorldSpace =
    | HostedSpace
    | { readonly home: string }
    | { readonly residency: ResidencyCode };

/** The databases of a space world: the universe's, the machine's replica and each space's by id. */
type SpaceDatabases = {
    readonly universe: typeof universeDatabase;
    readonly machine: typeof TestMachine.database;
} & Readonly<Record<string, ReturnType<typeof defineDatabase>>>;

/** The directory below the machine's files that stores its own resources, such as Git storage. */
const RESOURCES = "resources";

/** The origin a test calls its served space at. */
const SPACE_URL = "http://space.test";

/** The most rounds of controller writes a settle follows. */
const SETTLE_ROUNDS = 20;

/** The scheme a request's authorization header puts its bearer under (RFC 6750 2.1). */
const BEARER_PREFIX = "Bearer ";

/** A JSON Web Token's compact form: three base64url segments (RFC 7519 3). */
const JWT_FORM = /^[\w-]+\.[\w-]+\.[\w-]*$/u;

/** The seed of the simulated worlds templates write and restores run in. */
const WORLD_SEED = 1;

/** The default start moment of a template's world: 2026-10-01T00:00:00Z. */
const WORLD_START = Date.UTC(2026, 9, 1);

/** The S3 credentials of the fixture's buckets. */
const BUCKET_CREDENTIALS = {
    accessKeyId: "AKIDSTACK",
    secretAccessKey: "fixture-secret-access-key",
};

/** The email the fixture space's owner signs up with. */
export const OWNER_EMAIL = "owner@example.test";

/** The identities the scenarios name, the fixture's account and space created at theirs. */
export const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    stackPackage: PackageId.parse("package-01996ab0-0000-7000-8000-00000000000a"),
    notes: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000005"),
    run: schema.identifier("run").parse("run-01996ab0-0000-7000-8000-000000000006"),
    machine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000007"),
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
};

/** The fixture's account, which its spaces belong to. */
const FIXTURE_ACCOUNT = { id: ids.account, handle: "acme" } as const;

/** The fixture's space, which a world hosts alone unless it lists others. */
const FIXTURE_SPACE: HostedSpace = { id: ids.space, account: ids.account, name: "space" };

/** A space of a test universe, hosted on its residency's cloud machine over a test database. */
export class TestSpace implements AsyncDisposable {
    /** The hosted space. */
    readonly id: Identifier<"space">;
    /** The account the space belongs to. */
    readonly account: Identifier<"account">;
    /** The universe the space lives in. */
    readonly universe: TestUniverse;
    /** The space's owner, signed up at the universe. */
    readonly owner: TestPerson;
    /** The residency's cloud machine hosting the space. */
    readonly machine: TestMachine;
    /** The world the universe, the machine and the space run in, which tests move through. */
    readonly simulation: Simulation;
    /** The space's host on the machine: its identity and its link to the universe. */
    readonly host: SpaceHost;
    /** The test database. */
    readonly #test: TestDatabase;
    /** The closing of the machine's database, absent while open. */
    #closed: Promise<void> | undefined;
    /** The machine's bucket host. */
    readonly #buckets: LocalBucketHost;
    /** The directory that stores the space's own databases, such as main. */
    readonly #main: string;
    /** The space's blobs, in the machine's files directory, referenced from its database. */
    readonly blobs: BlobStore<LocalBlobStorage>;
    /** The space's builds, read through to the machine's cache. */
    readonly #builds: BuildStore;
    /** The packages the space serves in addition to the stack's, with their events and sockets. */
    readonly #packages: readonly Pick<PackageHandle, "objects" | "tables">[];
    /** The machine's connectors of bound resources, such as mail, by provider code. */
    readonly #connectors: readonly Connector[];
    /** What each serve of the space serves in addition to its packages. */
    readonly #beside: Required<Pick<TestSpaceOptions, "companions" | "providers">>;
    /** The release the machine ships of each shipped server's package, as the test installs it. */
    readonly #bundled: Map<PackageId, PackageRelease>;
    /** The releases the machine ships with the space. */
    readonly releases: StackReleases;
    /** The index of the builds, opened once. */
    #index: Promise<BunGraphStorage> | undefined;
    /** The registry the space's releases come from. */
    readonly registry: TestRegistry;
    /** The world's machines answering at their origins, this space's machine among them. */
    readonly gateway: TestGateway;
    /** The stack the space declares, as the test last applied it. */
    #stack: JsonObject = {};
    /** Run a cleanup once the fixture's test or template write ends. */
    readonly #defer: Defer;

    /** The world the space's machine runs in, with every capability. */
    get environment(): Environment<HostCapability> {
        return this.simulation.environment(UNIVERSE_NODE);
    }

    /** Store a hosted space over its test database. */
    private constructor(fields: {
        readonly id: Identifier<"space">;
        readonly account: Identifier<"account">;
        readonly universe: TestUniverse;
        readonly owner: TestPerson;
        readonly machine: TestMachine;
        readonly simulation: Simulation;
        readonly registry: TestRegistry;
        readonly gateway: TestGateway;
        readonly host: SpaceHost;
        readonly test: TestDatabase;
        readonly buckets: LocalBucketHost;
        readonly main: string;
        readonly blobs: BlobStore<LocalBlobStorage>;
        readonly builds: BuildStore;
        readonly packages: readonly Pick<PackageHandle, "objects" | "tables">[];
        readonly connectors: readonly Connector[];
        readonly beside: Required<Pick<TestSpaceOptions, "companions" | "providers">>;
        readonly bundled: Map<PackageId, PackageRelease>;
        readonly releases: StackReleases;
        readonly defer: Defer;
    }) {
        // store the space, its universe, owner, machine and host, the machine's stores and shipped releases
        this.id = fields.id;
        this.account = fields.account;
        this.universe = fields.universe;
        this.#packages = fields.packages;
        this.#connectors = fields.connectors;
        this.#beside = fields.beside;
        this.#bundled = fields.bundled;
        this.releases = fields.releases;
        this.#buckets = fields.buckets;
        this.#main = fields.main;
        this.blobs = fields.blobs;
        this.#builds = fields.builds;
        this.owner = fields.owner;
        this.machine = fields.machine;
        this.simulation = fields.simulation;
        this.registry = fields.registry;
        this.gateway = fields.gateway;
        this.host = fields.host;
        this.#test = fields.test;
        this.#defer = fields.defer;
    }

    /** Stop the fixture's own replica follow, while it runs. */
    #following: (() => Promise<void>) | undefined;

    /** The machine's database. */
    get database(): DatabaseConnection {
        return this.#test.database;
    }

    /** The directory below which the machine stores its own resources, such as Git storage. */
    get resources(): string {
        return join(dirname(this.#main), RESOURCES);
    }

    /** Open a space made at the universe, the fixture's account and space by default, hosted. */
    static async open(options: TestSpaceOptions = {}): Promise<TestSpace> {
        // restore the template's world for the fixture's own space in a fork of it, refusing a given world
        const isWorld = WORLD_OPTIONS.every((name) => options[name] === undefined);
        if (isWorld) {
            const template = TestSpace.template(options, BORN_SPACE);
            const copied = await TestSpace.copy(template, options);

            return copied.space;
        }

        // make it in the test's world, else in the fixture's own, run through the world's simulation
        const simulation = TestSpace.#simulation(options);
        const defer = TestSpace.#afterTest(simulation);

        return await simulation.settle(TestSpace.#create(options, { defer, simulation }));
    }

    /** Host the residency's home on its cloud machine as the operator. */
    static async residency(options: Omit<TestSpaceOptions, "owner"> = {}): Promise<TestSpace> {
        // copy the run's written home when the test brings no world of its own
        const given: TestSpaceOptions = options;
        const isWorld = WORLD_OPTIONS.every((name) => given[name] === undefined);
        if (isWorld) {
            const template = TestSpace.template(options, RESIDENCY_HOME);
            const copied = await TestSpace.copy(template, options);

            return copied.space;
        }

        // host it in the test's world, run through the world's simulation
        const simulation = TestSpace.#simulation(options);
        const defer = TestSpace.#afterTest(simulation);

        return await simulation.settle(
            TestSpace.#residency(options, RESIDENCY, { defer, simulation }),
        );
    }

    /** Host a residency's home on its cloud machine as the operator, born as at its first start. */
    static async #residency(
        options: Omit<TestSpaceOptions, "owner">,
        code: ResidencyCode,
        lifetime: {
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
            /** The template's database the home goes into, a new one by default. */
            readonly database?: TestDatabase;
            /** The directory the machine's files stay in, a temporary one by default. */
            readonly files?: string;
            /** The world the universe, the machine and the home run in. */
            readonly simulation: Simulation;
        },
    ): Promise<TestSpace> {
        // read the home the universe's definition created, and the platform account of its owner
        const { defer, simulation } = lifetime;
        const world = await TestSpace.#universe(options, defer, simulation);
        const { universe } = world;
        const snapshot = Snapshot.live(universe.storage.database);
        const home = await Residency.space(snapshot, code);
        const row = await snapshot.read(space.table).findFirst({ where: { id: home } });
        const hosted = { id: home, account: present(row, `the home of ${code}`).scope };

        // host it as the operator once the residency's machine follows its placement
        const owner = await universe.operator();
        const given = { ...options, world, owner };
        const test =
            lifetime.database ??
            (await TestDatabase.create(TestSpace.database(given), {
                environment: universe.environment,
                isMigrated: true,
            }));
        const machine = options.machine ?? (await universe.machine());
        const files = await TestSpace.#openFiles(
            simulation.environment(UNIVERSE_NODE),
            defer,
            lifetime.files,
        );
        const fixture = await TestSpace.#hosting(world, owner, test, hosted, given, {
            machine,
            files,
            defer,
        });

        // start it once for its birth, as its machine's first start does, then keep its replicas followed as its machine does
        const born = await fixture.serve();
        await born[Symbol.asyncDispose]();
        await fixture.#follow();

        return fixture;
    }

    /** Define a template of the space's world at a start, and of a kit's world written on it. */
    static template<World extends JsonValue>(
        options: TestSpaceOptions,
        world: SpaceWorldWrite<World>,
    ): TestTemplate<SpaceWorld<World>, SpaceDatabases> {
        // read the spaces' tables and its world's start
        const database = TestSpace.database(options);
        const names = database.tables.map((table) => table[TABLE].sqlName).toSorted();
        const { start = WORLD_START } = options;
        const spaces = world.spaces ?? [FIXTURE_SPACE];

        // name the template by its start, the kit's world and the tables, written in a world of its own
        return new TestTemplate({
            name: `stack.world ${new Date(start).toISOString()} ${world.name} ${names.join(" ")}`,
            databases: {
                universe: universeDatabase,
                machine: TestMachine.database,
                ...Object.fromEntries(spaces.map((each) => [TestSpace.#key(each), database])),
            },
            value: spaceWorldOf(world.value),
            world: { seed: WORLD_SEED, start },
            write: async (databases, directory, simulation) => {
                // run the world's simulation over the template's databases and born space
                return await simulation.settle(
                    TestSpace.#write(options, world, simulation, databases, directory),
                );
            },
        });
    }

    /** Write a template's world and a kit's over its databases and directory. */
    static async #write<World extends JsonValue>(
        options: TestSpaceOptions,
        world: SpaceWorldWrite<World>,
        simulation: Simulation,
        databases: TemplateStorage<SpaceDatabases>,
        directory: string,
    ): Promise<SpaceWorld<World>> {
        // open the universe, the owner and the machine over the template's databases, closed once written
        await using lifetime = new AsyncDisposableStack();
        const defer: Defer = (cleanup) => lifetime.defer(cleanup);
        const shared = await TestSpace.#universe(options, defer, simulation, databases.universe);
        const { universe } = shared;
        const owner = await universe.signIn(OWNER_EMAIL);
        const machine = await universe.machine(undefined, { storage: databases.machine });

        // host each space over its template database, its files below its own directory
        const spaces = new Map<Identifier<"space">, TestSpace>();
        const written: SpaceWorld<null>["spaces"][number][] = [];
        const accounts = new Set<Identifier<"account">>();
        for (const listed of world.spaces ?? [FIXTURE_SPACE]) {
            const key = TestSpace.#key(listed);
            const hosting = {
                defer,
                database: present(databases[key], `the database of ${key}`),
                files: join(directory, key),
                simulation,
            };
            const given = { ...options, world: shared, machine };

            // host a residency's home as the operator
            if ("residency" in listed) {
                const fixture = await TestSpace.#residency(given, listed.residency, hosting);
                spaces.set(fixture.id, fixture);
                written.push({ key, ...fixture.#state(), owner: OPERATOR_EMAIL });
            }
            // host a person's personal home, which their sign-up created, the owner's or another's
            else if ("home" in listed) {
                const person =
                    listed.home === OWNER_EMAIL ? owner : await universe.signIn(listed.home);
                const account = Account.personal(person.id);
                const home = { account, id: Account.home(account), name: "home" };
                const fixture = await TestSpace.#create(
                    { ...given, owner: person, ...home },
                    hosting,
                );
                spaces.set(fixture.id, fixture);
                written.push({ key, ...fixture.#state(), owner: listed.home });
            }
            // create another space as the owner, and its account at its first space, the fixture's as acme
            else {
                if (!accounts.has(listed.account)) {
                    const isFixture = listed.account === FIXTURE_ACCOUNT.id;
                    const handle = isFixture ? FIXTURE_ACCOUNT.handle : listed.name;
                    await TestSpace.account(universe, owner, { id: listed.account, handle });
                    accounts.add(listed.account);
                }
                const fixture = await TestSpace.#create({ ...given, owner, ...listed }, hosting);
                spaces.set(fixture.id, fixture);
                written.push({ key, ...fixture.#state(), owner: OWNER_EMAIL });
            }
        }

        // write the kit's world on the born space, then return what a copy restores of each space
        const born = present(spaces.values().next().value, "the born space");
        const kit = await world.write(born, directory, spaces);

        return {
            universe: await universe.world(),
            machine: machine.world(),
            files: directory,
            spaces: written.map((entry) => ({
                ...entry,
                ...present(spaces.get(entry.id), entry.id).#state(),
            })),
            world: kit,
        };
    }

    /** Name a world's space's database in its template. */
    static #key(listed: WorldSpace): string {
        // key a residency's home by its code, the owner's home by its owner, and others by identifier
        if ("residency" in listed) {
            return `residency-${listed.residency}`;
        } else if ("home" in listed) {
            return `home-${listed.home}`;
        }

        return listed.id;
    }

    /** Describe what a copy restores of the space in addition to its database and owner. */
    #state(): Omit<SpaceWorld<null>["spaces"][number], "key" | "owner"> {
        const bundled = [...this.#bundled.values()]
            .filter((release) => release.package.id !== stackPackage.id)
            .map((release) => ({ packageId: release.package.id, manifest: release.manifest }));

        return {
            id: this.id,
            account: this.account,
            stack: this.#stack,
            releases: [...this.registry.manifests],
            bundled,
        };
    }

    /** Copy a template's world into a fork of it at the write's end, with the kit's world. */
    static async copy<World extends JsonValue>(
        template: TestTemplate<SpaceWorld<World>, SpaceDatabases>,
        options: TestSpaceOptions = {},
    ): Promise<{
        readonly space: TestSpace;
        readonly spaces: ReadonlyMap<Identifier<"space">, TestSpace>;
        readonly world: World;
    }> {
        // copy the template's databases, then restore its world in the fork's simulation
        const copied = await TestDatabase.copy(template, { label: "copy" });

        return await copied.simulation.settle(TestSpace.#restore(copied, options));
    }

    /** Make the space at the universe, host and birth it, over the given databases when written. */
    static async #create(
        options: TestSpaceOptions,
        lifetime: {
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
            /** The template's database the space goes into, a new one by default. */
            readonly database?: TestDatabase;
            /** The directory the machine's files stay in, a temporary one by default. */
            readonly files?: string;
            /** The world the universe, the machine and the space run in. */
            readonly simulation: Simulation;
        },
    ): Promise<TestSpace> {
        // sign the owner up, and create the fixture's account unless the space goes in a given one
        const { defer, database, simulation } = lifetime;
        const world = await TestSpace.#universe(options, defer, simulation);
        const { universe } = world;
        const owner = options.owner ?? (await universe.signIn(OWNER_EMAIL));
        const { account = ids.account, id = ids.space, name = "space", stack } = options;
        const accounts =
            options.account === undefined
                ? await TestSpace.account(universe, owner)
                : owner.client(accountPackage);

        // create the space in the account as the owner, born with the given stack or the universe's, leaving the account's home its creation made
        if (id !== Account.home(account)) {
            await accounts.space.create({
                accountId: account,
                id,
                requestId: RequestId.create(universe.environment),
                name,
                ...(stack === undefined ? {} : { stack }),
            });
        }

        // host it once the residency's machine follows its placement
        const test =
            database ??
            (await TestDatabase.create(TestSpace.database(options), {
                environment: universe.environment,
                isMigrated: true,
            }));
        const machine = options.machine ?? (await universe.machine());
        const files = await TestSpace.#openFiles(
            simulation.environment(UNIVERSE_NODE),
            defer,
            lifetime.files,
        );
        const hosted = { id, account };
        const fixture = await TestSpace.#hosting(world, owner, test, hosted, options, {
            machine,
            files,
            defer,
        });

        // start it once for its birth, as its machine's first start does, then keep its replicas followed as its machine does
        const born = await fixture.serve();
        await born[Symbol.asyncDispose]();
        await fixture.#follow();

        return fixture;
    }

    /** Restore the world a template wrote: universe, owner, machine, each space and kit's world. */
    static async #restore<World extends JsonValue>(
        copied: {
            readonly simulation: Simulation;
            readonly databases: TemplateStorage<SpaceDatabases>;
            readonly value: SpaceWorld<World>;
        },
        options: TestSpaceOptions,
    ): Promise<{
        readonly space: TestSpace;
        readonly spaces: ReadonlyMap<Identifier<"space">, TestSpace>;
        readonly world: World;
    }> {
        // run the restore's cleanups once the test ends, in the fork's simulation
        const { simulation, databases, value } = copied;
        const defer = TestSpace.#afterTest(simulation);

        // restore the universe and its machine from the world's keys on a running clock, closed in turn once the test ends
        const environment = simulation.environment(UNIVERSE_NODE);
        const closing = new AsyncDisposableStack();
        const world = await TestSpace.#universe(
            options,
            (cleanup) => closing.defer(cleanup),
            simulation,
            databases.universe,
            value.universe,
        );
        const { universe } = world;
        const machine = await universe.machine(undefined, {
            storage: databases.machine,
            world: value.machine,
        });
        defer(async () => {
            await closing.disposeAsync();
            await databases.machine.close();
            await databases.universe.close();
        });

        // copy the machine's files, and host each space over its own
        const directory = await environment.files.makeTemporaryDirectory("destack-stack-files-");
        await environment.files.copy(value.files, directory);
        defer(() => environment.files.remove(directory, { recursive: true }));
        const spaces = new Map<Identifier<"space">, TestSpace>();
        for (const restored of value.spaces) {
            const files = await TestSpace.#openFiles(
                environment,
                defer,
                join(directory, restored.key),
            );
            const database = present(databases[restored.key], `the database of ${restored.key}`);
            const owner = universe.person(restored.owner);
            const fixture = await TestSpace.#hosting(world, owner, database, restored, options, {
                machine,
                files,
                defer,
            });
            await fixture.#restoreState(restored, options);
            await fixture.#follow();
            spaces.set(restored.id, fixture);
        }
        const born = present(spaces.values().next().value, "the born space");

        return { space: born, spaces, world: value.world };
    }

    /** Publish a write's releases again, and store the stack and servers it installed. */
    async #restoreState(
        restored: SpaceWorld<null>["spaces"][number],
        options: TestSpaceOptions,
    ): Promise<void> {
        // publish the releases the write published again, from the machine's builds of them
        for (const manifest of restored.releases) {
            if (!(await this.registry.contains(manifest))) {
                await this.registry.push(this.#builds, manifest);
            }
        }

        // store the stack the write applied, and ship the releases it installed of the servers given
        this.#stack = restored.stack;
        for (const { packageId, manifest } of restored.bundled) {
            const server = (options.servers ?? []).find((each) => each.package.id === packageId);
            if (server !== undefined) {
                this.#bundled.set(packageId, { package: Package.of(server.package), manifest });
            }
        }
    }

    /** Create the fixture's account as its owner, answering the owner's account client. */
    static async account(
        universe: TestUniverse,
        owner: TestPerson,
        created: { readonly id: Identifier<"account">; readonly handle: string } = FIXTURE_ACCOUNT,
    ): Promise<ClientOf<typeof accountPackage>> {
        const accounts = owner.client(accountPackage);
        await accounts.account.create({
            id: created.id,
            scope: owner.subject.id,
            requestId: RequestId.create(universe.environment),
            handle: created.handle,
            name: created.handle,
            residencyId: "eu",
            kind: "shared",
        });

        return accounts;
    }

    /** Open a machine without a space of its own yet, such as the target of a move. */
    static async empty(options: TestSpaceOptions = {}): Promise<TestSpace> {
        // take the universe and the owner, and host an empty database of the test's tables
        const simulation = TestSpace.#simulation(options);
        const defer = TestSpace.#afterTest(simulation);
        const world = await TestSpace.#universe(options, defer, simulation);
        const { universe } = world;
        const owner = options.owner ?? (await universe.signIn(OWNER_EMAIL));
        const machine = options.machine ?? (await universe.machine());
        const files = await TestSpace.#openFiles(simulation.environment(UNIVERSE_NODE), defer);

        return await TestSpace.#hosting(
            world,
            owner,
            await TestDatabase.create(TestSpace.database(options), {
                environment: universe.environment,
                isMigrated: true,
            }),
            { id: ids.space, account: ids.account },
            options,
            { machine, files, defer },
        );
    }

    /** Derive the space's database from the test's packages and follows and the stack's. */
    static database(options: TestSpaceOptions): ReturnType<typeof defineDatabase> {
        // take the stack's own database when the test adds nothing
        const { packages = [], follows = [], tables = [] } = options;
        if (packages.length === 0 && follows.length === 0 && tables.length === 0) {
            return spaceDatabase;
        }

        // derive the space's tables and replicas with the test's
        const derived = spaceTables({ packages, follows, tables: [identityKey, ...tables] });

        return defineDatabase({
            name: "main",
            tables: derived.tables,
            replicated: [...spaceReplicated, entitlement.table, ...(derived.replicated ?? [])],
        });
    }

    /** Run a cleanup once the test ends, in the simulation of the world it closes in. */
    static #afterTest(simulation: Simulation): Defer {
        // run the cleanups in reverse once the test ends, closing what serves before what it serves from
        const cleanups = new AsyncDisposableStack();
        onTestFinished(() => simulation.settle(cleanups.disposeAsync()));

        return (cleanup) => {
            cleanups.defer(cleanup);
        };
    }

    /** Take the simulation of the world the space joins, else start the fixture's own. */
    static #simulation(options: TestSpaceOptions): Simulation {
        return options.world?.simulation ?? TestSpace.#world(WORLD_START);
    }

    /** Start the simulated world a template writes or a restore runs in, its clock at a start. */
    static #world(start: number): Simulation {
        return new Simulation({ seed: WORLD_SEED, start });
    }

    /** Build the releases a test machine ships with the fixture's space, with a world's crypto. */
    static async releases(crypto: Crypto): Promise<StackReleases> {
        // build the empty stack and the stack package, declaring its settings
        const empty = await MemoryBuild.of(
            STACKS.space.package,
            [
                {
                    kind: "stack",
                    kindPackageId: stackPackage.id,
                    name: STACKS.space.export,
                    description: { installations: {} },
                },
            ],
            crypto,
        );
        const stack = await MemoryBuild.of(
            Package.of(stackPackage),
            [buildRetention].map((declared) => ({
                kind: "setting",
                kindPackageId: settingPackage.id,
                name: declared.name,
                description: describeSetting(declared),
            })),
            crypto,
        );

        // identify both by their manifests' digests
        const source = {
            ...STACKS.space,
            manifest: await PackageStore.digest(empty.manifest, crypto),
        };

        return {
            space: empty,
            stack,
            source,
            manifest: await PackageStore.digest(stack.manifest, crypto),
        };
    }

    /** Take the world the space joins, else open a universe and registry closed after the test. */
    static async #universe(
        options: TestSpaceOptions,
        defer: Defer,
        simulation: Simulation,
        storage?: TestDatabase,
        world?: UniverseWorld,
    ): Promise<TestSpaceWorld> {
        // join the given world
        if (options.world !== undefined) {
            return options.world;
        }

        // open the universe opening releases from the world's registry
        const environment = simulation.environment(UNIVERSE_NODE);
        const { source } = await TestSpace.releases(environment.crypto);
        const packages = await TestPackageStore.create(environment);
        defer(() => packages[Symbol.asyncDispose]());
        const registry = new TestRegistry(packages);
        const universe = await TestUniverse.open({
            stacks: { person: source, residency: source, account: source, space: source },
            releases: registry,
            ...(storage === undefined ? {} : { storage }),
            ...(world === undefined ? {} : { world }),
            policies: options.policies ?? [],
            ...(options.residencies === undefined ? {} : { residencies: options.residencies }),
            ...(options.publishers === undefined ? {} : { publishers: options.publishers }),
            simulation,
        });
        defer(() => universe[Symbol.asyncDispose]());
        const gateway = new TestGateway(simulation);
        defer(() => gateway[Symbol.asyncDispose]());

        return { simulation, universe, registry, gateway };
    }

    /** Host the fixture's space on a machine with its buckets, closed once its lifetime ends. */
    static async #hosting(
        world: TestSpaceWorld,
        owner: TestPerson,
        test: TestDatabase,
        hosted: { readonly id: Identifier<"space">; readonly account: Identifier<"account"> },
        options: TestSpaceOptions,
        stores: {
            /** The machine hosting the space. */
            readonly machine: TestMachine;
            /** The machine's files directory, with its buckets and the space's databases. */
            readonly files: {
                readonly directory: string;
                readonly buckets: LocalBucketHost;
                readonly builds: TestPackageStore;
                readonly blobs: LocalBlobStorage;
                readonly main: string;
            };
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
        },
    ): Promise<TestSpace> {
        // follow the directory as the hosting machine until it places the space
        const { machine, files, defer } = stores;
        const { simulation, universe, registry, gateway } = world;
        const { buckets, main } = files;
        await universe.settle();
        await machine.settle();
        await machine.directory.waitForPlacement(
            hosted.id,
            machine.clock,
            new AbortController().signal,
        );

        // store the space's builds in its blobs, read through to the machine's cache
        const blobs = new BlobStore({
            storage: files.blobs,
            database: test.database,
            environment: machine.environment,
        });
        const builds = new BuildStore(
            new PackageStore({ blobs, environment: machine.environment }),
            new BuildCache(files.builds, ["bun", "browser"]),
        );
        const servers = [...(options.servers ?? []), ...(options.companions ?? [])];
        const releases = await TestSpace.releases(machine.environment.crypto);
        const bundled = new Map<PackageId, PackageRelease>([
            [stackPackage.id, { package: Package.of(stackPackage), manifest: releases.manifest }],
        ]);

        // lend the machine to the space's host, answering where the test serves it or where it published
        const lent: SpaceMachine = {
            id: machine.id,
            environment: machine.environment,
            keyring: machine.keyring,
            origin:
                options.origin ??
                (await machine.directory.answering(machine.id, machine.clock.now())),
            directory: machine.directory,
            identities: machine.identities,
            universe: machine.universe,
            fetch: (audience) => machine.fetch(audience),
            send: (request) => simulation.network.of(machine.id).fetch(request),
            builds,
            servers,
            bundled,
        };

        // store the machine's own resources below its files, such as Git storage
        const local = options.resources?.(
            join(files.directory, RESOURCES),
            simulation.environment(UNIVERSE_NODE),
        );

        // start the space's identity on its host, its registry publishing the space's stack
        const host = new SpaceHost(lent, hosted.id, test.database);
        await host.open();
        const fixture = new TestSpace({
            ...hosted,
            universe,
            owner,
            machine,
            simulation,
            registry,
            gateway,
            host,
            test,
            buckets,
            main,
            blobs,
            builds,
            packages: options.packages ?? [],
            connectors: [...(options.connectors ?? []), ...(local?.connectors ?? [])],
            beside: {
                companions: options.companions ?? [],
                providers: [...(options.providers ?? []), ...(local?.providers ?? [])],
            },
            bundled,
            releases,
            defer,
        });
        defer(() => fixture[Symbol.asyncDispose]());
        for (const release of [releases.space, releases.stack, ...(options.releases ?? [])]) {
            await fixture.registry.release(release);
        }

        // ship the machine's builds of its servers' packages, fixed before the space serves
        for (const build of options.bundled ?? []) {
            await fixture.registry.release(build);
            const released = PackageManifest.package(build.manifest);
            const manifest = await fixture.registry.find(released.id, released.version);
            bundled.set(released.id, { package: released, manifest });
        }

        return fixture;
    }

    /** Open a machine's buckets and the space's own databases below a directory. */
    static async #openFiles(
        environment: Environment<"files">,
        defer: Defer,
        given?: string,
    ): Promise<{
        readonly directory: string;
        readonly buckets: LocalBucketHost;
        readonly builds: TestPackageStore;
        readonly blobs: LocalBlobStorage;
        readonly main: string;
    }> {
        // store the files below the given directory, else in a temporary one removed with them
        const { files } = environment;
        const directory = given ?? (await files.makeTemporaryDirectory("destack-stack-files-"));
        const main = join(directory, "main");
        await files.mkdir(main, { recursive: true });
        const buckets = new LocalBucketHost({
            directory: join(directory, "buckets"),
            endpoint: new URL("http://s3.localhost"),
            region: "local",
            environment,
            credentials: async () => BUCKET_CREDENTIALS,
        });
        const builds = await TestPackageStore.open(join(directory, "builds"), environment);
        const blobs = await LocalBlobStorage.open(join(directory, "blobs"), environment);
        defer(async () => {
            await buckets[Symbol.asyncDispose]();
            await builds[Symbol.asyncDispose]();
            if (given === undefined) {
                await files.remove(directory, { recursive: true });
            }
        });

        return { directory, buckets, builds, blobs, main };
    }

    /** Fill the stack package's options: the host's, then the machine's stores and runtimes. */
    async options(overrides: FixtureOptions = {}): Promise<SpaceOptions> {
        return {
            ...(await this.host.options()),
            handoff: async (source) => {
                throw new TypeError(`the fixture knows no handoff of ${source}`);
            },
            buildRunner: this.machine.id,
            main: spaceDatabase,
            sandboxes: [],
            blobs: this.blobs,
            registry: this.registry,
            buckets: await this.buckets(),
            builds: await this.builds(),
            runtimes: [new TestRuntime(), new ProcessRuntime()],
            events: overrides.events ?? (await this.#events()),
            sockets: overrides.sockets ?? (await this.sockets()),
            isEnforcingEntitlements: false,
            isServingHostnames: true,
            forge: null,
            follows: [],
            controllers: [],
            external: await this.index(),
            ...overrides,
            servers: await this.#placed(overrides.servers ?? []),
            providers: [
                ...(overrides.providers ?? []),
                TestSpaceDatabaseHost.over(this.#main, this.environment),
            ],
        };
    }

    /** Place servers without an installation at their base installations in the space. */
    #placed(servers: readonly PackageImplementation[]): Promise<PackageImplementation[]> {
        const scope = space.reference(this.account, this.id);

        return Promise.all(
            servers.map(async (server) =>
                server.installation === undefined
                    ? server.at(
                          await Installation.base(scope, server.package, this.environment.crypto),
                      )
                    : server,
            ),
        );
    }

    /** Control a space as its machine does, serving its installed revisions at each change. */
    controller(store: ObjectStore, spaceId: Identifier<"space">): ReconcileController {
        const watches = [object.rollout.table, object.binding.table, object.database.table];

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
        store: Pick<ObjectStore, "database" | "mutate" | "principal" | "clock" | "environment">,
        spaceId: Identifier<"space">,
    ): Promise<void> {
        // read the revisions main applied, as the machine's start serves them
        const builds = await this.builds();
        const installed = await Database.installed(
            Snapshot.live(this.database),
            spaceId,
            (packageId, manifest) => builds.pull(this.registry, packageId, manifest),
        );
        const revisions = new Map(installed.map((entry) => [entry.rollout.id, entry.revisionId]));

        // record what the space serves
        await Rollout.serve(store, spaceId, revisions);
    }

    /** Read the distinct refusals a settled apply reports, none when it applies. */
    static async refusals(
        applying: Promise<unknown>,
    ): Promise<readonly (readonly [string, string, string])[]> {
        // take an apply that settles, and rethrow any failure but the servers' reports
        const errors = await applying.then(
            (): unknown[] => [],
            (error: unknown) => {
                if (!(error instanceof AggregateError)) {
                    throw error;
                }

                const reported: unknown[] = error.errors;

                return reported;
            },
        );

        // read each reported failure's cause once, as the stack retries a refused apply
        const refusals = new Map<string, readonly [string, string, string]>();
        for (const each of errors) {
            const { code, reason, message } = Failure.of(each instanceof Error ? each.cause : each);
            refusals.set(`${code} ${reason} ${message}`, [code, reason, message]);
        }

        return [...refusals.values()];
    }

    /** Send an in-store installation's call to a peer installation as it, as a dispatcher does. */
    async #egress(caller: EgressCaller, request: Request): Promise<Response> {
        // require a peer installation's address and a package mount below it
        const egressed = Egress.of(request);
        const peer = egressed === undefined ? undefined : Egress.installation(egressed.address);
        if (egressed === undefined || peer === undefined) {
            throw new TypeError(
                `the test space's egress sends to peer installations alone, not ${request.url}`,
            );
        }
        const below = new URL(egressed.request.url);
        const mount = PackageMount.parse(below.pathname);

        // require a peer path the caller may call
        if (mount === undefined || !EgressCaller.binds(caller, peer, mount.path)) {
            throw new TypeError(
                `installation ${caller.installationId} does not bind ${peer.name} of ${peer.spaceId} at ${below.pathname}`,
            );
        }
        // refuse an exchange for the call's subject, which the test space's egress makes for nobody
        else if (Exchange.asking(egressed.request) !== undefined) {
            throw new TypeError("the test space's egress exchanges no call for its subject");
        }

        // drop the caller's secret and identity claims, as a dispatcher does
        const headers = new Headers(egressed.request.headers);
        for (const header of ["authorization", CALLER_HEADER, "cookie"]) {
            headers.delete(header);
        }

        // send it signed as the caller to the machine serving the peer
        const located = await this.host.machine.directory.locate(
            peer.spaceId,
            this.host.machine.environment.clock.now(),
        );
        const mounted = InstallationMount.url(located.url, peer.spaceId, peer.name);
        const signer = principal.installation.reference(caller.scope, caller.installationId);
        const send = this.host.source(signer).fetchSigned(stackPackage.id);

        return send(
            copyRequest(
                egressed.request,
                { headers },
                `${mounted}${below.pathname}${below.search}`,
            ),
        );
    }

    /** Settle the universe, its machine and the space to a fixpoint, their replicas caught up. */
    settle(served: TestObjectStore): Promise<void> {
        return this.simulation.settle(this.#settle(served));
    }

    /** Settle in rounds until the universe and the space commit nothing, then wait for liveness. */
    async #settle(served: TestObjectStore): Promise<void> {
        // settle the universe, the machine following it and the space until a round commits nothing
        const positions = async () =>
            JSON.stringify([
                await this.universe.storage.database.log.position(),
                await this.database.log.position(),
            ]);
        let before = await positions();
        let isSettled = false;
        for (let round = 0; round < SETTLE_ROUNDS && !isSettled; round += 1) {
            // settle the universe and the machine, and let the space's replicas catch up to the universe
            await this.universe.settle();
            await this.machine.settle();
            await TestObjectStore.settleReplicas([
                this.universe.universe.store,
                this.machine.store,
                served.store,
            ]);

            // settle the space, and stop once a round commits nothing
            await served.settle();
            const after = await positions();
            isSettled = after === before;
            before = after;
        }

        // refuse a world still committing after every round
        if (!isSettled) {
            throw new Error(`the space's world kept committing after ${SETTLE_ROUNDS} rounds`);
        }

        // wait until each replica of the space's chain is live
        await served.store.untilLive(new AbortController().signal);
        await served.settle();
    }

    /** Run the space's world to a time through every timer on the way, then settle what is due. */
    async advance(served: TestObjectStore, to: number): Promise<void> {
        await this.simulation.run({ to });
        await this.settle(served);
    }

    /** Jump the space's world to a time, waking each due wait once, then settle what is due. */
    async jump(served: TestObjectStore, to: number): Promise<void> {
        await this.simulation.jump(to);
        await this.settle(served);
    }

    /** Suspend or resume the space at the universe as its owner, settled as its machine follows. */
    change(served: TestObjectStore, action: "suspend" | "resume"): Promise<void> {
        return this.simulation.settle(this.#change(served, action));
    }

    /** Suspend or resume the space at the universe as its owner, settled as its machine follows. */
    async #change(served: TestObjectStore, action: "suspend" | "resume"): Promise<void> {
        // call the universe as the owner
        await this.owner.client(accountPackage).space[action]({
            accountId: this.account,
            id: this.id,
            requestId: RequestId.create(this.machine.environment),
        });

        // wait until the space's row arrives at the machine, then settle the space mirroring it
        await this.universe.settle();
        await this.database.log.until(async () => {
            const [row] = await Snapshot.live(this.database)
                .read(space.table)
                .findMany({ where: { id: this.id } });

            return (row?.suspendedAt !== null) === (action === "suspend");
        }, new AbortController().signal);
        await this.settle(served);
    }

    /** The machine's buckets. */
    async buckets(): Promise<LocalBucketHost> {
        return this.#buckets;
    }

    /** The machine's build store. */
    async builds(): Promise<BuildStore> {
        return this.#builds;
    }

    /** Index the space's builds, each index attached to its build, until the test ends. */
    index(): Promise<BunGraphStorage> {
        this.#index ??= (async () => {
            const index = new BunGraphStorage(
                this.database,
                await this.builds(),
                this.environment.files,
            );
            this.#defer(() => index[Symbol.asyncDispose]());

            return index;
        })();

        return this.#index;
    }

    /** Push a running build's files and code index as its runner, answering the files uploaded. */
    async storeBuild(
        runner: TestCaller,
        target: { readonly spaceId: Identifier<"space">; readonly id: Identifier<"build"> },
        reader: BuildReader,
    ): Promise<readonly Digest[]> {
        // report storing, then push the build to the space's builds adapter, recording each file uploaded
        const client = runner.client(stackPackage);
        await client.build.progress({
            ...target,
            requestId: RequestId.create(this.machine.environment),
            progress: { phase: "storing" },
        });
        const { fetch } = runner.endpoint(stackPackage);
        const uploaded: Digest[] = [];
        const pushing = new PackageClient(
            `${SPACE_URL}${PUSH_PATH}/${target.spaceId}/${target.id}/`,
            async (request) => {
                const file = /\/files\/([0-9a-f]{64})$/u.exec(new URL(request.url).pathname);
                if (request.method === "PUT" && file !== null) {
                    uploaded.push(Digest.parse(nth(file, 1)));
                }

                return await fetch(request);
            },
            this.machine.environment.crypto,
        );
        await pushing.push(reader);

        // upload an empty code index with it, then finish with the manifest
        const bytes = new TextEncoder().encode(JSON.stringify([]));
        const code = {
            digest: await Digest.of(bytes, this.machine.environment.crypto),
            size: bytes.byteLength,
        };
        await pushing.upload(code.digest, async () => bytes);
        await client.build.finish({
            ...target,
            requestId: RequestId.create(this.machine.environment),
            end: { kind: "stored", manifest: reader.manifest, code },
        });

        return uploaded;
    }

    /** Describe a build's revision of an installation as its submission does. */
    async describeRevision(
        target: Pick<Installation, "name" | "capabilities" | "allowed">,
        reader: BuildReader,
    ): Promise<Pick<object.InstallationRevision, "dependencies" | "states">> {
        const { store } = serveSpace(await this.options());
        const options = { builds: await this.builds(), registry: this.registry, blobs: this.blobs };

        return InstallationRevision.describe(target, reader, store.objects, options);
    }

    /** Put a build with its code index in the build store, and record it succeeded now. */
    async recordBuild(
        reader: BuildReader,
        index: object.CodeIndex = { code: [], modules: [] },
    ): Promise<{ readonly kind: "build"; readonly build: Identifier<"build"> }> {
        // put the build in the build store, with its code index attached as its runner's finish does
        const { database } = this;
        const { space: packages } = await this.builds();
        const manifest = await packages.put(reader);
        const bytes = new TextEncoder().encode(JSON.stringify(index));
        const indexed = {
            digest: await Digest.of(bytes, this.machine.environment.crypto),
            size: bytes.byteLength,
        };
        await packages.receive(indexed.digest, bytes);
        await packages.attach(manifest, object.CodeIndex.file(indexed));

        // record the build succeeded now
        const id = Identifier.create("build", this.machine.environment);
        const now = this.machine.clock.now();
        await database.transaction(
            async (transaction) =>
                await transaction.insert(object.build.table).values({
                    id,
                    scope: this.id,
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
                }),
        );

        // insert the build's own scope inside its space's, as its creation does
        await insertBuildScope(database, this.id, id);

        return { kind: "build", build: id };
    }

    /** Resolve each declared installation to its release's manifest in the registry. */
    async resolve(definition: JsonObject): Promise<JsonObject> {
        const { installations } = StackDefinition.collections(definition);
        const pinned = await Promise.all(
            Object.entries(installations).map(async ([key, installed]) => {
                // pin an installed release to its manifest
                const { packageId, selection } = installed;
                if (selection.kind !== "release") {
                    return [key, installed] as const;
                }
                const manifest = await this.registry.find(packageId, selection.version);

                return [key, { ...installed, selection: { ...selection, manifest } }] as const;
            }),
        );

        return { ...definition, installations: Object.fromEntries(pinned) };
    }

    /** Put a package's fixture release in the build store, as the space's build of it. */
    async release(
        packageId: PackageId,
        version: string,
    ): Promise<{ readonly id: Identifier<"build">; readonly manifest: string }> {
        // replicate the registry's release into the build store, and record it as the space's build
        const manifest = await this.registry.find(packageId, version);
        const reader = await (await this.builds()).pull(this.registry, packageId, manifest);
        const release = { package: packageId, version, manifest };

        return insertRelease(
            this.database,
            this.id,
            release,
            Object.keys(reader.manifest.outputs),
            this.machine.environment,
        );
    }

    /** Record the space's calls and its packages' events, under its retention. */
    #events(): Promise<EventStore> {
        // list the event types of the served packages in addition to the calls
        const kinds = this.#objects().flatMap((type) =>
            type.storage === "events" ? [type.eventKind()] : [],
        );

        return testEvents(
            this.database,
            this.machine.environment,
            kinds,
            SettingValue.retention(this.database),
        );
    }

    /** List the object types of the packages the space serves in addition to the stack's. */
    #objects(): ObjectType[] {
        return this.#packages
            .flatMap((handle) => Object.values(handle.objects))
            .filter((type): type is ObjectType => type instanceof ObjectType);
    }

    /** Open the machine's live sockets in memory until the test ends. */
    async sockets(): Promise<SocketRegistry> {
        // collect the store's failures
        const reported: unknown[] = [];
        const sockets = await SocketRegistry.open({
            objects: this.#objects(),
            environment: this.machine.environment,
            tracker: this.database.tracker,
            memory: (tables) =>
                connectBunSqlite(":memory:", tables, { environment: this.machine.environment }),
            report: (error) => reported.push(error),
        });

        // close the store and require no failure once the test ends
        this.#defer(async () => {
            await sockets[Symbol.asyncDispose]();
            expect(reported).toEqual([]);
        });

        return sockets;
    }

    /** Serve the space as its machine does, hosting the installed revisions of shipped packages. */
    serve(
        options: FixtureOptions & {
            /** Whether the store runs its controllers, true by default. */
            readonly isControlled?: boolean;
        } = {},
    ): Promise<TestObjectStore> {
        return this.simulation.settle(this.#serve(options));
    }

    /** Serve the space as its machine does, hosting the installed revisions of shipped packages. */
    async #serve(
        options: FixtureOptions & {
            /** Whether the store runs its controllers, true by default. */
            readonly isControlled?: boolean;
        } = {},
    ): Promise<TestObjectStore> {
        // pause the fixture's own follow, which the served store takes over
        await this.#following?.();
        this.#following = undefined;

        // host a primary's revisions main applied, calling the space's own packages in its served store
        const { isControlled = true, ...given } = options;
        const providers = [...this.#beside.providers, ...(given.providers ?? [])];
        const serving = Promise.withResolvers<TestObjectStore>();
        const stopping = new AbortController();
        const role = given.role ?? (await this.host.options()).role;
        const installed =
            role === "primary"
                ? await Database.installed(
                      Snapshot.live(this.database),
                      this.id,
                      (packageId, manifest) =>
                          this.#builds.pull(this.registry, packageId, manifest),
                  )
                : [];
        const hosted = await this.host.servers(
            installed,
            {
                call: async (packageId, request, authentication) => {
                    const served = await serving.promise;
                    const handler = present(
                        served.store.packages.find((each) => each.package.id === packageId),
                        `the served package ${packageId}`,
                    );

                    return served.server(handler.package).fetch(request, authentication);
                },
                store: serving.promise.then((served) => served.store),
                providers,
                connectors: this.#connectors,
                buckets: this.#buckets,
                egress: (caller, request) => this.#egress(caller, request),
                sandbox: (row, server, declarations) =>
                    new HookSandbox({
                        server,
                        declarations,
                        send: async () => {
                            throw new TypeError(
                                `the test space runs no hook of ${row.installation.name}`,
                            );
                        },
                    }),
            },
            stopping.signal,
        );

        // serve the space's companions and the test's own packages at their base installations in the space
        const own = await this.#placed([...this.#beside.companions, ...(given.servers ?? [])]);

        // serve as the machine's start does, once the space's replicas are ready and its birth settled
        const { store, packages } = serveSpace(
            await this.options({
                ...given,
                servers: [...hosted.servers, ...own],
                providers: providers.filter((each): each is SpaceProvider => !("connect" in each)),
                sandboxes: hosted.sandboxes,
            }),
        );

        // receive the space's and the test's own base installations on each start, with roles from the types they serve
        const born = [
            ...SPACE_KERNEL,
            ...own
                .filter(
                    (server) =>
                        server.installation !== undefined &&
                        !ServerContext.isInstalled(server.installation),
                )
                .map((server) => server.package),
        ];
        await Transaction.run(store.database, async (transaction) => {
            // leave a space this machine has no row of, such as one arriving by a move
            const [here] = await Scope.read(transaction.view(), [this.id]);
            if (here === undefined) {
                return;
            }

            // receive it again, born already, with the roles its bases have now
            const authorization = new Authorization(
                store.authorizer,
                transaction,
                () => AccessContext.of(Caller.of(store.principal), store.clock.now()),
                store.environment,
            );
            await Installation.receive(
                authorization,
                space.reference(this.account, this.id),
                [this.account],
                born,
            );
        });

        // serve the store as its machine does, in the partitions the space serves
        const served = TestObjectStore.serve(store, {
            scopes: await this.host.partitions(),
            packages: packages.map((handler) =>
                isControlled ? handler : { ...handler, controllers: [] },
            ),
            credential: this.#credential(store),
        });
        serving.resolve(served);
        const unserve = this.gateway.serve(this, served);
        this.#defer(async () => {
            // stop every served space's controllers in the world first, then stop serving this one
            await this.gateway.stopControllers();
            unserve();
            stopping.abort();
            await served[Symbol.asyncDispose]();
        });
        await served.ready();
        await this.universe.settle();
        await served.settle();

        return served;
    }

    /** Keep the space's replicas followed while no served store follows them. */
    async #follow(): Promise<void> {
        // follow only a space placed on the fixture's machine, as its machine serves only those
        const placed = await Snapshot.live(this.database)
            .read(placement.table)
            .findFirst({ where: { id: this.id } });
        if (placed?.machineId !== this.machine.id) {
            return;
        }

        // run only the replica follow of a store over the space's database
        const { store } = serveSpace(await this.options());
        const loop = new ControlLoop(
            this.database,
            store.controllers().filter((controller) => controller.name === "replica"),
            {
                clock: store.clock,
                report: (_controller, _key, error) => {
                    throw error;
                },
            },
        );

        // stop following once a store serves the space or the fixture ends
        const stopping = new AbortController();
        const running = loop.run(stopping.signal);
        const stop = async () => {
            stopping.abort();
            await running;
        };
        this.#following = stop;
        this.#defer(stop);
        await this.simulation.settle(store.untilLive(stopping.signal));
    }

    /** Call a served space as a person, carrying the token their client exchanges for it. */
    person(
        served: TestObjectStore,
        person: TestPerson,
        partition: Identifier<"space"> | Identifier<"account"> = this.id,
    ): TestCaller {
        // sign each request with the person's token for the called package, bound to the space or the account it is the home of
        const signing: TestServed = {
            scope: served.scope,
            environment: served.environment,
            server: (handle) => served.server(handle),
            type: (reference) => served.type(reference),
            named: (name) => served.named(name),
            sign: async (request, signer) => {
                // exchange the person's token for the package, and send it as the bearer
                const { accessToken } = await person.exchange(signer.audience, { partition });
                const headers = new Headers(request.headers);
                headers.set("authorization", `${BEARER_PREFIX}${accessToken}`);

                return new Request(request, { headers });
            },
        };

        return new TestCaller(signing, person.subject);
    }

    /** Authenticate a served space's callers: people by their tokens, test callers by bearer. */
    #credential(store: ObjectStore): TestCredential {
        // verify a universe token for its package and this space, against the keys the machine's directory has
        const { environment } = this.machine;
        const keys: KeyLookup = async (header) => {
            const { snapshot } = this.machine.directory;
            const set = await Identity.keys(snapshot, Scope.universe.id, environment.crypto);

            return Jwks.local(set, environment.crypto)(header);
        };
        const verify = (audience: PackageId, token: string) =>
            new TokenVerifier({
                authority: { kind: "universe" },
                issuer: this.machine.universe.issuer,
                audience,
                keys,
                environment,
            }).verify(token, store.clock.now());

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
            authenticate: async (request, audience) => {
                // serve a request without a bearer as no one
                const header = request.headers.get("authorization");
                if (header === null) {
                    return undefined;
                }

                // verify a person's token, else authenticate a test caller by its bearer's subject
                const bearer = header.slice(BEARER_PREFIX.length);
                if (JWT_FORM.test(bearer)) {
                    const verified = await verify(audience, bearer);
                    await this.host.requirePartition(verified);

                    return verified;
                }
                const now = store.clock.now();
                const subject = TestBearer.subject(request);
                const assurance = TestBearer.assurance(request, now);
                const tested = TestBearer.authenticate(subject, audience, now, assurance);

                return principal.user.is(subject)
                    ? {
                          caller: tested.caller,
                          credential: { ...tested.credential, kind: "session", scope: this.id },
                      }
                    : tested;
            },
        };
    }

    /** Apply the space's stack with some collections replaced, approved as its owner. */
    async apply<const Types extends StackTypes>(
        served: TestObjectStore,
        types: Types,
        collections: StackDocument<Types>,
    ): Promise<void> {
        // fail with the store's reported failures, else unless the controllers applied this submission
        const { ready, generation, failures } = await this.simulation.settle(
            this.#submit(served, types, collections),
        );
        if (failures.length > 0) {
            throw new AggregateError(failures, "the space's controllers failed");
        } else if (ready?.observedGeneration !== generation) {
            throw new Error(
                `the space's stack is at generation ${generation}, its controllers observed ${ready?.observedGeneration}`,
            );
        } else if (ready?.reason !== "Applied") {
            throw new Error(`the space's stack is ${ready?.reason}: ${ready?.message}`);
        }
    }

    /** Submit the space's stack with some collections replaced, answering its ready condition. */
    async submit<const Types extends StackTypes>(
        served: TestObjectStore,
        types: Types,
        collections: StackDocument<Types>,
    ): Promise<readonly [string | undefined, string | undefined]> {
        // require each failure the store reports to be the failed apply the stack retries
        const { ready, failures } = await this.simulation.settle(
            this.#submit(served, types, collections),
        );
        const retried = failures.every(
            (failure) =>
                ready?.reason === "ApplyFailed" &&
                failure instanceof Error &&
                Failure.of(failure.cause).message === ready.message,
        );
        if (!retried) {
            throw new AggregateError(failures, "the space's controllers failed");
        }

        return [ready?.reason, ready?.message];
    }

    /** Submit the space's stack as its owner, approving a waiting plan, with the failures. */
    async #submit<const Types extends StackTypes>(
        served: TestObjectStore,
        types: Types,
        collections: StackDocument<Types>,
    ) {
        // replace the collections given, once the space has its stack
        this.#stack = { ...this.#stack, ...StackDocument.of(types, collections) };
        await served.settle();
        const owner = served.as(this.owner.subject).client(stackPackage);
        const installed = served.as(this.owner.subject).client(objectPackage);
        const listed = await installed.installation.list({ scope: this.id });
        const stack = single(listed.items.filter((entry) => entry.export !== null));

        // submit the stack's build evaluated to the definition
        const definition = await this.resolve(this.#stack);
        await owner.rollout.submit({
            spaceId: this.id,
            id: stack.id,
            selection: stack.selection,
            build: await this.recordBuild(this.releases.space),
            evaluation: { export: STACKS.space.export, parameters: {}, definition },
            requestId: RequestId.create(this.machine.environment),
        });

        // approve a plan waiting for its approval as the owner
        let failures = await TestSpace.#failures(served.settle());
        const submitted = await installed.installation.get({ scope: this.id, id: stack.id });
        if (submitted.conditions["Ready"]?.reason === "AwaitingApproval") {
            const waiting = await owner.rollout.plan({ spaceId: this.id, id: stack.id });
            await owner.rollout.approve({
                spaceId: this.id,
                id: stack.id,
                requestId: RequestId.create(this.machine.environment),
                plan: await Plan.digest(waiting, this.machine.environment.crypto),
            });
            failures = await TestSpace.#failures(served.settle());
        }

        // read how the controllers left it
        const applied = await installed.installation.get({ scope: this.id, id: stack.id });

        return { ready: applied.conditions["Ready"], generation: applied.generation, failures };
    }

    /** Read the failures a settle reports, none when it settles cleanly. */
    static async #failures(settling: Promise<void>): Promise<readonly unknown[]> {
        return await settling.then(
            (): unknown[] => [],
            (error: unknown) => {
                if (!(error instanceof AggregateError)) {
                    throw error;
                }
                const reported: unknown[] = error.errors;

                return reported;
            },
        );
    }

    /** Install a build through the space's stack as its owner, served once applied. */
    install(
        served: TestObjectStore,
        build: BuildReader,
        options: {
            /** The installation's bindings, none by default. */
            readonly bindings?: schema.Input<typeof InstallationDefinition>["bindings"];
            /** The capabilities the installation is given, none by default. */
            readonly capabilities?: schema.Input<typeof InstallationDefinition>["capabilities"];
        } = {},
    ): Promise<Identifier<"installation">> {
        return this.simulation.settle(this.#install(served, build, options));
    }

    /** Install a build through the space's stack as its owner, served once applied. */
    async #install(
        served: TestObjectStore,
        build: BuildReader,
        options: {
            /** The installation's bindings, none by default. */
            readonly bindings?: schema.Input<typeof InstallationDefinition>["bindings"];
            /** The capabilities the installation is given, none by default. */
            readonly capabilities?: schema.Input<typeof InstallationDefinition>["capabilities"];
        } = {},
    ): Promise<Identifier<"installation">> {
        // publish the build as its package's release
        await this.registry.release(build);
        const released = PackageManifest.package(build.manifest);

        // declare it after the earlier installations
        const name = released.name.slice(released.name.lastIndexOf("/") + 1);
        const declared = {
            packageId: released.id,
            selection: { kind: "release" as const, version: released.version },
            status: "enabled" as const,
            name,
            bindings: options.bindings ?? {},
            compute: {},
            capabilities: options.capabilities ?? [],
            directories: [],
            tags: {},
        };
        const { installations } = StackDefinition.collections(this.#stack);
        await this.apply(served, [], { installations: { ...installations, [name]: declared } });

        // serve the revision the stack applied, as the machine does on the change
        await this.serveInstalled(served.store, this.id);
        await served.settle();

        // read the installation the stack applied, refusing one its rollout refuses to deploy
        const listed = await served
            .as(this.owner.subject)
            .client(objectPackage)
            .installation.list({ scope: this.id, where: { name } });
        const { id } = single(listed.items);
        const [deployed] = await this.database
            .select({ conditions: installation.table.conditions })
            .from(installation.table)
            .where(eq(installation.table.id, id));
        const ready = present(deployed, `the installation ${name}`).conditions["Ready"];
        const isRefused = REFUSAL_REASONS.some((reason) => reason === ready?.reason);
        if (ready?.status === "false" && isRefused) {
            throw new Error(`the installation ${name} is ${ready.reason}: ${ready.message}`);
        }

        return id;
    }

    /** Connect to a served space's handoff as a caller on this space's machine. */
    handoff(served: TestObjectStore, caller: TestCaller): Handoff {
        // send each request signed as the caller to the served stack package
        const { fetch } = caller.endpoint(stackPackage);

        return Handoff.connect(SPACE_URL, (request) => fetch(request), this.machine.environment);
    }

    /** Stop the fixture's own follow, then close the machine's database once. */
    async [Symbol.asyncDispose](): Promise<void> {
        // stop the fixture's own follow, then close the database once
        await this.#following?.();
        this.#following = undefined;
        this.#closed ??= this.#test.close();
        await this.#closed;
    }
}

/** The world spaces share: the simulation, the universe, the registry and the machines' gateway. */
export type TestSpaceWorld = Pick<TestSpace, "simulation" | "universe" | "registry" | "gateway">;

/** How a fixture opens a machine's database. */
export interface TestSpaceOptions {
    /** The packages the space serves in addition to the stack's, whose tables its database adds. */
    readonly packages?: readonly Pick<PackageHandle, "objects" | "tables">[];
    /** The shapes the space follows in addition to its own, whose replicas its database adds. */
    readonly follows?: readonly Follow[];
    /** The host's tables in addition to the space's. */
    readonly tables?: readonly Table[];
    /** The world the space joins: its simulation, universe and registry, a new one by default. */
    readonly world?: TestSpaceWorld;
    /** The policies a new universe decides with its own, such as the forge's, none by default. */
    readonly policies?: TestUniverseOptions["policies"];
    /** The residencies a new universe declares, Europe alone by default. */
    readonly residencies?: TestUniverseOptions["residencies"];
    /** The handles a new universe's platform publishes under, none by default. */
    readonly publishers?: TestUniverseOptions["publishers"];
    /** The owner of the space, a new sign-up by default, such as a move's source's owner. */
    readonly owner?: TestPerson;
    /** The origin the test serves the hosting machine at, its published one by default. */
    readonly origin?: string;
    /** The machine hosting the space, the residency's cloud machine by default. */
    readonly machine?: TestMachine;
    /** The servers the machine ships, whose packages' installations run in process. */
    readonly servers?: readonly PackageImplementation[];
    /** The builds the machine ships of its servers' packages, fixed before the space serves. */
    readonly bundled?: readonly BuildReader[];
    /** The servers every serve runs with the packages, such as a vault, none by default. */
    readonly companions?: readonly PackageImplementation[];
    /** The providers every serve offers with its database host, served ones in process. */
    readonly providers?: readonly (SpaceProvider | StoreProvider)[];
    /** The moment a template's world starts at, which a restoring simulation's clock reads. */
    readonly start?: number;
    /** The machine's connectors of bound resources such as mail, none by default. */
    readonly connectors?: readonly Connector[];
    /** The machine's resources below its files, such as Git storage, opened per hosting. */
    readonly resources?: (
        directory: string,
        environment: Environment<HostCapability>,
    ) => {
        /** The providers placing the resources below the directory. */
        readonly providers: readonly (SpaceProvider | StoreProvider)[];
        /** The connectors opening them there. */
        readonly connectors: readonly Connector[];
    };
    /** The existing account a space opens in, the fixture's account created anew by default. */
    readonly account?: Identifier<"account">;
    /** The space's identifier, the fixture's by default. */
    readonly id?: Identifier<"space">;
    /** The space's name in its account, `space` by default. */
    readonly name?: string;
    /** The stack the space is born with, the universe's default stack by default. */
    readonly stack?: StackSource;
    /** The builds the registry publishes before the space's birth, such as its stack's. */
    readonly releases?: readonly BuildReader[];
}

/** Reconcile one key of a served object type's controller once. */
export async function reconcile(
    store: Pick<ObjectStore, "controllers" | "database" | "environment">,
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
        "controllers" | "database" | "blobs" | "objects" | "as" | "mutate" | "clock" | "environment"
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
            // leave following controllers, which keys never select
            if (controller.mode === "follow") {
                continue;
            }
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

    // refuse controllers that still write
    throw new TypeError(`the controllers kept writing after ${SETTLE_ROUNDS} rounds`);
}

/** Find a served converging controller by name, refusing one the fixture does not serve. */
function controllerOf(
    store: Pick<ObjectStore, "controllers" | "environment">,
    name: string,
): ReconcileController {
    const controller = store.controllers().find((each) => each.name === name);
    if (controller === undefined || controller.mode === "follow") {
        throw new TypeError(`the fixture serves no converging controller ${name}`);
    }

    return controller;
}

/** The options a test gives the fixture's space. */
export type FixtureOptions = Partial<SpaceOptions>;
