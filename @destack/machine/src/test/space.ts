import { Registry } from "@destack/registry/client";
import { SpaceStorage } from "../space/storage.ts";
import * as accountObject from "@destack/account/object";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import { pathToFileURL } from "node:url";
import { KNOWN_TYPES } from "@destack/stack/stack";
import { Crypto } from "@destack/environment";
import { TestTelemetry } from "@destack/telemetry/test";
import { Refusal } from "./refusal.ts";
import { InstallationDefinition, STACK_KIND, StackDefinition } from "@destack/stack/declare";
import type { Environment, HostCapability, Scheduler } from "@destack/environment";
import type { Connector, Plan } from "@destack/resource";
import { join } from "node:path";
import { LocalBucketHost } from "@destack/bucket/bun";
import {
    type ControllerSet,
    ControlLoop,
    type DatabaseConnection,
    databaseProvider,
    defineDatabase,
    eq,
    hashName,
    Snapshot,
    type Table,
    TABLE,
    DatabaseHandle,
} from "@destack/db";
import { type BuildReader, Lock } from "@destack/package/manifest";
import { ObjectTypeReference, RequestId, type Subject, ObjectReference } from "@destack/sync";
import { TestBearer, TestCaller, type TestSigner, type TestScope } from "@destack/object/test";
import { TestTemplate, TestDatabase, TestRun, type TemplateStorage } from "@destack/db/test";
import { onTestFinished, single, expect } from "@destack/test";
import { Failure } from "@destack/error";
import { type Authentication, principal } from "@destack/access";
import { Account, Residency, type ResidencyCode } from "@destack/account/object";
import { OPERATOR_EMAIL, STACKS } from "@destack/account/test";
import accountPackage from "@destack/account/package";
import {
    AWAITING_APPROVAL,
    type Follow,
    ObjectType,
    type Package,
    Installation,
    installation,
    Observation,
    PackageMount,
    SpaceMount,
    SYNCED,
} from "@destack/object";
import { ObjectStore, type PackageImplementation } from "@destack/object/server";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { PackageVersion, PackageId, type PackageRelease } from "@destack/package";
import settingPackage from "@destack/setting/package";
import { describeSetting } from "@destack/setting/inspect";
import { buildRetention } from "@destack/stack/setting";
import {
    Digest,
    Identifier,
    type JsonObject,
    type VersionRange,
    type JsonValue,
    present,
    schema,
} from "@destack/schema";
import objectPackage from "@destack/object/package";
import { Simulation } from "@destack/environment/simulation";
import { StackDocument, type StackRecordType } from "@destack/stack/declare";
import { Handoff } from "../transfer/handoff.ts";
import * as object from "@destack/stack/object";
import { SocketRegistry, type InstallationBuild } from "../index.ts";
import { Machine } from "../machine/machine.ts";
import { Space } from "../space/space.ts";
import { type StoreProvider, type SpaceProvider } from "@destack/stack/object";
import { Control } from "@destack/control";
import {
    MachineWorld,
    TestMachine,
    TestControl,
    type TestControlOptions,
    ControlWorld,
    UNIVERSE_NODE,
} from "@destack/control/test";
import type { TestPerson } from "@destack/account/test";
import type { ClientOf } from "@destack/object/client";
import stackPackage, { PUSH_PATH } from "@destack/stack/package";
import { lockBuild, lockOf, TestRegistry } from "./build.ts";
import { TestSpaceStorage } from "./storage.ts";
import { TestGateway } from "./gateway.ts";
import { TestRuntime } from "./runtime.ts";
import { ProcessRuntime, type Runtime } from "../runtime/index.ts";
import { insertBuildScope, insertRelease } from "./install.ts";
import { BunGraphStorage } from "../computer/graph.ts";
import { PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import type { StackRelease } from "@destack/account/object";
import { BuildObject, PackageClient, BuildStore } from "@destack/build/store";
import { TestBuildStore } from "@destack/build/test";
import { RolloutTarget } from "../installation/target.ts";
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
    /** The digest of the empty stack's release's lock. */
    readonly sourceLock: Digest;
    /** The digest of the stack package's release's lock. */
    readonly lock: Digest;
}

/** The residency a test universe declares, whose home a residency fixture hosts. */
const RESIDENCY: ResidencyCode = "eu";

/** The directory below a template's files keeping the registry's builds, beside each space's. */
const REGISTRY_DIRECTORY = "registry";

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

/** Describe a space world's state outside its databases: keys, files, stack and its own world. */
function spaceWorldOf<World extends JsonValue>(world: schema.Schema<World>) {
    return schema.object({
        /** The universe's keys and its people's sign-ins. */
        universe: ControlWorld,
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
                /** The releases the registry published, published again on restore. */
                releases: schema.array(Digest),
                /** The linked release of each server's package the write installed, by package. */
                linked: schema.array(schema.object({ packageId: PackageId, lock: Digest })),
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
    readonly universe: typeof Control.database;
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
};

/** The fixture's account, which its spaces belong to. */
const FIXTURE_ACCOUNT = { id: ids.account, handle: "acme" } as const;

/** The fixture's space, which a world hosts alone unless it lists others. */
const FIXTURE_SPACE: WorldSpace = { id: ids.space, account: ids.account, name: "space" };

/** A space of a test universe, hosted on its residency's cloud machine over a test database. */
export class TestSpace implements AsyncDisposable, TestScope {
    /** The hosted space. */
    readonly id: Identifier<"space">;
    /** The account the space belongs to. */
    readonly account: Identifier<"account">;
    /** The universe the space lives in. */
    readonly universe: TestControl;
    /** The space's owner, signed up at the universe. */
    readonly owner: TestPerson;
    /** The machine hosting the space, by its identifier in the world's machines. */
    readonly #machineId: Identifier<"machine">;
    /** The world the universe, the machine and the space run in, which tests move through. */
    readonly simulation: Simulation;
    /** The registry the space's releases come from. */
    readonly registry: TestRegistry;
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
                "isBuilding" | "isEnforcingEntitlements" | "runtimes" | "destack"
            >,
        ) => Promise<Machine>
    >;
    /** The failures the world captured, which its disposal refuses untaken. */
    readonly telemetry: TestTelemetry;
    /** The servers the world's spaces run in process, which its machines load by linked lock. */
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
        readonly universe: TestControl;
        readonly owner: TestPerson;
        readonly machine: Identifier<"machine">;
        readonly simulation: Simulation;
        readonly registry: TestRegistry;
        readonly gateway: TestGateway;
        readonly machines: Map<Identifier<"machine">, Machine>;
        readonly starts: Map<
            Identifier<"machine">,
            (
                changes: Pick<
                    TestSpaceOptions,
                    "isBuilding" | "isEnforcingEntitlements" | "runtimes" | "destack"
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
        this.simulation = fields.simulation;
        this.registry = fields.registry;
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
        const snapshot = Snapshot.live(universe.storage.database);
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

        // wait until the machine serves the home, born as at its first start
        return await fixture.serve();
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
            databases: { universe: Control.database, machine: TestMachine.database },
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
            files: directory,
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
            // create another space as the owner, and its account at its first space, the fixture's as acme
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
            .map((release) => ({ packageId: release.package.id, lock: release.lock }));

        return {
            id: this.id,
            account: this.account,
            stack: this.#stack,
            releases: [...this.registry.locks],
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
        const fixture = await TestSpace.#host(world, owner, { id, account }, options, {
            machine,
            defer,
            ...(lifetime.files === undefined ? {} : { directory: lifetime.files }),
            ...(lifetime.create === undefined ? {} : { create: lifetime.create }),
        });

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
            { storage: databases.universe, world: value.universe, files: directory },
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

        // hold each space's written database on the machine before it follows
        const held = new Map(
            [...copy.volumes].map(([id, database]) => [
                schema.identifier("space").parse(id),
                database,
            ]),
        );
        const linked = value.spaces.flatMap((state) =>
            state.linked.flatMap(({ packageId, lock }) => {
                const server = (options.servers ?? []).find(
                    (each) => each.package.id === packageId,
                );

                return server === undefined
                    ? []
                    : [{ package: PackageVersion.of(server.package), lock }];
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
            await fixture.#restoreState(state);
            spaces.set(state.id, await fixture.serve());
        }
        const first = present(spaces.values().next().value, "the first space");

        return { space: first, spaces, world: value.world };
    }

    /** Publish a write's releases again, and store the stack and servers it installed. */
    async #restoreState(state: SpaceWorld<null>["spaces"][number]): Promise<void> {
        // record the releases the write published again, from the registry's builds it kept
        for (const lock of state.releases) {
            await this.registry.push(await this.builds(), lock);
        }

        // store the stack the write applied
        this.#stack = state.stack;
    }

    /** Create the fixture's account as its owner, answering the owner's account client. */
    static async account(
        universe: TestControl,
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

        // follow the space stack at its release, each build identified by its lock alone
        const source = {
            ...STACKS.space,
            selection: { kind: "release" as const, version: STACK_VERSION },
        };

        return {
            space: empty,
            stack,
            source,
            sourceLock: await lockOf(empty, crypto),
            lock: await lockOf(stack, crypto),
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
            readonly world?: ControlWorld;
            /** The files directory keeping the registry's builds, a temporary store by default. */
            readonly files?: string;
        } = {},
    ): Promise<TestSpaceWorld> {
        // join the given world
        if (options.world !== undefined) {
            return options.world;
        }

        // open the universe opening releases from the world's registry
        const { storage, world, files } = stored;
        const environment = simulation.environment(UNIVERSE_NODE);
        const { source } = await TestSpace.releases(environment.crypto);
        const packages =
            files === undefined
                ? await TestBuildStore.create(environment)
                : await TestBuildStore.open(join(files, REGISTRY_DIRECTORY), environment);
        defer(() => packages[Symbol.asyncDispose]());
        const registry = new TestRegistry(packages, ids.package);
        const universe = await TestControl.open({
            stacks: { home: source, space: source },
            home: source,
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
        const telemetry = TestTelemetry.of(simulation.telemetry);
        defer(() => telemetry[Symbol.dispose]());

        return {
            simulation,
            universe,
            registry,
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
        const { simulation, registry, universe } = world;
        const environment = simulation.environment(UNIVERSE_NODE);
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
        const builds = await TestBuildStore.open(join(directory, "builds"), environment);
        defer(async () => {
            await buckets[Symbol.asyncDispose]();
            await builds[Symbol.asyncDispose]();
            if (lifetime.directory === undefined) {
                await files.remove(directory, { recursive: true });
            }
        });

        // open the volumes over the main database, with the machine's resources' providers
        const storage = simulation.environment(test.id);
        const { dialect } = universe.storage.database;
        const local = options.resources?.(join(directory, RESOURCES), environment);
        const volumes = new TestSpaceStorage({
            main: TestSpace.#base(options),
            replicated: SpaceStorage.replicas({ follows: options.follows ?? [] }),
            databases: lifetime.databases ?? new Map(),
            create:
                lifetime.create ??
                (() =>
                    TestDatabase.create(TestSpace.#base(options), {
                        environment: storage,
                        dialect,
                        isMigrated: true,
                    })),
            objects: TestSpaceStorage.objects(options.packages ?? []),
            environment: storage,
            files: { directory, buckets },
            // store each space's databases as SQLite files
            providers: [
                ...(options.providers ?? []),
                ...(local?.providers ?? []),
                databaseProvider(
                    new BunSqliteDatabaseHost(pathToFileURL(`${databases}/`), environment),
                    object.database,
                ),
                TestSpaceDatabaseHost.over(main, environment, (space) => volumes.database(space)),
            ],
            connectors: [...(options.connectors ?? []), ...(local?.connectors ?? [])],
        });

        // build the machine as the residency's cloud machine
        const stack = await TestSpace.releases(environment.crypto);

        // link the stack package, the restored servers' releases and the builds the test ships, before any start
        const shipped: PackageRelease[] = [];
        for (const build of options.linked ?? []) {
            await registry.release(build);
            const released = PackageManifest.package(build.manifest);
            shipped.push({
                package: released,
                lock: await registry.find(released.id, released.version),
            });
        }
        const linked = new Map<PackageId, PackageRelease>(
            [
                { package: PackageVersion.of(stackPackage), lock: stack.lock },
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
                "isBuilding" | "isEnforcingEntitlements" | "runtimes" | "destack"
            >,
        ): Promise<Machine> => {
            // build the machine over the options as changed
            const changed = { ...options, ...changes };
            const machine: Machine = new Machine({
                id: test.id,
                environment: simulation.environment(test.id),
                identity: test.identity,
                keyring: test.keyring,
                database: test.database,
                enrollment: async () => {
                    // read how the universe enrolled the machine: under an account, or as the residency's
                    const row = await enrolledAs();
                    const enrolled = { universe: universe.origin, status: "enrolled" } as const;

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
                releases: registry,
                isBuilding: changed.isBuilding ?? true,
                isEnforcingEntitlements: changed.isEnforcingEntitlements ?? false,
                isServingHostnames: options.isServingHostnames ?? true,
                domain: "localhost",
                runtimes: changed.runtimes?.({ scheduler: simulation.scheduler, builds }) ?? [
                    new TestRuntime({ scheduler: simulation.scheduler, builds }),
                    new ProcessRuntime(),
                ],
                hibernation: undefined,
                alarm: undefined,
                server: async (lock) =>
                    world.servers.find(
                        (server) => machine.linked.get(server.package.id)?.lock === lock,
                    ),
                linked: world.machines.get(test.id)?.linked ?? linked,
                shipped: [],
                skus: undefined,
                endpoint:
                    options.origin ?? (await test.directory.answering(test.id, test.clock.now())),
                registry: FORGE_INSTALLATION,
                destack: changed.destack ?? { oldest: DESTACK_VERSION, newest: DESTACK_VERSION },
                authenticate: (request, audience, partition) =>
                    TestSpace.#authenticate(machine, request, audience, partition),
                authorizeSpace: async () => {},
            });

            // run it as the world's machine
            world.machines.set(test.id, machine);

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
        const { simulation, universe, registry, gateway } = world;
        for (const server of options.servers ?? []) {
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
        gateway.serve(machine);
        registry.serve(
            Registry.of(
                machine.directory,
                async (account) => (await machine.accounts()).residency.locate({ account }),
                (audience, partition) => machine.fetch(audience, { partition }),
                machine.environment,
                machine.registry,
            ),
        );

        // release and link the builds the space ships on a machine that started without them
        for (const build of options.linked ?? []) {
            const released = PackageManifest.package(build.manifest);
            if (machine.linked.get(released.id) === undefined) {
                await registry.release(build);
                machine.link({
                    package: released,
                    lock: await registry.find(released.id, released.version),
                });
            }
        }

        // hold the fixture, closed with its lifetime
        const fixture = new TestSpace({
            ...address,
            universe,
            owner,
            machine: machine.id,
            simulation,
            registry,
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

        // publish the space's releases, and ship the machine's builds of its servers' packages
        for (const release of [
            fixture.releases.space,
            fixture.releases.stack,
            ...(options.releases ?? []),
        ]) {
            await registry.release(release);
        }

        return fixture;
    }

    /** Link a build as its package's release on the machine, as a newer distribution does. */
    async link(build: BuildReader): Promise<void> {
        // publish the build, then link its lock on the machine
        await this.registry.release(build);
        const released = PackageManifest.package(build.manifest);
        const lock = await this.registry.find(released.id, released.version);
        this.machine.link({ package: released, lock });
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
            "isBuilding" | "isEnforcingEntitlements" | "runtimes" | "destack"
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

    /** Read the installed set main applied, each at its revision and the lock it runs. */
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
            const [row] = await Snapshot.live(this.database)
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

    /** Push a running build's files and code index as its runner, answering the files uploaded. */
    async storeBuild(
        runner: TestCaller,
        target: { readonly spaceId: Identifier<"space">; readonly id: Identifier<"build"> },
        reader: BuildReader,
    ): Promise<readonly Digest[]> {
        // report storing, then push the build to the space's builds mount, recording each file uploaded
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
                // record the objects the space lacks, which the push writes next
                const asked = request.clone();
                const answer = await fetch(request);
                if (new URL(request.url).pathname.endsWith("/has")) {
                    const digests = schema.array(Digest).parse(await asked.json());
                    const stored = schema
                        .array(schema.boolean())
                        .parse(await answer.clone().json());
                    uploaded.push(...digests.filter((_digest, index) => stored[index] !== true));
                }

                return answer;
            },
        );
        await using store = await TestBuildStore.create(this.machine.environment);
        const root = await store.put(reader);
        const lock = await store.putLock(Lock.of(root));
        await pushing.push(store, [lock]);

        // upload an empty code index with it
        const bytes = new TextEncoder().encode(JSON.stringify([]));
        const code = {
            digest: await Digest.of(bytes, this.machine.environment.crypto),
            size: bytes.byteLength,
        };
        await pushing.write(BuildObject.files([{ digest: code.digest, bytes: bytes }]));
        await client.build.finish({
            ...target,
            requestId: RequestId.create(this.machine.environment),
            end: { kind: "stored", lock, code },
        });

        return uploaded;
    }

    /** Describe the revision a submission of a build records for an installation. */
    async describeRevision(
        target: Pick<Installation, "name" | "capabilities" | "allowed">,
        reader: BuildReader,
    ): Promise<Pick<object.InstallationRevision, "dependencies" | "states">> {
        // read the store's types and the options a submission describes with
        const space = await this.space;
        const store = await space.store();
        const options = { space, registry: this.registry, blobs: space.blobs, known: KNOWN_TYPES };

        // lock the build alone in the space's store
        const { lock } = await this.#lock(reader, []);

        return RolloutTarget.describe(
            target,
            await space.builds.lock(lock),
            store.objects,
            options,
        );
    }

    /** Store a build locked with the builds it runs with, answering its lock and root. */
    /** Store a build locked with the builds it runs with, answering its lock and root. */
    async #lock(
        reader: BuildReader,
        builds: readonly BuildReader[],
    ): Promise<{ readonly lock: Digest; readonly root: Digest }> {
        // put each build and their lock in the space's store
        const stored = await this.builds();

        return await lockBuild(stored, reader, builds);
    }

    /** Store a build locked with the builds it runs with, and record it succeeded now. */
    async recordBuild(
        reader: BuildReader,
        index: object.CodeIndex = { code: [], modules: [] },
        builds: readonly BuildReader[] = [],
    ): Promise<{ readonly kind: "build"; readonly build: Identifier<"build"> }> {
        // put the build and its lock in the build store
        const { database } = this;
        const packages = await this.builds();
        const { lock, root } = await this.#lock(reader, builds);
        const bytes = new TextEncoder().encode(JSON.stringify(index));
        const indexed = {
            digest: await Digest.of(bytes, this.machine.environment.crypto),
            size: bytes.byteLength,
        };
        await packages.write(BuildObject.files([{ digest: indexed.digest, bytes: bytes }]));
        await packages.attach(root, object.CodeIndex.file(indexed));

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
                    lock,
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

    /** Record a build of the space's stack declaring a definition, locked with its releases. */
    async recordStack(
        exported: string,
        definition: JsonObject,
    ): Promise<{ readonly kind: "build"; readonly build: Identifier<"build"> }> {
        // pull each installed release's lock
        const { installations } = StackDefinition.collections(definition);
        const released = Object.values(installations).flatMap(
            ({ packageId, selection, version }) => {
                // lock a pinned release, else the release it was built with
                const pinned = selection?.kind === "release" ? selection.version : undefined;
                const built = selection === undefined ? version : undefined;
                const locked = pinned ?? built;

                return locked === undefined ? [] : [{ packageId, version: locked }];
            },
        );
        const builds = await this.builds();
        const requires: Record<string, string> = {};
        const locked: BuildReader[] = [];
        for (const { packageId, version } of released) {
            const lock = await (
                await this.space
            ).pull(packageId, await this.registry.find(packageId, version));
            const root = await builds.contents(lock.root);
            requires[root.manifest.package.name] = version;
            for (const build of Lock.builds(lock)) {
                locked.push(await builds.contents(build));
            }
        }

        // declare the definition at the space stack's export
        const reader = await MemoryBuild.of(
            PackageManifest.package(this.releases.space.manifest),
            [{ ...STACK_KIND.identity, name: exported, description: definition }],
            this.machine.environment.crypto,
            { requires },
        );

        return await this.recordBuild(reader, undefined, locked);
    }

    /** Put a package's fixture release in the build store, as the space's build of it. */
    async release(
        packageId: PackageId,
        version: string,
    ): Promise<{ readonly id: Identifier<"build">; readonly lock: Digest }> {
        // replicate the registry's release into the build store, and record it as the space's build
        const lock = await this.registry.find(packageId, version);
        const reader = await (await this.space).root(packageId, lock);
        const release = { package: packageId, version, lock };

        return insertRelease(
            this.database,
            this.id,
            release,
            Object.keys(reader.manifest.outputs),
            this.machine.environment,
        );
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
            type: (reference) => this.type(reference),
            named: (name) => this.named(name),
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
        reference: ObjectTypeReference,
    ): { readonly type: ObjectType; readonly handle: Package } | undefined {
        const key = ObjectTypeReference.key(reference);

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
        const recorded = await this.recordStack(STACKS.space.export, this.#stack);

        // follow the build declaring the definition, which the space evaluates and applies
        await this.simulation.idle();
        const installed = this.as(this.owner.subject).client(objectPackage);
        const listed = await installed.installation.list({ scope: this.id });
        const stack = single(listed.items.filter((entry) => entry.export !== null));
        await installed.installation.update({
            scope: this.id,
            id: stack.id,
            requestId: RequestId.create(this.machine.environment),
            revision: stack.revision,
            ...ObjectReference.columns("follows", object.build.reference(this.id, recorded.build)),
            directory: null,
        });

        // approve a plan waiting for its approval as the owner
        await this.simulation.idle();
        let failures = this.#takeFailures();
        const submitted = await installed.installation.get({ scope: this.id, id: stack.id });
        const owner = this.as(this.owner.subject).client(stackPackage);
        if (submitted.conditions["Ready"]?.reason === AWAITING_APPROVAL) {
            const { digest } = await owner.rollout.plan({ spaceId: this.id, id: stack.id });
            await owner.rollout.approve({
                spaceId: this.id,
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
        // publish the build as its package's release
        await this.registry.release(build);
        const released = PackageManifest.package(build.manifest);

        // declare it after the earlier installations
        const name = released.name.slice(released.name.lastIndexOf("/") + 1);
        const declared = {
            packageId: released.id,
            selection:
                options.isolation === "host"
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
            .client(objectPackage)
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

/** The world spaces share: simulation, universe, registry, gateway and machines. */
export type TestSpaceWorld = Pick<
    TestSpace,
    | "simulation"
    | "universe"
    | "registry"
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
    readonly isBuilding?: boolean;
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
    /** The policies a new universe decides with its own, such as the forge's, none by default. */
    readonly policies?: TestControlOptions["policies"];
    /** The residencies a new universe declares, Europe alone by default. */
    readonly residencies?: TestControlOptions["residencies"];
    /** The handles a new universe's platform publishes under, none by default. */
    readonly publishers?: TestControlOptions["publishers"];
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
            const rows = await Snapshot.live(store.database)
                .read(controller.table)
                .findMany({ where: key });
            await controller.reconcile(rows, reconciliation);
        } else if (projection !== undefined) {
            await projection.project(
                present(key[projection.key], `the ${projection.key} key`),
                reconciliation,
            );
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
