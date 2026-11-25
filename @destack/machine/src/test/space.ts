import { KNOWN_TYPES } from "@destack/stack/stack";
import {
    Installation,
    installation,
    type StoreProvider,
    type SpaceProvider,
} from "@destack/stack/object";
import { SpaceStorage } from "../space/storage.ts";
import * as accountObject from "@destack/account/object";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import { pathToFileURL } from "node:url";
import {
    Crypto,
    WebSocketUpgrade,
    type ClientSocket,
    type Environment,
    type HostCapability,
    type Scheduler,
} from "@destack/environment";
import { TestTelemetry } from "@destack/telemetry/test";
import { Refusal } from "./refusal.ts";
import {
    InstallationDefinition,
    STACK_KIND,
    StackDefinition,
    StackDocument,
    type StackRecordType,
} from "@destack/stack/declare";
import type { Connector, Plan } from "@destack/resource";
import { join } from "node:path";
import { LocalBucketHost } from "@destack/bucket/bun";
import { defineDatabase, eq, hashName, type Table, TABLE } from "@destack/db";
import {
    type ControllerSet,
    ControlLoop,
    type DatabaseConnection,
    databaseProvider,
    DatabaseHandle,
} from "@destack/db/database";
import {
    type BuildReader,
    BuildImage,
    BuildLayer,
    PackageManifest,
} from "@destack/package/manifest";
import { ObjectTypeReference, RequestId, type Subject, ObjectReference } from "@destack/sync";
import { TestBearer, TestCaller, type TestSigner, type TestScope } from "@destack/object/test";
import { TestTemplate, TestDatabase, TestRun, type TemplateStorage } from "@destack/db/test";
import { onTestFinished, single, expect } from "@destack/test";
import { Failure } from "@destack/error";
import { AccessContext, type Authentication, Caller, principal } from "@destack/access";
import { Account, Residency, type ResidencyCode, type StackRelease } from "@destack/account/object";
import { OPERATOR_EMAIL, RESIDENCY_STACK, STACKS, type TestPerson } from "@destack/account/test";
import accountPackage from "@destack/account/package";
import {
    AWAITING_APPROVAL,
    type Follow,
    ObjectType,
    type Package,
    Observation,
    PackageMount,
    SpaceMount,
    SYNCED,
    type Fetch,
} from "@destack/object";
import { ObjectStore, type PackageImplementation } from "@destack/object/server";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { graph, PackageVersion, PackageId, type PackageRelease } from "@destack/package";
import settingPackage from "@destack/setting/package";
import { describeSetting } from "@destack/setting/inspect";
import { buildRetention } from "@destack/stack/setting";
import {
    Digest,
    Identifier,
    type JsonObject,
    type VersionRange,
    type JsonValue,
    canonicalize,
    nth,
    present,
    Commit,
    schema,
} from "@destack/schema";
import objectDefinition from "@destack/object/destack.json" with { type: "json" };
import { Simulation } from "@destack/environment/simulation";
import { Handoff } from "../transfer/handoff.ts";
import * as object from "@destack/stack/object";
import { SocketRegistry, type InstallationBuild } from "../index.ts";
import { Machine } from "../machine/machine.ts";
import { Space } from "../space/space.ts";
import { Universe } from "@destack/universe";
import {
    MachineWorld,
    TEST_UNIVERSE,
    TestMachine,
    TestUniverse,
    type TestUniverseOptions,
    UniverseWorld,
    UNIVERSE_NODE,
} from "@destack/universe/test";
import type { ClientOf } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import stackPackage, { PUSH_PATH } from "@destack/stack/package";
import {
    fixtureBuild,
    FIXTURE_VERSIONS,
    fixtureOutputs,
    imageBuild,
    imageOf,
    TAGS,
} from "./build.ts";
import { FORGE_HOME_OPTIONS, FORGE_HOME_STACK, FORGE_HOME_TYPES } from "./home.ts";
import { callRegistry } from "../machine/registry.ts";
import { LATEST_TAG, reference } from "@destack/registry/object";
import registryPackage from "@destack/registry/package";
import { TestSpaceStorage } from "./storage.ts";
import { TestGateway } from "./gateway.ts";
import { TestRuntime } from "./runtime.ts";
import { ProcessRuntime, type Runtime } from "../runtime/index.ts";
import { insertBuildScope, insertRelease } from "./install.ts";
import { BunGraphStorage } from "../computer/graph.ts";
import { MemoryBuild } from "@destack/package/test";
import { RemoteBuildStore, BuildStore } from "@destack/build/store";
import { fixtureGraph, TestBuildStore, testStore } from "@destack/build/test";
import { TestDistribution } from "../computer/test/distribution.ts";
import { InstallationTarget } from "../installation/target.ts";
import { testWorldOptions } from "@destack/test/world";

/** The Destack version the fixture machine runs: this checkout's. */
const DESTACK_VERSION = import.meta.destack.package.version;

/** The main database of a space a test machine serves without other modules. */
export const spaceDatabase = SpaceStorage.main();

/** Run a cleanup once a fixture's test or template write ends. */
type Defer = (cleanup: () => Promise<void> | void) => void;

/** The releases a test machine links with the fixture's space, built in its world. */
export interface StackReleases {
    /** The fixture space's stack: the empty stack of a space without apps. */
    readonly space: BuildReader;
    /** The stack package's release, declaring its settings. */
    readonly stack: BuildReader;
    /** The stack of the fixture's space: the empty stack's release. */
    readonly source: StackRelease;
    /** The digest of the empty stack's release's image. */
    readonly sourceImage: Digest;
    /** The digest of the stack package's release's image. */
    readonly image: Digest;
    /** The object package's release, with its real definition, its kinds' code in process. */
    readonly object: BuildReader;
    /** The digest of the object package's release's image. */
    readonly objectImage: Digest;
    /** The empty stack every residency's home runs, which machines ship. */
    readonly residency: BuildReader;
    /** The digest of the residency stack's release's image. */
    readonly residencyImage: Digest;
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
    "layer",
] as const satisfies readonly (keyof TestSpaceOptions)[];

/** Describe a space world's state outside its databases: keys, files, stack and its own world. */
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
                /** The space's identifier. */
                id: schema.identifier("space"),
                /** The account the space belongs to. */
                account: schema.identifier("account"),
                /** The email of the space's owner, the operator for a residency's home. */
                owner: schema.string(),
                /** The stack the space declares, as the write last applied it. */
                stack: schema.jsonObject(),
                /** The linked release of each server's package the write installed, by package. */
                linked: schema.array(schema.object({ packageId: PackageId, image: Digest })),
            }),
        ),
        /** What the fixture's write returned. */
        world,
    });
}

/** A space world's state in addition to its databases, with a fixture's world. */
type SpaceWorld<World extends JsonValue> = schema.Infer<ReturnType<typeof spaceWorldOf<World>>>;

/** The born space alone, without a fixture's world. */
export const BORN_SPACE: SpaceWorldWrite<null> = {
    name: "space",
    value: schema.null(),
    write: async () => null,
};

/** The world of a residency's home hosted on its cloud machine, its fixture writing nothing. */
export const RESIDENCY_HOME: SpaceWorldWrite<null> = {
    name: "residency",
    value: schema.null(),
    spaces: [{ residency: RESIDENCY }],
    write: async () => null,
};

/** A fixture's world a space template writes on its born space, and its copies share. */
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
export type AccountSpace = Required<Pick<TestSpaceOptions, "id" | "account" | "name">>;

/** A space a world hosts: an owner's space, a person's home by email, or a residency's home. */
export type WorldSpace = (
    | AccountSpace
    | { readonly home: string }
    | { readonly residency: ResidencyCode }
) & {
    /** The packages the space serves, the world's when absent. */
    readonly packages?: TestSpaceOptions["packages"];
};

/** The databases of a space world: the universe's and the machine's replica. */
type SpaceDatabases = {
    readonly universe: typeof Universe.database;
    readonly machine: typeof TestMachine.database;
};

/** The directory below the machine's files that stores its own resources, such as Git storage. */
const RESOURCES = "resources";

/** The origin a test calls its served space at. */
const SPACE_URL = "http://space.test";

/** The scheme a request's authorization header puts its bearer under (RFC 6750 2.1). */
const BEARER_PREFIX = "Bearer ";

/** A JSON Web Token's compact form: three base64url segments (RFC 7519 3). */
const JWT_FORM = /^[\w-]+\.[\w-]+\.[\w-]*$/u;

/** The version of the account fixture's stand-in stacks, which the fixture's space builds again. */
const STACK_VERSION = "2026.9.0";

/** The export of the shipped platform stack that every account's home runs. */
const HOME_EXPORT = "home";

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
    residencyHome: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000009"),
    repository: schema
        .identifier("repository")
        .parse("repository-01996ab0-0000-7000-8000-00000000000b"),
    reference: schema
        .identifier("reference")
        .parse("reference-01996ab0-0000-7000-8000-00000000000c"),
};

/** The fixture repository's main branch at the residency's forge, which installations follow. */
export const FIXTURE_BRANCH = reference.reference(ids.residencyHome, ids.reference);

/** The fixture's account, which its spaces belong to. */
const FIXTURE_ACCOUNT = { id: ids.account, handle: "example" } as const;

/** The commit every fixture release names, as no checkout built it. */
const FIXTURE_COMMIT = "0".repeat(40);

/** The fixture repository's commit at a package directory, the source of the fixture's builds. */
export function fixtureSource(directory = ".") {
    return {
        kind: "commit" as const,
        repository: ids.repository,
        commit: Commit.parse(FIXTURE_COMMIT),
        directory,
    };
}

/** The fixture's space, which a world hosts alone unless it lists others. */
const FIXTURE_SPACE: WorldSpace = { id: ids.space, account: ids.account, name: "space" };

/** A space of a test universe, hosted on its residency's cloud machine over a test database. */
export class TestSpace implements AsyncDisposable, TestScope {
    /** The hosted space. */
    readonly id: Identifier<"space">;
    /** The account the space belongs to. */
    readonly account: Identifier<"account">;
    /** The universe the space lives in. */
    readonly universe: TestUniverse;
    /** The space's owner, signed up at the universe. */
    readonly owner: TestPerson;
    /** The machine hosting the space, by its identifier in the world's machines. */
    readonly #machineId: Identifier<"machine">;
    /** The test machine the space's machine runs as. */
    readonly #test: TestMachine;
    /** The world the universe, the machine and the space run in, which tests move through. */
    readonly simulation: Simulation;
    /** Open each residency's home the world serves with its forge, once, by residency. */
    readonly homes: Map<ResidencyCode, () => Promise<TestSpace>>;
    /** The world's machines answering at their origins, this space's machine among them. */
    readonly gateway: TestGateway;
    /** The world's running machines, by machine. */
    readonly machines: Map<Identifier<"machine">, Machine>;
    /** Start each of the world's machines again over its options, some changed, by machine. */
    readonly starts: Map<
        Identifier<"machine">,
        (
            changes: Pick<
                TestSpaceOptions,
                "isRunner" | "isEnforcingEntitlements" | "runtimes" | "destack"
            >,
        ) => Promise<Machine>
    >;
    /** The failures the world captured, which its disposal refuses untaken. */
    readonly telemetry: TestTelemetry;
    /** The servers the world's spaces run in process, which its machines load by linked image. */
    readonly servers: PackageImplementation[];
    /** The function stopping each running machine's follow of its directory, by machine. */
    readonly stops: Map<Identifier<"machine">, () => Promise<void>>;
    /** The releases the machine links with the space. */
    readonly releases: StackReleases;
    /** The stack the space declares, as the test last applied it. */
    #stack: JsonObject = {};
    /** The index of the builds, opened once. */
    #index: Promise<BunGraphStorage> | undefined;
    /** The store of the start the space served at its last serve, whose types callers call. */
    #store: ObjectStore | undefined;
    /** Run a cleanup once the fixture's test or template write ends. */
    readonly #defer: Defer;

    /** The world the space's machine runs in, with every capability. */
    get environment(): Environment<HostCapability> {
        return this.simulation.environment(UNIVERSE_NODE);
    }

    /** Hold a space a machine hosts. */
    private constructor(fields: {
        readonly id: Identifier<"space">;
        readonly account: Identifier<"account">;
        readonly universe: TestUniverse;
        readonly owner: TestPerson;
        readonly machine: Identifier<"machine">;
        readonly test: TestMachine;
        readonly simulation: Simulation;
        readonly homes: Map<ResidencyCode, () => Promise<TestSpace>>;
        readonly gateway: TestGateway;
        readonly machines: Map<Identifier<"machine">, Machine>;
        readonly starts: Map<
            Identifier<"machine">,
            (
                changes: Pick<
                    TestSpaceOptions,
                    "isRunner" | "isEnforcingEntitlements" | "runtimes" | "destack"
                >,
            ) => Promise<Machine>
        >;
        readonly telemetry: TestTelemetry;
        readonly servers: PackageImplementation[];
        readonly stops: Map<Identifier<"machine">, () => Promise<void>>;
        readonly releases: StackReleases;
        readonly defer: Defer;
    }) {
        // hold the space, its universe, owner and machines, and the world it runs in
        this.id = fields.id;
        this.account = fields.account;
        this.universe = fields.universe;
        this.owner = fields.owner;
        this.#machineId = fields.machine;
        this.#test = fields.test;
        this.simulation = fields.simulation;
        this.homes = fields.homes;
        this.gateway = fields.gateway;
        this.machines = fields.machines;
        this.starts = fields.starts;
        this.telemetry = fields.telemetry;
        this.servers = fields.servers;
        this.stops = fields.stops;
        this.releases = fields.releases;
        this.#defer = fields.defer;
    }

    /** The machine hosting the space now. */
    get machine(): Machine {
        return present(this.machines.get(this.#machineId), `the machine '${this.#machineId}'`);
    }

    /** The space as its machine serves it now. */
    get space(): Promise<Space> {
        return this.machine.space(this.id);
    }

    /** The space's database: its volume's on its machine now. */
    get database(): DatabaseConnection {
        return TestSpace.#volumes(this.machine).database(this.id).database;
    }

    /** The served scope: the space. */
    get scope(): string {
        return this.id;
    }

    /** The directory of the machine's files: databases, blobs, buckets and builds. */
    get files(): string {
        return TestSpace.#volumes(this.machine).directory;
    }

    /** The directory below which the machine stores its own resources, such as Git storage. */
    get resources(): string {
        return join(TestSpace.#volumes(this.machine).directory, RESOURCES);
    }

    /** Read a test machine's volumes, which every machine of a test world stores in. */
    static #volumes(machine: Machine): TestSpaceStorage {
        const { volumes } = machine;
        if (!(volumes instanceof TestSpaceStorage)) {
            throw new TypeError(`the machine '${machine.id}' stores no test volumes`);
        }

        return volumes;
    }

    /** Open a space made at the universe, the fixture's account and space by default, hosted. */
    static async open(options: TestSpaceOptions = {}): Promise<TestSpace> {
        // restore the template's world for the fixture's own space in a fork of it, refusing a given world
        const isWorld = WORLD_OPTIONS.every((name) => options[name] === undefined);
        if (isWorld) {
            const template = TestSpace.template(options, BORN_SPACE);
            const copy = await TestSpace.copy(template, options);

            return copy.space;
        }

        // make it in the test's world, else in the fixture's own, run through the world's simulation
        const simulation = TestSpace.#simulation(options);
        const defer = TestSpace.#afterTest();

        return await TestSpace.#create(options, { defer, simulation });
    }

    /** Host the residency's home on its cloud machine as the operator. */
    static async residency(options: Omit<TestSpaceOptions, "owner"> = {}): Promise<TestSpace> {
        // copy the run's written home when the test brings no world of its own
        const given: TestSpaceOptions = options;
        const isWorld = WORLD_OPTIONS.every((name) => given[name] === undefined);
        if (isWorld) {
            const template = TestSpace.template(options, RESIDENCY_HOME);
            const copy = await TestSpace.copy(template, options);

            return copy.space;
        }

        // host it in the test's world, run through the world's simulation
        const simulation = TestSpace.#simulation(options);
        const defer = TestSpace.#afterTest();

        return await TestSpace.#residency(options, RESIDENCY, { defer, simulation });
    }

    /** Host a residency's home on its cloud machine as the operator, born as at its first start. */
    static async #residency(
        options: Omit<TestSpaceOptions, "owner">,
        code: ResidencyCode,
        lifetime: {
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
            /** Create a space's database in the template a write runs in, a new one by default. */
            readonly create?: (space: Identifier<"space">) => Promise<TestDatabase>;
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
        const snapshot = universe.storage.database.snapshot();
        const home = await Residency.space(snapshot, code);
        const row = await snapshot
            .read(accountObject.space.table)
            .findFirst({ where: { id: home } });
        const address = { id: home, account: present(row, `the home of ${code}`).scope };

        // host it as the operator once the residency's machine follows its placement
        const owner = await universe.operator();
        const given = { ...options, world, owner };
        const machine = options.machine ?? (await universe.machine());
        const fixture = await TestSpace.#host(world, owner, address, given, {
            machine,
            defer,
            ...(lifetime.files === undefined ? {} : { directory: lifetime.files }),
            ...(lifetime.create === undefined ? {} : { create: lifetime.create }),
        });

        // wait until the machine serves the home, born as at its first start, then open its forge
        await fixture.serve();
        await TestSpace.#home(
            world,
            options,
            {
                machine,
                defer,
                ...(lifetime.files === undefined ? {} : { directory: lifetime.files }),
                ...(lifetime.create === undefined ? {} : { create: lifetime.create }),
            },
            fixture,
        );

        return fixture;
    }

    /** Define a template of the space's world at a start, and of the world written on it. */
    static template<World extends JsonValue>(
        options: TestSpaceOptions,
        world: SpaceWorldWrite<World>,
    ): TestTemplate<SpaceWorld<World>, SpaceDatabases> {
        // read the spaces' tables and its world's start
        const database = TestSpace.database(options);
        const names = database.tables.map((table) => table[TABLE].sqlName).toSorted();
        const { start = WORLD_START } = options;

        // name the template by its start, the fixture's world and a hash of its tables, written in a world of its own
        const tables = hashName(names.join(" "));

        return new TestTemplate({
            name: `stack.world ${new Date(start).toISOString()} ${world.name} ${tables}`,
            databases: { universe: Universe.database, machine: TestMachine.database },
            volume: TestSpace.#base(options),
            value: spaceWorldOf(world.value),
            world: { seed: WORLD_SEED, start },
            build: async ({ databases, directory, simulation, open }) => {
                // run the world's simulation over the template's databases, its machine creating the spaces' volumes
                return await TestSpace.#write(options, world, simulation, {
                    databases,
                    open,
                    directory,
                });
            },
        });
    }

    /** Write a template's world and a fixture's over its databases and directory. */
    static async #write<World extends JsonValue>(
        options: TestSpaceOptions,
        world: SpaceWorldWrite<World>,
        simulation: Simulation,
        template: {
            readonly databases: TemplateStorage<SpaceDatabases>;
            readonly open: (space: Identifier<"space">) => Promise<TestDatabase>;
            readonly directory: string;
        },
    ): Promise<SpaceWorld<World>> {
        // open the universe, the owner and the machine over the template's databases, closed once written
        const { databases, open, directory } = template;
        await using lifetime = new AsyncDisposableStack();
        const defer: Defer = (cleanup) => lifetime.defer(cleanup);
        const shared = await TestSpace.#universe(options, defer, simulation, {
            storage: databases.universe,
        });
        const { universe } = shared;
        const owner = await universe.signIn(OWNER_EMAIL);
        const machine = await universe.machine(undefined, { storage: databases.machine });

        // host each space over its template database, its files below its own directory
        const spaces = new Map<Identifier<"space">, TestSpace>();
        const states: SpaceWorld<null>["spaces"][number][] = [];
        const accounts = new Set<Identifier<"account">>();
        for (const listed of world.spaces ?? [FIXTURE_SPACE]) {
            const hosting = { defer, files: directory, simulation, create: open };
            const given = {
                ...TestSpace.#options(options, listed.packages),
                world: shared,
                machine,
            };

            // host a residency's home as the operator
            if ("residency" in listed) {
                const fixture = await TestSpace.#residency(given, listed.residency, hosting);
                spaces.set(fixture.id, fixture);
                states.push({ ...fixture.#state(), owner: OPERATOR_EMAIL });
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
                states.push({ ...fixture.#state(), owner: listed.home });
            }
            // create another space as the owner, and its account at its first space, the fixture's as example
            else {
                if (!accounts.has(listed.account)) {
                    const isFixture = listed.account === FIXTURE_ACCOUNT.id;
                    const handle = isFixture ? FIXTURE_ACCOUNT.handle : listed.name;
                    await TestSpace.account(universe, owner, { id: listed.account, handle });
                    accounts.add(listed.account);
                }
                const address = { id: listed.id, account: listed.account, name: listed.name };
                const fixture = await TestSpace.#create({ ...given, owner, ...address }, hosting);
                spaces.set(fixture.id, fixture);
                states.push({ ...fixture.#state(), owner: OWNER_EMAIL });
            }
        }

        // write the fixture's world on the first space, then return what a copy restores of each space
        const first = present(spaces.values().next().value, "the first space");
        const own = await world.write(first, directory, spaces);

        return {
            universe: await universe.world(),
            machine: machine.world(),
            files: directory,
            spaces: states.map((entry) => ({
                ...entry,
                ...present(spaces.get(entry.id), entry.id).#state(),
            })),
            world: own,
        };
    }

    /** Describe what a copy restores of the space in addition to its database and owner. */
    #state(): Omit<SpaceWorld<null>["spaces"][number], "key" | "owner"> {
        const linked = [...this.machine.linked.values()]
            .filter((release) => release.package.id !== stackPackage.id)
            .map((release) => ({ packageId: release.package.id, image: release.image }));

        return {
            id: this.id,
            account: this.account,
            stack: this.#stack,
            linked,
        };
    }

    /** Take a space's options serving its own packages, the world's when it names none. */
    static #options(
        options: TestSpaceOptions,
        packages: TestSpaceOptions["packages"] | undefined,
    ): TestSpaceOptions {
        return packages === undefined ? options : { ...options, packages };
    }

    /** Copy a template's world into a fork of it at the write's end, with the fixture's world. */
    static async copy<World extends JsonValue>(
        template: TestTemplate<SpaceWorld<World>, SpaceDatabases>,
        options: TestSpaceOptions = {},
    ): Promise<{
        readonly space: TestSpace;
        readonly spaces: ReadonlyMap<Identifier<"space">, TestSpace>;
        readonly world: World;
    }> {
        // fork the template's image, then restore its world in the fork's simulation
        const copy = await (await TestRun.image(template)).fork("copy");

        return await TestSpace.#restore(copy, options);
    }

    /** Make the space at the universe, host and birth it, over the given databases when written. */
    static async #create(
        options: TestSpaceOptions,
        lifetime: {
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
            /** Create a space's database in the template a write runs in, a new one by default. */
            readonly create?: (space: Identifier<"space">) => Promise<TestDatabase>;
            /** The directory the machine's files stay in, a temporary one by default. */
            readonly files?: string;
            /** The world the universe, the machine and the space run in. */
            readonly simulation: Simulation;
        },
    ): Promise<TestSpace> {
        // sign the owner up, and create the fixture's account unless the space goes in a given one
        const { defer, simulation } = lifetime;
        const world = await TestSpace.#universe(options, defer, simulation);
        const { universe } = world;
        const owner = options.owner ?? (await universe.signIn(OWNER_EMAIL));
        const { account = ids.account, id = ids.space, name = "space", stack } = options;
        const accounts =
            options.account === undefined
                ? await TestSpace.account(universe, owner)
                : owner.client(accountPackage);

        // host it on the machine before its creation places it there
        const machine = options.machine ?? (await universe.machine());
        const hosting = {
            machine,
            defer,
            ...(lifetime.files === undefined ? {} : { directory: lifetime.files }),
            ...(lifetime.create === undefined ? {} : { create: lifetime.create }),
        };
        const fixture = await TestSpace.#host(world, owner, { id, account }, options, hosting);

        // publish the releases the space's birth resolves at the residency's forge
        const home = await TestSpace.#home(world, options, hosting);
        for (const release of options.releases ?? []) {
            await TestSpace.#publish(home, release, [], LATEST_TAG);
        }

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

        // wait until the machine serves the space, born as at its machine's first start
        await fixture.serve();

        return fixture;
    }

    /** Restore a template's world: universe, owner, machine, each space and its own world. */
    static async #restore<World extends JsonValue>(
        copy: {
            readonly simulation: Simulation;
            readonly databases: TemplateStorage<SpaceDatabases>;
            readonly volumes: ReadonlyMap<string, TestDatabase>;
            readonly value: SpaceWorld<World>;
        },
        options: TestSpaceOptions,
    ): Promise<{
        readonly space: TestSpace;
        readonly spaces: ReadonlyMap<Identifier<"space">, TestSpace>;
        readonly world: World;
    }> {
        // run the restore's cleanups once the test ends, in the fork's simulation
        const { simulation, databases, value } = copy;
        const defer = TestSpace.#afterTest();

        // copy the machine's files and the registry's builds the write kept
        const environment = simulation.environment(UNIVERSE_NODE);
        const directory = await environment.files.makeTemporaryDirectory("destack-stack-files-");
        await environment.files.copy(value.files, directory);
        defer(() => environment.files.remove(directory, { recursive: true }));

        // restore the universe and its machine from the world's keys on a running clock, closed in turn once the test ends
        const closing = new AsyncDisposableStack();
        const world = await TestSpace.#universe(
            options,
            (cleanup) => closing.defer(cleanup),
            simulation,
            { storage: databases.universe, world: value.universe },
        );
        const { universe } = world;
        const machine = await universe.machine(undefined, {
            storage: databases.machine,
            world: value.machine,
        });
        defer(async () => {
            // close the universe and the machine, then their databases
            await closing.disposeAsync();
            await databases.machine.close();
            await databases.universe.close();

            // close the spaces' databases the restore copied
            for (const volume of copy.volumes.values()) {
                await volume.close();
            }
        });

        // hold each space's written database on the machine before it follows
        const held = new Map(
            [...copy.volumes].map(([id, database]) => [
                schema.identifier("space").parse(id),
                database,
            ]),
        );
        const linked = value.spaces.flatMap((state) =>
            state.linked.flatMap(({ packageId, image }) => {
                const server = (options.servers ?? []).find(
                    (each) => each.package.id === packageId,
                );

                return server === undefined
                    ? []
                    : [{ package: PackageVersion.of(server.package), image }];
            }),
        );
        const spaces = new Map<Identifier<"space">, TestSpace>();
        for (const state of value.spaces) {
            const owner = universe.person(state.owner);
            const fixture = await TestSpace.#host(world, owner, state, options, {
                machine,
                defer,
                directory,
                databases: held,
                linked,
            });
            fixture.#stack = state.stack;
            spaces.set(state.id, await fixture.serve());
        }
        const first = present(spaces.values().next().value, "the first space");

        return { space: first, spaces, world: value.world };
    }

    /** Create the fixture's account as its owner, answering the owner's account client. */
    static async account(
        universe: TestUniverse,
        owner: TestPerson,
        created: { readonly id: Identifier<"account">; readonly handle: string } = FIXTURE_ACCOUNT,
    ): Promise<ClientOf<typeof accountPackage>> {
        // create the account once, as the residency's forge creates the fixture's first
        const accounts = owner.client(accountPackage);
        const [existing] = await universe.storage.database
            .select({ id: accountObject.account.table.id })
            .from(accountObject.account.table)
            .where(eq(accountObject.account.table.id, created.id));
        if (existing !== undefined) {
            return accounts;
        }
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
        const defer = TestSpace.#afterTest();
        const world = await TestSpace.#universe(options, defer, simulation);
        const { universe } = world;
        const owner = options.owner ?? (await universe.signIn(OWNER_EMAIL));
        const machine = options.machine ?? (await universe.machine());

        return await TestSpace.#host(
            world,
            owner,
            { id: ids.space, account: ids.account },
            options,
            { machine, defer },
        );
    }

    /** Declare the space's database as a machine does, with the test's additions. */
    static database(options: TestSpaceOptions): ReturnType<typeof defineDatabase> {
        const { packages = [], follows = [], tables = [] } = options;

        return SpaceStorage.main({ packages, follows, tables });
    }

    /** Define the tables a space's database has before its installations deploy. */
    static #base(options: TestSpaceOptions): ReturnType<typeof defineDatabase> {
        const { follows = [], tables = [] } = options;

        return SpaceStorage.main({ packages: [], follows, tables });
    }

    /** Run a cleanup once the test ends. */
    static #afterTest(): Defer {
        // run the cleanups in reverse once the test ends, closing what serves before what it serves from
        const cleanups = new AsyncDisposableStack();
        onTestFinished(() => cleanups.disposeAsync());

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
        return new Simulation({ world: testWorldOptions(), seed: WORLD_SEED, start });
    }

    /** Build the releases a test machine links with the fixture's space, with a world's crypto. */
    static async releases(crypto: Crypto): Promise<StackReleases> {
        // build the empty stack and the stack package, declaring its settings
        const empty = await MemoryBuild.of(
            PackageVersion.parse({
                id: STACKS.space.packageId,
                name: "@example/space",
                version: STACK_VERSION,
            }),
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
            PackageVersion.of(stackPackage),
            [buildRetention].map((declared) => ({
                kind: "setting",
                kindPackageId: settingPackage.id,
                name: declared.name,
                description: describeSetting(declared),
            })),
            crypto,
        );

        // build the object package's release, which machines link, its kinds' exports loading in process
        const files = new Map([
            ["destack.json", new TextEncoder().encode(JSON.stringify(objectDefinition))],
        ]);
        const inspect = { "./inspect": { path: "server/inspect.js", imports: [] } };
        const objectRelease = await MemoryBuild.of(PackageVersion.of(objectPackage), [], crypto, {
            role: "builtin",
            files,
            outputs: fixtureOutputs({
                server: { runtime: "bun", emit: true, workloads: {}, views: {}, exports: inspect },
                worker: {
                    runtime: "workerd",
                    emit: true,
                    workloads: {},
                    views: {},
                    exports: inspect,
                },
            }),
        });

        // build the empty platform stack every residency's and account's home runs, which machines ship
        const residency = await MemoryBuild.of(
            PackageVersion.parse({
                id: RESIDENCY_STACK.packageId,
                name: "@platform/stack",
                version: STACK_VERSION,
            }),
            [RESIDENCY_STACK.export, HOME_EXPORT].map((name) => ({
                kind: "stack",
                kindPackageId: stackPackage.id,
                name,
                description: { installations: {} },
            })),
            crypto,
        );

        // follow the space stack at its release, each build identified by its image alone
        const source = {
            ...STACKS.space,
            version: { kind: "release" as const, version: STACK_VERSION },
        };

        return {
            space: empty,
            stack,
            source,
            sourceImage: await imageOf(empty, crypto),
            image: await imageOf(stack, crypto),
            object: objectRelease,
            objectImage: await imageOf(objectRelease, crypto),
            residency,
            residencyImage: await imageOf(residency, crypto),
        };
    }

    /** Take the world the space joins, else open a universe and registry closed after the test. */
    static async #universe(
        options: TestSpaceOptions,
        defer: Defer,
        simulation: Simulation,
        stored: {
            /** The database the universe stores in, a fresh one by default. */
            readonly storage?: TestDatabase;
            /** The universe's world a template wrote, none by default. */
            readonly world?: UniverseWorld;
        } = {},
    ): Promise<TestSpaceWorld> {
        // join the given world
        if (options.world !== undefined) {
            return options.world;
        }

        // open the universe, its spaces born with the stack the residency's forge publishes, its residencies' homes with the shipped one
        const { storage, world } = stored;
        const environment = simulation.environment(UNIVERSE_NODE);
        const { source } = await TestSpace.releases(environment.crypto);
        const homes = new Map<ResidencyCode, () => Promise<TestSpace>>();
        const universe = await TestUniverse.open({
            stacks: {
                home: { ...RESIDENCY_STACK, export: HOME_EXPORT },
                space:
                    (options.isPublishingFixtures ?? true)
                        ? source
                        : { ...RESIDENCY_STACK, export: HOME_EXPORT },
            },
            home: RESIDENCY_STACK,
            // ship the forge's and the space's linked builds, as a deploy ships its distribution
            ...(await TestSpace.#distribution(
                TestSpace.#withForge(options).linked ?? [],
                environment,
            )),
            ...(storage === undefined ? {} : { storage }),
            ...(world === undefined ? {} : { world }),
            ...(options.residencies === undefined ? {} : { residencies: options.residencies }),
            ...(options.publishers === undefined ? {} : { publishers: options.publishers }),
            simulation,
        });
        defer(() => universe[Symbol.asyncDispose]());
        const gateway = new TestGateway(simulation);
        defer(() => gateway[Symbol.asyncDispose]());
        const telemetry = TestTelemetry.of(simulation.telemetry);
        defer(() => telemetry[Symbol.dispose]());

        return {
            simulation,
            universe,
            homes,
            gateway,
            machines: new Map(),
            starts: new Map(),
            telemetry,
            servers: [],
            stops: new Map(),
        };
    }

    /** Run the world's machine of a test machine, building it once with its volumes and files. */
    static async #machine(
        world: TestSpaceWorld,
        test: TestMachine,
        options: TestSpaceOptions,
        lifetime: {
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
            /** The directory the machine's files stay in, a temporary one by default. */
            readonly directory?: string;
            /** The databases of the spaces the machine stores already, held before it follows. */
            readonly databases?: ReadonlyMap<Identifier<"space">, TestDatabase>;
            /** Create a space's database in the template a write runs in, a new one by default. */
            readonly create?: (space: Identifier<"space">) => Promise<TestDatabase>;
            /** The releases of the servers' packages the machine links from its start. */
            readonly linked?: readonly PackageRelease[];
        },
    ): Promise<Machine> {
        // run each test machine once per world
        const running = world.machines.get(test.id);
        if (running !== undefined) {
            return running;
        }

        return await TestSpace.#startMachine(world, test, options, lifetime);
    }

    /** Build a test machine's machine over its files and start its follow of the directory. */
    static async #startMachine(
        world: TestSpaceWorld,
        test: TestMachine,
        options: TestSpaceOptions,
        lifetime: {
            readonly defer: Defer;
            readonly directory?: string;
            readonly databases?: ReadonlyMap<Identifier<"space">, TestDatabase>;
            readonly create?: (space: Identifier<"space">) => Promise<TestDatabase>;
            readonly linked?: readonly PackageRelease[];
        },
    ): Promise<Machine> {
        // open the machine's files: its directory, buckets, builds and the spaces' own databases
        const { defer } = lifetime;
        const { simulation, universe } = world;
        const environment = simulation.environment(UNIVERSE_NODE);
        const withForge = TestSpace.#withForge(options);
        const { files } = environment;
        const directory =
            lifetime.directory ?? (await files.makeTemporaryDirectory("destack-stack-files-"));
        const main = join(directory, "main");
        const databases = join(directory, "databases");
        await files.mkdir(main, { recursive: true });
        await files.mkdir(databases, { recursive: true });
        const buckets = new LocalBucketHost({
            directory: join(directory, "buckets"),
            endpoint: new URL("http://s3.localhost"),
            region: "local",
            environment,
            credentials: async () => BUCKET_CREDENTIALS,
        });
        const builds = await TestBuildStore.open(
            join(directory, "builds"),
            environment,
            options.layer ?? BuildLayer.empty(),
        );
        defer(async () => {
            await buckets[Symbol.asyncDispose]();
            await builds[Symbol.asyncDispose]();
            if (lifetime.directory === undefined) {
                await files.remove(directory, { recursive: true });
            }
        });

        // close the spaces' databases the machine created once it stops
        const created = new Set<TestDatabase>();
        defer(async () => {
            for (const database of created) {
                await database.close();
            }
        });

        // open the volumes over the main database, with the machine's resources' providers
        const storage = simulation.environment(test.id);
        const { dialect } = universe.storage.database;
        const local = withForge.resources?.(join(directory, RESOURCES), environment);
        const volumes = new TestSpaceStorage({
            main: TestSpace.#base(options),
            replicated: SpaceStorage.replicas({ follows: withForge.follows ?? [] }),
            databases: lifetime.databases ?? new Map(),
            create:
                lifetime.create ??
                (async () => {
                    // create a new space's database, closed with the others once the machine stops
                    const database = await TestDatabase.create(TestSpace.#base(options), {
                        environment: storage,
                        dialect,
                        isMigrated: true,
                    });
                    created.add(database);

                    return database;
                }),
            objects: TestSpaceStorage.objects(withForge.packages ?? []),
            environment: storage,
            files: { directory, buckets },
            // store each space's databases as SQLite files
            providers: [
                ...(withForge.providers ?? []),
                ...(local?.providers ?? []),
                databaseProvider(
                    new BunSqliteDatabaseHost(pathToFileURL(`${databases}/`), environment),
                    object.database,
                ),
                TestSpaceDatabaseHost.over(main, environment, (space) => volumes.database(space)),
            ],
            connectors: [...(withForge.connectors ?? []), ...(local?.connectors ?? [])],
        });

        // build the machine as the residency's cloud machine
        const stack = await TestSpace.releases(environment.crypto);

        // store the system releases the machine ships and links: the residency's stack, the stack and object packages, and the servers' builds
        for (const build of [stack.residency, stack.stack, stack.object]) {
            await imageBuild(builds, build, []);
        }
        const shipped: PackageRelease[] = [];
        for (const build of withForge.linked ?? []) {
            const { image } = await imageBuild(builds, build, []);
            shipped.push({ package: PackageManifest.package(build.manifest), image });
        }
        const linked = new Map<PackageId, PackageRelease>(
            [
                { package: PackageVersion.of(stackPackage), image: stack.image },
                { package: PackageVersion.of(objectPackage), image: stack.objectImage },
                ...(lifetime.linked ?? []),
                ...shipped,
            ].map((release) => [release.package.id, release]),
        );
        // read the machine's row at the universe, which says how it enrolled
        const enrolledAs = async () =>
            (
                await universe.storage.database
                    .select({
                        kind: accountObject.machine.table.kind,
                        scope: accountObject.machine.table.scope,
                        platform: accountObject.machine.table.platform,
                        architecture: accountObject.machine.table.architecture,
                    })
                    .from(accountObject.machine.table)
                    .where(eq(accountObject.machine.table.id, test.id))
            )[0];

        // start the machine with the options as changed, over the same volumes, files and links
        const start = async (
            changes: Pick<
                TestSpaceOptions,
                "isRunner" | "isEnforcingEntitlements" | "runtimes" | "destack"
            >,
        ): Promise<Machine> => {
            // build the machine over the options as changed, its first runtime keeping its instances' sockets
            const changed = { ...options, ...changes };
            const runtimes = changed.runtimes?.({ scheduler: simulation.scheduler, builds }) ?? [
                new TestRuntime({ scheduler: simulation.scheduler, builds }),
                new ProcessRuntime(),
            ];
            const machine: Machine = new Machine({
                id: test.id,
                environment: simulation.environment(test.id),
                identity: test.identity,
                keyring: test.keyring,
                database: test.database,
                enrollment: async () => {
                    // read how the universe enrolled the machine: under an account, or as the residency's
                    const row = await enrolledAs();
                    const enrolled = {
                        universe: TEST_UNIVERSE.toJSON(),
                        status: "enrolled",
                    } as const;

                    return row?.kind === "personal"
                        ? {
                              kind: "personal",
                              accountId: accountObject.account.identifier(row.scope),
                              ...enrolled,
                          }
                        : { kind: "cloud", residencyId: RESIDENCY, ...enrolled };
                },
                describe: async () => {
                    // report a personal machine's hardware, as it enrolled with it
                    const row = await enrolledAs();
                    const host =
                        row?.kind === "personal" &&
                        row.platform !== null &&
                        row.architecture !== null
                            ? { platform: row.platform, architecture: row.architecture }
                            : null;

                    return { osVersion: null, startedAt: null, update: null, host };
                },
                volumes,
                builds,
                targets: ["bun", "browser"],
                isRunner: changed.isRunner ?? true,
                isEnforcingEntitlements: changed.isEnforcingEntitlements ?? false,
                isServingHostnames: options.isServingHostnames ?? true,
                domain: "localhost",
                runtimes,
                instances: (instanceId, path, request) =>
                    nth(runtimes, 0).deliver(
                        schema.identifier("instance").parse(instanceId),
                        path,
                        new Request(`http://instance${path}`, request),
                    ),
                alarm: undefined,
                server: async (image) =>
                    world.servers.find(
                        (server) => machine.linked.get(server.package.id)?.image === image,
                    ),
                linked: world.machines.get(test.id)?.linked ?? linked,
                shipped: [
                    {
                        package: PackageManifest.package(stack.residency.manifest),
                        image: stack.residencyImage,
                    },
                    ...shipped,
                ],
                endpoint:
                    options.origin ?? (await test.directory.answering(test.id, test.clock.now())),
                destack: changed.destack ?? { oldest: DESTACK_VERSION, newest: DESTACK_VERSION },
                authenticate: (request, audience, partition) =>
                    TestSpace.#authenticate(machine, request, audience, partition),
                authorizeSpace: async () => {},
            });

            // run it as the world's machine, answering at its origin before it follows, as its spaces' starts call it
            world.machines.set(test.id, machine);
            await world.gateway.serve(machine);

            // follow the directory as the machine until it caught up
            world.stops.set(test.id, await TestSpace.#follow(machine, defer));

            return machine;
        };
        world.starts.set(test.id, start);

        return await start({});
    }

    /** Run a machine's follow of its directory until stopped, waiting until it caught up. */
    static async #follow(machine: Machine, defer: Defer): Promise<() => Promise<void>> {
        // follow at each change of the machine's database
        const stopping = new AbortController();
        const caught = Promise.withResolvers<void>();
        const { log } = machine.database;
        const changed = async () => {
            const { sequence } = await log.position();
            await log.waitUntil(
                async () => (await log.position()).sequence > sequence,
                stopping.signal,
            );
        };
        const running = machine.follow({
            key: machine.id,
            signal: stopping.signal,
            changed,
            synced: () => caught.resolve(),
        });

        // stop once the world ends, then wait until caught up
        const stop = async () => {
            stopping.abort();
            await running;
        };
        defer(stop);
        await Promise.race([caught.promise, running]);

        return stop;
    }

    /** Verify a caller as a cloud machine does, a test caller by its bearer's subject. */
    static async #authenticate(
        machine: Machine,
        request: Request,
        audience: PackageId,
        partition: string | undefined,
    ): Promise<Authentication | undefined> {
        // serve a request without a bearer as no one
        const header = request.headers.get("authorization");
        if (header === null) {
            return undefined;
        }

        // verify the universe's or a space's token, bound to the space it calls
        if (JWT_FORM.test(header.slice(BEARER_PREFIX.length))) {
            const verified = await accountObject.Identity.authenticate(
                machine.directory.snapshot,
                request,
                machine.verifier,
                { audience, now: machine.clock.now() },
            );
            if (verified !== undefined && partition !== undefined) {
                const space = await machine.space(schema.identifier("space").parse(partition));
                await space.requirePartition(verified);
            }

            return verified;
        }

        // authenticate a test caller by its bearer's subject, a person's in the space it calls
        const now = machine.clock.now();
        const subject = TestBearer.subject(request);
        const assurance = TestBearer.assurance(request, now);
        const tested = TestBearer.authenticate(subject, audience, now, assurance);
        if (partition === undefined) {
            return tested;
        }

        // bind the token to the space it calls
        const kind = principal.user.is(subject) ? "session" : tested.credential.kind;

        return {
            caller: tested.caller,
            credential: { ...tested.credential, kind, scope: partition },
        };
    }

    /** Host a space on the world's machine of a test machine, closed with its lifetime. */
    static async #host(
        world: TestSpaceWorld,
        owner: TestPerson,
        address: { readonly id: Identifier<"space">; readonly account: Identifier<"account"> },
        options: TestSpaceOptions,
        lifetime: {
            /** The test machine hosting the space. */
            readonly machine: TestMachine;
            /** Run a cleanup once the test or the template's write ends. */
            readonly defer: Defer;
            /** The directory the machine's files stay in, a temporary one by default. */
            readonly directory?: string;
            /** The databases of the spaces the machine stores already, held before it follows. */
            readonly databases?: ReadonlyMap<Identifier<"space">, TestDatabase>;
            /** Create a space's database in the template a write runs in, a new one by default. */
            readonly create?: (space: Identifier<"space">) => Promise<TestDatabase>;
            /** The releases of the servers' packages the machine links from its start. */
            readonly linked?: readonly PackageRelease[];
        },
    ): Promise<TestSpace> {
        // run the space's servers on the world's machine
        const { machine: test, defer } = lifetime;
        const { simulation, universe, gateway } = world;
        const withForge = TestSpace.#withForge(options);
        for (const server of withForge.servers ?? []) {
            if (!world.servers.includes(server)) {
                world.servers.push(server);
            }
        }
        const machine = await TestSpace.#machine(world, test, options, {
            defer,
            ...(lifetime.directory === undefined ? {} : { directory: lifetime.directory }),
            ...(lifetime.databases === undefined ? {} : { databases: lifetime.databases }),
            ...(lifetime.create === undefined ? {} : { create: lifetime.create }),
            linked: lifetime.linked ?? [],
        });
        await gateway.serve(machine);

        // store and link the builds the space ships on a machine that started without them
        for (const build of withForge.linked ?? []) {
            const released = PackageManifest.package(build.manifest);
            if (machine.linked.get(released.id) === undefined) {
                const { image } = await imageBuild(machine.builds, build, []);
                machine.link({ package: released, image });
            }
        }

        // hold the fixture, closed with its lifetime
        const fixture = new TestSpace({
            ...address,
            universe,
            owner,
            machine: machine.id,
            test,
            simulation,
            homes: world.homes,
            gateway,
            machines: world.machines,
            starts: world.starts,
            telemetry: world.telemetry,
            servers: world.servers,
            stops: world.stops,
            releases: await TestSpace.releases(test.environment.crypto),
            defer,
        });
        defer(() => fixture[Symbol.asyncDispose]());

        // let the world open its residency's forge through the first space it hosts
        if (!world.homes.has(RESIDENCY)) {
            const hosting = { machine: fixture.#test, defer: fixture.#defer };
            world.homes.set(RESIDENCY, TestSpace.#homeOpener(world, options, hosting, fixture));
        }

        return fixture;
    }

    /** Publish a build with the builds it runs with to the residency's forge as its owner. */
    async publish(
        build: BuildReader,
        builds: readonly BuildReader[] = [],
        tag: string = LATEST_TAG,
    ): Promise<Digest> {
        return await TestSpace.#publish(await this.#openedHome(), build, builds, tag);
    }

    /** Report whether the residency's forge stores an object below the fixtures' handle. */
    async isPushed(digest: Digest): Promise<boolean> {
        // ask the uploads below the account's handle
        const home = await this.#openedHome();
        const builds = `${await TestSpace.#forgeUrl(home)}${registryPackage.mounts.builds.path}`;
        const uploads = new RemoteBuildStore(
            `${builds}/${FIXTURE_ACCOUNT.handle}/`,
            TestSpace.#fetchAs(home),
        );
        const [isStored] = await uploads.has([digest]);

        return isStored === true;
    }

    /** Open the world's residency home with its forge, which the fixtures publish to. */
    #openedHome(): Promise<TestSpace> {
        return TestSpace.#home(this, {}, { machine: this.#test, defer: this.#defer }, this);
    }

    /** Serve a world's residency home with its forge once, with the fixtures' releases. */
    static #home(
        world: TestSpaceWorld,
        options: TestSpaceOptions,
        hosting: TestHosting,
        caller?: TestSpace,
    ): Promise<TestSpace> {
        // open it once per world, through the opener set first
        let open = world.homes.get(RESIDENCY);
        if (open === undefined) {
            open = TestSpace.#homeOpener(world, options, hosting, caller);
            world.homes.set(RESIDENCY, open);
        }

        return open();
    }

    /** Make an opener of a world's residency home that opens it once. */
    static #homeOpener(
        world: TestSpaceWorld,
        options: TestSpaceOptions,
        hosting: TestHosting,
        caller?: TestSpace,
    ): () => Promise<TestSpace> {
        let opening: Promise<TestSpace> | undefined;

        return () => (opening ??= TestSpace.#openHome(world, options, hosting, caller));
    }

    /** Serve the residency's home with its forge, its fixtures' account and releases, each once. */
    static async #openHome(
        world: TestSpaceWorld,
        options: TestSpaceOptions,
        hosting: TestHosting,
        caller?: TestSpace,
    ): Promise<TestSpace> {
        // host the residency's home as the operator, applying its forge's stack once
        const { universe } = world;
        const snapshot = universe.storage.database.snapshot();
        const home = await Residency.space(snapshot, RESIDENCY);
        const row = await snapshot
            .read(accountObject.space.table)
            .findFirst({ where: { id: home } });
        const address = { id: home, account: present(row, `the home of ${RESIDENCY}`).scope };
        const operator = await universe.operator();
        const residency =
            caller?.id === home
                ? caller
                : await TestSpace.#host(
                      world,
                      operator,
                      address,
                      { ...options, world, owner: operator },
                      hosting,
                  );
        await residency.serve();
        const installed = await residency.database
            .snapshot()
            .read(installation.table)
            .findFirst({ where: { name: FORGE_INSTALLATION } });
        const isApplied = installed !== undefined;
        if (!isApplied) {
            await residency.apply([...FORGE_HOME_TYPES], FORGE_HOME_STACK);
        }

        // publish nothing in a world holding no fixtures
        if (!(options.isPublishingFixtures ?? true)) {
            return residency;
        }

        // create the fixtures' account as its owner, who publishes them
        const owner = await universe.signIn(OWNER_EMAIL);
        await TestSpace.account(universe, owner);

        // publish the space stack and the fixture package's versions, each once
        const { crypto } = universe.environment;
        const { space } = await TestSpace.releases(crypto);
        await TestSpace.#publish(residency, space, [], LATEST_TAG);
        for (const version of FIXTURE_VERSIONS) {
            const build = await fixtureBuild(ids.package, version, crypto);
            await TestSpace.#publish(residency, build, [], LATEST_TAG);
        }

        // point the fixture package's tags
        const registry = callRegistry(residency.machine, home, TestSpace.#fetchAs(residency));
        for (const [name, version] of Object.entries(TAGS)) {
            await registry.package.point({
                spaceId: home,
                id: ids.package,
                requestId: RequestId.create(universe.environment),
                name,
                version,
            });
        }

        return residency;
    }

    /** Publish a build in an image hoisted with its builds under a tag, once per image. */
    static async #publish(
        home: TestSpace,
        build: BuildReader,
        builds: readonly BuildReader[],
        tag: string,
    ): Promise<Digest> {
        // store the builds in an image hoisted by package name
        const { machine } = home;
        await using store = await TestBuildStore.create(machine.environment);
        const digests: Record<string, Digest> = {};
        for (const each of builds) {
            digests[each.manifest.package.name] = await store.putIndexed(each);
        }
        const root = await store.putIndexed(build);
        const hoisted = await BuildImage.hoisted(root, digests, (digest) => store.get(digest));
        const image = await store.putImage(hoisted);

        // leave a release the forge published already from this image
        const fetch = TestSpace.#fetchAs(home);
        const registry = callRegistry(machine, home.id, fetch);
        const released = PackageManifest.package(build.manifest);
        const { items } = await registry.release.list({
            spaceId: home.id,
            where: { parentId: released.id, version: released.version },
            limit: 1,
        });
        if (items.some((each) => each.image === image)) {
            return image;
        }

        // push it below the account's handle, then publish the account's packages in it
        await machine.push(store, image, FIXTURE_ACCOUNT.id, fetch);
        await registry.package.publish({
            spaceId: home.id,
            requestId: RequestId.create(machine.environment),
            accountId: FIXTURE_ACCOUNT.id,
            image,
            commit: FIXTURE_COMMIT,
            tag,
        });

        return image;
    }

    /** Write a layer of linked builds once per process, as a deploy ships its distribution. */
    static async #distribution(
        linked: readonly BuildReader[],
        environment: Environment<"files" | "process" | "host">,
    ): Promise<Pick<TestUniverseOptions, "distribution" | "layer">> {
        // image each build in the run's store, then write their layer
        await using store = await testStore(environment);
        const images = await Promise.all(
            linked.map(async (build) => (await imageBuild(store, build, [])).image),
        );
        const layer = await TestDistribution.layer(images, store);
        const distribution = linked.map((build, index) => ({
            package: PackageManifest.package(build.manifest),
            image: nth(images, index),
        }));

        return { distribution, layer };
    }

    /** Locate the forge in the residency's home: the URL its package's mounts lie below. */
    static #forgeUrl(home: TestSpace): Promise<string> {
        const { machine, id } = home;

        return machine.directory.package(id, registryPackage.id, machine.clock.now());
    }

    /** Send requests to the residency's forge as the fixtures' owner, with a token each. */
    static #fetchAs(home: TestSpace): Fetch {
        return async (request) => {
            // exchange the fixtures' owner's token for the home, and send it as the bearer
            const owner = home.universe.person(OWNER_EMAIL);
            const { accessToken } = await owner.exchange(registryPackage.id, {
                partition: home.id,
            });
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${accessToken}`);

            return await home.machine.environment.network.fetch(new Request(request, { headers }));
        };
    }

    /** Take a world's options with the residency's forge every world serves beside them. */
    static #withForge(options: TestSpaceOptions): TestSpaceOptions {
        // add the forge's packages, servers, builds, resources and follows once
        const forge = FORGE_HOME_OPTIONS;
        const own = options.resources;
        const resources: TestSpaceOptions["resources"] =
            own === undefined || own === forge.resources
                ? forge.resources
                : (directory, environment) => {
                      const added = forge.resources(directory, environment);
                      const given = own(directory, environment);

                      return {
                          providers: [...added.providers, ...given.providers],
                          connectors: [...added.connectors, ...given.connectors],
                      };
                  };

        return {
            ...options,
            packages: union(forge.packages, options.packages),
            servers: union(forge.servers, options.servers),
            linked: union(forge.linked, options.linked),
            connectors: union(forge.connectors, options.connectors),
            providers: union(forge.providers, options.providers),
            tables: union(forge.tables, options.tables),
            follows: union(forge.follows, options.follows),
            resources,
        };
    }

    /** Read the forge's row of a package's release, which an installation's version names. */
    async releaseRow(packageId: PackageId, version: string): Promise<ObjectReference> {
        await this.#openedHome();

        return await InstallationTarget.resolveVersion(this.machine, packageId, {
            kind: "release",
            version,
        });
    }

    /** Link a build as its package's release on the machine, as a newer distribution does. */
    async link(build: BuildReader): Promise<void> {
        // store the build in the machine's builds, then link its image there
        const { image } = await imageBuild(this.machine.builds, build, []);
        this.machine.link({ package: PackageManifest.package(build.manifest), image });
    }

    /** Wait until the machine serves the space live, refusing the failures its servers report. */
    async serve(): Promise<TestSpace> {
        // run the machine's follow again after a stop, then wait until it serves the space live
        if (!this.stops.has(this.machine.id)) {
            this.stops.set(this.machine.id, await TestSpace.#follow(this.machine, this.#defer));
        }
        await this.simulation.idle();
        const store = await (await this.space).store();
        await store.waitLive(new AbortController().signal);
        await this.simulation.idle();
        this.#store = store;

        // refuse what the servers reported meanwhile
        const failures = this.#takeFailures();
        if (failures.length > 0) {
            throw new AggregateError(failures, "the space's servers reported failures");
        }

        return this;
    }

    /** Stop the machine hosting the space: its follow and every space it serves. */
    async stop(): Promise<void> {
        const stop = this.stops.get(this.machine.id);
        this.stops.delete(this.machine.id);
        await stop?.();
    }

    /** Restart the machine hosting the space with some options changed, as a process restarts. */
    async restart(
        changes: Pick<
            TestSpaceOptions,
            "isRunner" | "isEnforcingEntitlements" | "runtimes" | "destack"
        >,
    ): Promise<TestSpace> {
        await this.stop();
        await present(
            this.starts.get(this.#machineId),
            `the start of '${this.#machineId}'`,
        )(changes);

        return await this.serve();
    }

    /** Read the store of the start serving the space now. */
    async store(): Promise<ObjectStore> {
        return await (await this.space).store();
    }

    /** Read the installed set main applied, each at its revision and the image it runs. */
    async installed(): Promise<readonly InstallationBuild[]> {
        return (await (await this.space).installed("forges")).installed;
    }

    /** Take the distinct refusals the machine's servers reported since the last take. */
    takeRefusals(): Refusal[] {
        return TestSpace.#distinct(this.#takeFailures());
    }

    /** Take what the machine's servers reported, requiring exactly these refusals, each once. */
    expectRefusals(refusals: readonly Refusal[]): void {
        expect(this.takeRefusals()).toEqual(refusals);
    }

    /** Take the failures the world captured since the last take, as their refusals. */
    #takeFailures(): Refusal[] {
        return Refusal.take(this.telemetry);
    }

    /** Read the distinct refusals an opening or apply failed with, none when it succeeds. */
    static async refusals(applying: Promise<unknown>): Promise<readonly Refusal[]> {
        // take a settled call, the refusal it answers itself, or the servers' reports
        return await applying.then(
            () => [],
            (error: unknown) => {
                // read each reported failure's cause
                if (error instanceof AggregateError) {
                    const errors: unknown[] = error.errors;

                    return TestSpace.#distinct(errors);
                }
                // read a refusal the call answered
                else if (Failure.isDeclared(error)) {
                    const { code, reason, message } = Failure.of(error);

                    return [[code, reason, message] as const];
                }
                // rethrow any other failure
                else {
                    throw error;
                }
            },
        );
    }

    /** Read each captured refusal once, as the stack retries a refused apply. */
    static #distinct(failures: readonly unknown[]): Refusal[] {
        const refusals = new Map<string, Refusal>();
        for (const each of failures) {
            const refusal = schema
                .tuple([schema.string(), schema.string(), schema.string()])
                .parse(each);
            refusals.set(refusal.join(" "), refusal);
        }

        return [...refusals.values()];
    }

    /** Suspend or resume the space at the universe as its owner, as its machine follows. */
    async change(action: "suspend" | "resume"): Promise<void> {
        // call the universe as the owner
        await this.owner.client(accountPackage).space[action]({
            accountId: this.account,
            id: this.id,
            requestId: RequestId.create(this.machine.environment),
        });

        // wait until the space's row arrives at the machine, then run the world idle mirroring it
        await this.simulation.idle();
        await this.database.log.waitUntil(async () => {
            const [row] = await this.database
                .snapshot()
                .read(accountObject.space.table)
                .findMany({ where: { id: this.id } });

            return (row?.suspendedAt !== null) === (action === "suspend");
        }, new AbortController().signal);
        await this.simulation.idle();
    }

    /** The machine's buckets. */
    async buckets(): Promise<LocalBucketHost> {
        return TestSpace.#volumes(this.machine).buckets;
    }

    /** The space's own builds, reading through to the machine's. */
    async builds(): Promise<BuildStore> {
        return (await this.space).builds;
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

    /** Push a compiling build's files and graph as its compile's runner, then complete it. */
    async storeBuild(
        runner: TestCaller,
        target: { readonly spaceId: Identifier<"space">; readonly id: Identifier<"build"> },
        reader: BuildReader,
    ): Promise<readonly Digest[]> {
        // push the build to the space's builds mount, recording each file uploaded
        const { fetch } = runner.endpoint(stackPackage);
        const uploaded: Digest[] = [];
        const pushing = new RemoteBuildStore(`${SPACE_URL}${PUSH_PATH}/`, async (request) => {
            // record the objects the space lacks, which the push writes next
            const asked = request.clone();
            const answer = await fetch(request);
            if (new URL(request.url).pathname.endsWith("/has")) {
                const digests = schema.array(Digest).parse(await asked.json());
                const stored = schema.array(schema.boolean()).parse(await answer.clone().json());
                uploaded.push(...digests.filter((_digest, index) => stored[index] !== true));
            }

            return answer;
        });
        await using store = await TestBuildStore.create(this.machine.environment);
        const root = await store.put(reader);
        const image = await store.putImage(BuildImage.of(root));

        // attach its graph as its builder does and push it
        await store.attachGraph(root, await fixtureGraph(reader));
        await pushing.push(store, [image]);

        // complete the build's compile with the image and the outputs it asked for, as its runner
        const built = await this.database
            .snapshot()
            .read(object.build.table)
            .findFirst({ where: { id: target.id } });
        const outputs = present(built, `build '${target.id}'`).outputs;
        await this.endCompile(runner, target.id, {
            method: "complete",
            output: { image, outputs: present(outputs, "the outputs compiled") },
        });

        return uploaded;
    }

    /** Wait for a build's open compile step, which its runner completes or fails. */
    async compileStep(buildId: Identifier<"build">) {
        // read the build's compile run by its key, then the step it placed and left open
        const open = async () => {
            const view = this.database.snapshot();
            const run = await view
                .read(object.compile.run.table)
                .findFirst({ where: { key: buildId } });

            return run === undefined
                ? undefined
                : await view.read(object.compile.step.table).findFirst({
                      where: {
                          parentId: run.id,
                          function: { isNotNull: true },
                          completedAt: { isNull: true },
                      },
                  });
        };

        // wait until the build's run placed it
        await this.simulation.idle();
        await this.database.log.waitUntil(
            async () => (await open()) !== undefined,
            new AbortController().signal,
        );

        return present(await open(), `the open compile step of build '${buildId}'`);
    }

    /** End a build's open compile step as its runner, answering the build once finished. */
    async endCompile(
        runner: TestCaller,
        buildId: Identifier<"build">,
        end:
            | { readonly method: "complete"; readonly output: JsonObject }
            | { readonly method: "fail"; readonly failure: Failure },
    ) {
        // end the step as the runner
        const { step } = object.compile;
        const placed = await this.compileStep(buildId);
        const objects = present(this.#store, `the store of space '${this.id}'`);
        const caller = AccessContext.of(Caller.of(runner.subject), objects.clock.now());
        const { method, ...input } = end;
        await objects.mutate(caller, {
            id: RequestId.create(this.machine.environment),
            calls: [
                step.call(method, { ...step.target({ scope: this.id, id: placed.id }), ...input }),
            ],
        });

        return this.finished(buildId);
    }

    /** Wait until a build finished, answering it. */
    async finished(buildId: Identifier<"build">) {
        // read the build
        const read = () =>
            this.database
                .snapshot()
                .read(object.build.table)
                .findFirst({ where: { id: buildId } });

        // wait until its run recorded its outcome
        await this.simulation.idle();
        await this.database.log.waitUntil(
            async () => ((await read())?.finishedAt ?? null) !== null,
            new AbortController().signal,
        );

        return present(await read(), `build '${buildId}'`);
    }

    /** Describe the revision a submission of a build records for an installation. */
    async describeRevision(
        target: Pick<Installation, "name" | "capabilities" | "allowed">,
        reader: BuildReader,
    ): Promise<object.RevisionDescription> {
        // read the options a submission describes with
        const space = await this.space;
        const options = { space, blobs: space.blobs, known: KNOWN_TYPES };

        // make an image of the build alone in the space's store
        const { image } = await this.#image(reader, []);

        return InstallationTarget.describe(await space.builds.image(image), options);
    }

    /** Store a build in an image with the builds it runs with, answering its image and root. */
    async #image(
        reader: BuildReader,
        builds: readonly BuildReader[],
    ): Promise<{ readonly image: Digest; readonly root: Digest }> {
        // put each build and their image in the space's store
        const stored = await this.builds();

        return await imageBuild(stored, reader, builds);
    }

    /** Store a build with its graph, the fixture's by default, and record it succeeded now. */
    async recordBuild(
        reader: BuildReader,
        modules?: readonly graph.Module[],
        builds: readonly BuildReader[] = [],
    ): Promise<{ readonly kind: "build"; readonly build: Identifier<"build"> }> {
        // put the build and its image in the build store
        const { database } = this;
        const packages = await this.builds();
        const { image, root } = await this.#image(reader, builds);
        await packages.attachGraph(root, modules ?? (await fixtureGraph(reader)));

        // record it succeeded now, as the build of a commit its identifier writes
        const id = Identifier.create("build", this.machine.environment);
        const now = this.machine.clock.now();
        const digits = id.slice(id.indexOf("-") + 1).replaceAll("-", "");
        const source = { ...fixtureSource(), commit: Commit.parse(digits.padStart(40, "0")) };
        await database.transaction(
            async (transaction) =>
                await transaction.insert(object.build.table).values({
                    id,
                    scope: this.id,
                    source,
                    key: object.Build.key({ source, outputs: null, version: null }),
                    commit: source.commit,
                    runnerId: this.machine.id,
                    outputs: ["bun"],
                    state: "succeeded",
                    image,
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

    /** Record a space stack's build declaring a definition, in an image with its releases. */
    async recordStack(
        exported: string,
        definition: JsonObject,
        stacked: BuildReader = this.releases.space,
    ): Promise<{ readonly kind: "build"; readonly build: Identifier<"build"> }> {
        // pull each installed release's image
        const { installations } = StackDefinition.collections(definition);
        const released = Object.values(installations).flatMap(({ packageId, version }) =>
            // image each installation's declared release, the host's and checkouts running without one
            version?.kind === "release" ? [{ packageId, version: version.version }] : [],
        );
        const builds = await this.builds();
        const requires: Record<string, string> = {};
        const readers: BuildReader[] = [];
        for (const { packageId, version } of released) {
            const image = await (
                await this.space
            ).pull(packageId, await this.machine.resolve(packageId, version));
            const root = await builds.contents(image.root);
            requires[root.manifest.package.name] = version;
            for (const build of BuildImage.builds(image)) {
                readers.push(await builds.contents(build));
            }
        }

        // declare the definition at the space stack's export
        const reader = await MemoryBuild.of(
            PackageManifest.package(stacked.manifest),
            [{ ...STACK_KIND.identity, name: exported, description: definition }],
            this.machine.environment.crypto,
            { requires },
        );

        return await this.recordBuild(reader, undefined, readers);
    }

    /** Put a package's fixture release in the build store, as the space's build of it. */
    async release(
        packageId: PackageId,
        version: string,
    ): Promise<{
        readonly id: Identifier<"build">;
        readonly image: Digest;
        readonly recorded: Pick<object.InstallationRevision, "release" | "declarations">;
    }> {
        // replicate the forge's release into the build store, and record it as the space's build
        const image = await this.machine.resolve(packageId, version);
        const space = await this.space;
        const reader = await space.root(packageId, image);
        const release = { package: packageId, version, image };
        const inserted = await insertRelease(
            this.database,
            this.id,
            release,
            Object.keys(reader.manifest.outputs),
            this.machine.environment,
        );

        // record the release and declarations a revision of it plans its upgrade from
        const declarations = await space.blobs.writeJson(await reader.declarations());

        return { ...inserted, recorded: { release: version, declarations } };
    }

    /** The space's socket registry, which its starts share. */
    async sockets(): Promise<SocketRegistry> {
        return (await this.space).volume.sockets;
    }

    /** Call the space through its machine as a subject, or as the principal an identifier names. */
    as(caller: string | Subject): TestCaller {
        return new TestCaller(this, caller);
    }

    /** Call the space as a person, carrying the token their client exchanges for it. */
    person(
        person: TestPerson,
        partition: Identifier<"space"> | Identifier<"account"> = this.id,
    ): TestCaller {
        // sign each request with the person's token for the called package, bound to the space or the account it is the home of
        const signing: TestScope = {
            scope: this.scope,
            environment: this.environment,
            simulation: this.simulation,
            server: (handle) => this.server(handle),
            type: (typeReference) => this.type(typeReference),
            named: (name) => this.named(name),
            sign: async (request, signer) => {
                // exchange the person's token for the package, and send it as the bearer
                const { accessToken } = await person.exchange(signer.audience, { partition });
                const headers = new Headers(request.headers);
                headers.set("authorization", `${BEARER_PREFIX}${accessToken}`);

                return new Request(request, { headers });
            },
            open: (handle, signer, url, protocols) =>
                this.#open(handle, url, protocols, (upgrade) => signing.sign(upgrade, signer)),
        };

        return new TestCaller(signing, person.subject);
    }

    /** Call a package the space serves through its machine, at the package's mount. */
    server(handle: Package): { readonly fetch: (request: Request) => Promise<Response> } {
        return {
            fetch: async (request) => {
                // mount the request's path below the space and the package on the machine's origin
                const called = new URL(request.url);
                const mounted = `${SpaceMount.url(this.machine.origin, this.id)}${PackageMount.path(handle.id)}`;

                return await this.machine.serve(
                    new Request(`${mounted}${called.pathname}${called.search}`, request),
                );
            },
        };
    }

    /** Read a type the space's served store serves, with its package. */
    type(
        typeReference: ObjectTypeReference,
    ): { readonly type: ObjectType; readonly handle: Package } | undefined {
        const key = ObjectTypeReference.key(typeReference);

        return this.#types().find(
            (each) => ObjectTypeReference.key(each.type.typeReference) === key,
        );
    }

    /** Read the one type of a name the space's served store serves, refusing a shared name. */
    named(name: string): { readonly type: ObjectType; readonly handle: Package } | undefined {
        const found = this.#types().filter((each) => each.type.name === name);
        if (found.length > 1) {
            throw new TypeError(`two packages the space serves declare '${name}'`);
        }

        return found[0];
    }

    /** Open a socket to a package the space serves, its upgrade signed as a caller. */
    open(
        handle: Package,
        signer: TestSigner,
        url: string,
        protocols: readonly string[],
    ): ClientSocket {
        return this.#open(handle, url, protocols, (upgrade) => this.sign(upgrade, signer));
    }

    /** Open a socket to a package the space serves through its machine's host, signed. */
    #open(
        handle: Package,
        url: string,
        protocols: readonly string[],
        sign: (upgrade: Request) => Promise<Request>,
    ): ClientSocket {
        const host = present(this.machine.hibernation, "the machine's sockets");

        return host.open(
            async (upgrade) => this.simulation.wait(this.server(handle).fetch(await sign(upgrade))),
            WebSocketUpgrade.to(url, protocols),
        );
    }

    /** Sign a request as a test caller by its bearer, as the world's machines authenticate it. */
    async sign(request: Request, signer: TestSigner): Promise<Request> {
        const headers = new Headers(request.headers);
        for (const [name, value] of Object.entries(
            TestBearer.headers(signer.subject, signer.isSteppedUp),
        )) {
            headers.set(name, value);
        }

        return new Request(request, { headers });
    }

    /** List the types the served store's packages serve, each with its package. */
    #types(): { readonly type: ObjectType; readonly handle: Package }[] {
        const store = this.#store;
        if (store === undefined) {
            throw new TypeError(`the space '${this.id}' is called before it serves`);
        }

        return store.packages.flatMap((handler) =>
            Object.values(handler.package.objects)
                .filter((declared): declared is ObjectType => declared instanceof ObjectType)
                .map((type) => ({ type, handle: handler.package })),
        );
    }

    /** Apply the space's stack with some collections replaced, approved as its owner. */
    async apply<const Types extends readonly StackRecordType[]>(
        types: Types,
        collections: StackDocument<Types>,
    ): Promise<void> {
        // submit it and read how far the stack came
        const { applied, failures } = await this.#submit(types, collections);
        const progress = Observation.progress(applied);

        // fail with the store's reported failures
        if (failures.length > 0) {
            throw new AggregateError(failures, "the space's controllers failed");
        }
        // fail with the generation each condition was observed at unless the stack applied it
        else if (progress !== "current") {
            const message = Observation.message(applied);
            const observed = Object.entries(applied.conditions)
                .map(([name, condition]) => `${name} at ${condition.observedGeneration}`)
                .join(", ");
            throw new Error(
                `the space's stack at generation '${applied.generation}' is '${progress}', observed '${observed}': ${message}`,
            );
        }
    }

    /** Submit the space's stack with some collections replaced, answering its deploy's refusal. */
    async submit<const Types extends readonly StackRecordType[]>(
        types: Types,
        collections: StackDocument<Types>,
    ): Promise<readonly [string, string, string] | undefined> {
        // leave the stack as it was when its deploy refuses the document
        const stack = this.#stack;
        const refusals = await TestSpace.refusals(this.apply(types, collections));
        if (refusals.length === 0) {
            return undefined;
        }
        this.#stack = stack;

        return single(refusals);
    }

    /** Submit the space's stack as its owner, approving a waiting plan, with the failures. */
    async #submit<const Types extends readonly StackRecordType[]>(
        types: Types,
        collections: StackDocument<Types>,
    ) {
        // replace the collections given, once the space has its stack
        this.#stack = { ...this.#stack, ...StackDocument.of(types, collections) };
        await this.simulation.idle();
        const installed = this.as(this.owner.subject).client(stackPackage);
        const listed = await installed.installation.list({ scope: this.id });
        const stack = single(listed.items.filter((entry) => entry.export !== null));

        // record a build of the space's own stack package declaring the definition at its export
        const { space, residency } = this.releases;
        const isResidency = stack.packageId === PackageManifest.package(residency.manifest).id;
        const exported = present(stack.export, "the stack's export");
        const recorded = await this.recordStack(
            exported,
            this.#stack,
            isResidency ? residency : space,
        );

        // follow the build declaring the definition, which the space evaluates and applies
        await this.simulation.idle();
        await installed.installation.update({
            scope: this.id,
            id: stack.id,
            requestId: RequestId.create(this.machine.environment),
            revision: stack.revision,
            ...ObjectReference.columns("version", object.build.reference(this.id, recorded.build)),
            directory: null,
        });

        // approve a plan waiting for its approval as the owner
        await this.simulation.idle();
        let failures = this.#takeFailures();
        const submitted = await installed.installation.get({ scope: this.id, id: stack.id });
        const owner = this.as(this.owner.subject).client(stackPackage);
        if (submitted.conditions["Ready"]?.reason === AWAITING_APPROVAL) {
            const { digest } = await owner.installation.plan({ scope: this.id, id: stack.id });
            await owner.installation.approve({
                scope: this.id,
                id: stack.id,
                requestId: RequestId.create(this.machine.environment),
                plan: digest,
            });
            await this.simulation.idle();
            failures = [...failures, ...this.#takeFailures()];
        }

        // read how the controllers left it
        const applied = await installed.installation.get({ scope: this.id, id: stack.id });

        return { applied, failures };
    }

    /** Remove an installation from the space's stack as its owner. */
    async uninstall(name: string): Promise<void> {
        const { [name]: removed, ...kept } = StackDefinition.collections(this.#stack).installations;
        present(removed, `the installation ${name}`);
        await this.apply([], { installations: kept });
    }

    /** Install a build through the space's stack as its owner, served once applied. */
    async install(
        build: BuildReader,
        options: {
            /** The installation's bindings, none by default. */
            readonly bindings?: schema.Input<typeof InstallationDefinition>["bindings"];
            /** The capabilities the installation is given, none by default. */
            readonly capabilities?: schema.Input<typeof InstallationDefinition>["capabilities"];
            /** Where the installation's server runs, a sandbox by default. */
            readonly isolation?: schema.Input<typeof InstallationDefinition>["isolation"];
        } = {},
    ): Promise<Identifier<"installation">> {
        // follow the host's release in process or of a system package the machine links, else publish the build at the residency's forge
        const released = PackageManifest.package(build.manifest);
        const isHost = options.isolation === "host" || this.machine.linked.has(released.id);
        if (!isHost) {
            await this.publish(build);
        }

        // declare it after the earlier installations
        const name = released.name.slice(released.name.lastIndexOf("/") + 1);
        const declared = {
            packageId: released.id,
            version: isHost
                ? { kind: "host" as const }
                : { kind: "release" as const, version: released.version },
            status: "enabled" as const,
            name,
            bindings: options.bindings ?? {},
            compute: {},
            capabilities: options.capabilities ?? [],
            directories: [],
            isolation: options.isolation ?? "sandbox",
            tags: {},
        };
        const { installations } = StackDefinition.collections(this.#stack);
        await this.apply([], { installations: { ...installations, [name]: declared } });

        // wait until the machine serves the revision the stack applied
        await this.simulation.idle();

        // read the installation the stack applied, refusing one its rollout refuses to deploy
        const listed = await this.as(this.owner.subject)
            .client(stackPackage)
            .installation.list({ scope: this.id, where: { name } });
        const { id } = single(listed.items);
        const [deployed] = await this.database
            .select({ conditions: installation.table.conditions })
            .from(installation.table)
            .where(eq(installation.table.id, id));
        const synced = present(deployed, `the installation ${name}`).conditions[SYNCED];
        if (synced?.status === "false") {
            throw new Error(`the installation '${name}' failed: ${synced.message}`);
        }

        return id;
    }

    /** Connect to a served space's handoff as a caller on this space's machine. */
    handoff(caller: TestCaller): Handoff {
        // send each request signed as the caller to the served stack package
        const { fetch } = caller.endpoint(stackPackage);

        return Handoff.connect(SPACE_URL, (request) => fetch(request), this.machine.environment);
    }

    /** Fail on reports the test never took as refusals. */
    async [Symbol.asyncDispose](): Promise<void> {
        const failures = this.#takeFailures();
        if (failures.length > 0) {
            throw new AggregateError(failures, "the space's servers reported failures");
        }
    }
}

/** Where a fixture hosts a space: its test machine, cleanups, files and databases. */
interface TestHosting {
    /** The test machine hosting the space. */
    readonly machine: TestMachine;
    /** Run a cleanup once the test or the template's write ends. */
    readonly defer: Defer;
    /** The directory the machine's files stay in, a temporary one by default. */
    readonly directory?: string;
    /** Create a space's database in the template a write runs in, a new one by default. */
    readonly create?: (space: Identifier<"space">) => Promise<TestDatabase>;
}

/** The world spaces share: simulation, universe, forges, gateway and machines. */
export type TestSpaceWorld = Pick<
    TestSpace,
    | "simulation"
    | "universe"
    | "homes"
    | "gateway"
    | "machines"
    | "starts"
    | "telemetry"
    | "servers"
    | "stops"
>;

/** How a test opens a space and the machine hosting it. */
export interface TestSpaceOptions {
    /** Whether the hosting machine runs the space's commit builds, true by default. */
    readonly isRunner?: boolean;
    /** Whether the hosting machine checks use against entitlements, false by default. */
    readonly isEnforcingEntitlements?: boolean;
    /** Whether the account's hostname requests reach the hosting machine, true by default. */
    readonly isServingHostnames?: boolean;
    /** The runtimes the machine runs workloads on, given its scheduler and builds. */
    readonly runtimes?: (machine: {
        readonly scheduler: Scheduler;
        readonly builds: Pick<BuildStore, "root">;
    }) => readonly Runtime[];
    /** The Destack versions whose releases the machine runs, this checkout's alone by default. */
    readonly destack?: VersionRange;
    /** The packages the space serves in addition to the stack's, whose tables its database adds. */
    readonly packages?: readonly Pick<Package, "objects" | "tables" | "protocols">[];
    /** The shapes the space follows in addition to its own, whose replicas its database adds. */
    readonly follows?: readonly Follow[];
    /** The host's tables in addition to the space's. */
    readonly tables?: readonly Table[];
    /** The world the space joins: its simulation, universe and registry, a new one by default. */
    readonly world?: TestSpaceWorld;
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
    /** The servers the store runs, whose packages' installations run in its process. */
    readonly servers?: readonly PackageImplementation[];
    /** The builds the machine links of its servers' packages, fixed before the space serves. */
    readonly linked?: readonly BuildReader[];
    /** The builds the machine ships, read in place, none by default. */
    readonly layer?: BuildLayer;
    /** The providers the machine's volumes offer beside its database host. */
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
    readonly stack?: StackRelease;
    /** The builds the registry publishes before the space's birth, such as its stack's. */
    readonly releases?: readonly BuildReader[];
    /** Whether the residency's forge holds the fixtures' account and releases, true by default. */
    readonly isPublishingFixtures?: boolean;
}

/** Plan and apply a test space's main database on the space's own volume, as the space provider. */
export class TestSpaceDatabaseHost extends BunSqliteDatabaseHost {
    /** The provider code of the spaces' own databases. */
    override readonly provider: string = object.SPACE_PROVIDER;
    /** Read the volume database of a space the machine stores. */
    readonly #volume: (space: Identifier<"space">) => TestDatabase;

    /** Serve the spaces' own databases from their volumes, other files below a directory. */
    constructor(
        root: URL,
        environment: Environment<"files" | "server">,
        volume: (space: Identifier<"space">) => TestDatabase,
    ) {
        super(root, environment);
        this.#volume = volume;
    }

    /** Provide spaces' own databases in a world from the machine's volumes. */
    static over(
        directory: string,
        environment: Environment<"files" | "server">,
        volume: (space: Identifier<"space">) => TestDatabase,
    ): SpaceProvider {
        return databaseProvider(
            new TestSpaceDatabaseHost(pathToFileURL(`${directory}/`), environment, volume),
            object.database,
        );
    }

    /** Open further connections to a volume, each closed once disposed. */
    static #connect(volume: TestDatabase) {
        return async (tables: readonly Table[]) => {
            const connection = await volume.connect(tables);

            return Object.assign(connection, {
                [Symbol.asyncDispose]: () => connection.close(),
            });
        };
    }

    /** Diff the volume's applied state against the desired states. */
    override plan(...[record, desired]: Parameters<BunSqliteDatabaseHost["plan"]>): Promise<Plan> {
        const volume = this.#volume(schema.identifier("space").parse(record.scope));
        const { dialect } = volume.database;

        return DatabaseHandle.plan(
            TestSpaceDatabaseHost.#connect(volume),
            desired.map((state) => state.tables[dialect]),
            record.id,
        );
    }

    /** Apply the reviewed plan to the volume, refusing one that changed since review. */
    override apply(
        ...[record, desired, digest]: Parameters<BunSqliteDatabaseHost["apply"]>
    ): Promise<void> {
        const volume = this.#volume(schema.identifier("space").parse(record.scope));
        const { dialect } = volume.database;

        return DatabaseHandle.apply(
            TestSpaceDatabaseHost.#connect(volume),
            desired.map((state) => state.tables[dialect]),
            digest,
            record.id,
        );
    }
}

/** Reconcile one key of a served type's controller once, else project the object of its key. */
export async function reconcile(
    store: ObjectStore,
    name: string,
    key: Readonly<Record<string, string>>,
): Promise<void> {
    // see the key changed at each commit after the reconciliation waits
    const { log } = store.database;
    const settled = new AbortController();
    const changed = async () => {
        const { sequence } = await log.position();
        await log.waitUntil(async () => (await log.position()).sequence > sequence, settled.signal);
    };
    const reconciliation = { signal: new AbortController().signal, changed };

    // reconcile the key's rows through the controller of the name, else project the key
    const sets = store.control();
    const controller = sets.flatMap((set) => set.controllers).find((each) => each.name === name);
    const projection = sets.flatMap((set) => set.projections).find((each) => each.name === name);
    try {
        if (controller !== undefined && "reconcile" in controller) {
            const rows = await store.database
                .snapshot()
                .read(controller.table)
                .findMany({ where: key });
            await controller.reconcile(rows, { ...reconciliation, key: canonicalize(key) });
        } else if (projection !== undefined) {
            const value = present(key[projection.key], `the ${projection.key} key`);
            await projection.project(value, { ...reconciliation, key: value });
        } else {
            throw new TypeError(
                `the fixture serves no converging controller or projection '${name}'`,
            );
        }
    } finally {
        settled.abort();
    }
}

/** Run some controllers and cursors until they idle. */
export async function settle(database: DatabaseConnection, control: ControllerSet): Promise<void> {
    // run the loop until it idles
    const stopping = new AbortController();
    const loop = new ControlLoop(database, [control]);
    const running = loop.run(stopping.signal);
    await loop.idle();
    stopping.abort();
    await running;
}

/** Join two lists, the added items first, each item once. */
function union<Item>(added: readonly Item[] | undefined, own: readonly Item[] | undefined): Item[] {
    return [...new Set([...(added ?? []), ...(own ?? [])])];
}
