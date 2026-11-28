import { testHost } from "@destack/test/host";
import { NpmMirror } from "./npm.ts";
import { installation, Installation } from "@destack/stack/object";
import {
    type InstanceOf,
    type ObjectType,
    type Package,
    PackageMount,
    Status,
} from "@destack/object";
import {
    type Environment,
    type FileSystem,
    FileSystemError,
    type ServerSocket,
} from "@destack/environment";
import { type CapturedRecord, TestTelemetry } from "@destack/telemetry/test";
import { Refusal } from "../../test/refusal.ts";
import { basename, dirname, join } from "node:path";
import { Account, type RegionCode } from "@destack/account/object";
import type { StepUp } from "@destack/access";
import type { Login } from "@destack/account/native";
import { freshHandle, type TestPerson } from "@destack/account/test";
import type { DatabaseConnection } from "@destack/db/database";
import { TestTemplate, type TestDatabase, TestRun } from "@destack/db/test";
import { Simulation, SimulatedClock } from "@destack/environment/simulation";
import { MemoryKeychain } from "@destack/identity";
import { createClient } from "@destack/object/client";
import { type Digest, Identifier, nth, schema, present } from "@destack/schema";
import type { HostListening, HostSocketAttachment, SocketRefusal } from "@destack/object/workload";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import stackPackage from "@destack/stack/package";
import { RequestId, Scope, ObjectReference } from "@destack/sync";
import { InstallationTarget } from "../../installation/target.ts";
import type { PackageId } from "@destack/package";
import { inject, onTestFinished } from "@destack/test";
import { Universe } from "@destack/universe";
import { TEST_UNIVERSE, TestUniverse, UniverseWorld } from "@destack/universe/test";
import { TUNNEL_PATH } from "@destack/tunnel";
import type { MachineEnrollment } from "../device/object/index.ts";
import machinePackage from "../../package.ts";
import { Computer, type ComputerOptions, MACHINE_KEYS } from "../computer.ts";
import { LoopbackEndpoint, ENDPOINT_FILE, ENDPOINT_MODE } from "../process/index.ts";
import { TestDistribution, type TestPublisher } from "./distribution.ts";
import { Toolchain } from "@destack/check/toolchain";

/** When every simulation starts: 16 October 2026, UTC. */
const START = Date.UTC(2026, 9, 16);

/** The seed of every simulation's randomness. */
const SEED = 1;

/** The email the computers' owner signs up with. */
export const OWNER_EMAIL = "owner@example.test";

/** The board fixture installed as `work` in the owner's home. */
export const WORK_BOARD: readonly TestInstall[] = [{ fixture: "board", name: "work" }];

/** The node the owner's browser runs as in the simulated network. */
const BROWSER_NODE = "browser";

/** The node of the computers' state directories, over the host's files. */
export const FILES_NODE = "files";

/** The port HTTPS origins answer at, which the universe's origin leaves implicit. */
const HTTPS_PORT = 443;

/** The node the region's relay runs as in the simulated network. */
export const RELAY_NODE = "relay";

/** The region the owner and the cloud machine live in. */
const REGION: RegionCode = "eu";

/** The characters of a machine identifier a test name uses: its UUID's last group. */
const NAME_SUFFIX_LENGTH = 12;

/** How long a computer waits for rows to arrive: 10 s, far above their milliseconds. */
const RECEIVE_MILLISECONDS = 10_000;

/** The wait for a space to get ready: 30 s, nine restarts of about a second, tripled under load. */
const READY_MILLISECONDS = 30_000;

/** How long an install may take to apply: 2 min, as a space pulls an app's closure in ~30 s. */
const INSTALL_MILLISECONDS = 120_000;

/** What a test computer's resources live as long as: the test, or a template's write. */
type Defer = (cleanup: () => Promise<void> | void) => void;

/** The world the universe template wrote: the universe's keys. */
const UniverseTemplate = schema.object({
    /** The universe's keys and its people's sign-ins. */
    universe: UniverseWorld,
});
/** The world the universe template wrote. */
type UniverseTemplate = schema.Infer<typeof UniverseTemplate>;

/** The world a home template wrote: the universe, the cloud computer's state, the login. */
const HomeWorld = schema.object({
    /** The universe's keys and its people's sign-ins. */
    universe: UniverseWorld,
    /** The cloud computer's state directory in the run's directory. */
    directory: schema.string(),
    /** The secrets of the cloud computer's keychains, by service and name. */
    keychains: schema.record(schema.string(), schema.record(schema.string(), schema.string())),
    /** The owner's login on the cloud computer's device. */
    login: schema.identifier("login"),
    /** The accounts of the organizations the owner created, by handle. */
    organizations: schema.record(schema.string(), schema.identifier("account")),
});
/** The world a home template wrote. */
type HomeWorld = schema.Infer<typeof HomeWorld>;

/** An organization the owner creates in a home's template: its handle and its name. */
export interface TestOrganization {
    /** The organization's handle. */
    readonly handle: string;
    /** The organization's name. */
    readonly name: string;
}

/** The organization publishing the tests' packages, which a home's template creates on request. */
export const PUBLISHER: TestOrganization = { handle: "example", name: "Example" };

/** What the owner's home starts with beside the cloud computer's options. */
export interface TestHomeOptions extends Omit<
    TestComputerOptions,
    "region" | "directory" | "keychains"
> {
    /** The organizations the owner created, each serving its home, none by default. */
    readonly organizations?: readonly TestOrganization[];
    /** The fixtures installed in the owner's home under their names, none by default. */
    readonly installs?: readonly TestInstall[];
    /** The fixtures the owner publishes for the organization their names give, none by default. */
    readonly published?: readonly string[];
}

/** A fixture a home template publishes and installs in the owner's home under a name. */
export interface TestInstall {
    /** The fixture's directory name below `tests/fixture`, such as `board`. */
    readonly fixture: string;
    /** The installation's name, such as `work`. */
    readonly name: string;
}

/** A test universe on a simulation's network, with what its computers ship and its owner. */
export interface TestWorld {
    /** The simulation whose clock and network every node runs on. */
    readonly simulation: Simulation;
    /** The universe, answering at its origin on the simulated network. */
    readonly universe: TestUniverse;
    /** What the computers ship. */
    readonly distribution: TestDistribution;
    /** The owner, signed up at the universe. */
    readonly owner: TestPerson;
    /** The exceptions the world's nodes captured, which a test takes or fails on. */
    readonly telemetry: TestTelemetry;
}

/** What a test computer runs with beside its world. */
export interface TestComputerOptions {
    /** The node the computer runs as on the simulated network, a new one by default. */
    readonly node?: string;
    /** The region whose cloud machine the computer runs, a personal machine by default. */
    readonly region?: RegionCode;
    /** The relay the computer tunnels through once enrolled, none by default. */
    readonly relay?: string;
    /** Run the machine's instances through an egress, as Bun processes by default. */
    readonly runtime?: ComputerOptions["runtime"];
    /** The installed distribution the machine stages and activates, none by default. */
    readonly distribution?: ComputerOptions["distribution"];
    /** The native device's local objects beside the machine's own. */
    readonly localObjects?: Readonly<Record<string, ObjectType>>;
    /** The state directory to start over, a fresh one by default. */
    readonly directory?: string;
    /** The secrets of the keychains to start with, by service and name, none by default. */
    readonly keychains?: Readonly<Record<string, Readonly<Record<string, string>>>>;
}

/** The region's relay a test reaches: its tunnel's URL, and the managed machine hosting it. */
export interface TestRegionRelay {
    /** The URL self-hosted machines open their tunnels at. */
    readonly url: string;
    /** The region's managed machine hosting the relay. */
    readonly computer: TestComputer;
}

/** A Destack computer as its process composes it, in a test universe on a simulated network. */
export class TestComputer implements AsyncDisposable {
    /** The world the computer runs in. */
    readonly world: TestWorld;
    /** The options the computer was opened with, which a restart opens it with again. */
    readonly #options: TestComputerOptions;
    /** The computer. */
    readonly computer: Computer;
    /** The computer's node in the world, its process included. */
    readonly environment: Environment<"files" | "process" | "host" | "server">;
    /** The name of the computer's node on the simulated network. */
    readonly node: string;
    /** The computer's state directory. */
    readonly directory: string;
    /** The origin the computer answers at on the simulated network. */
    readonly endpoint: string;
    /** The computer's keychains, by service. */
    readonly keychains: Map<string, MemoryKeychain>;
    /** Stop the computer and its listener. */
    readonly #stack: AsyncDisposableStack;

    /** Take a started computer. */
    private constructor(fields: {
        readonly world: TestWorld;
        readonly options: TestComputerOptions;
        readonly computer: Computer;
        readonly environment: Environment<"files" | "process" | "host" | "server">;
        readonly node: string;
        readonly directory: string;
        readonly endpoint: string;
        readonly keychains: Map<string, MemoryKeychain>;
        readonly stack: AsyncDisposableStack;
    }) {
        // set the computer, its world and its state
        this.world = fields.world;
        this.#options = fields.options;
        this.computer = fields.computer;
        this.environment = fields.environment;
        this.node = fields.node;
        this.directory = fields.directory;
        this.endpoint = fields.endpoint;
        this.keychains = fields.keychains;
        this.#stack = fields.stack;
    }

    /** List the failures the world captured since the last take, but the tunnel's retried dials. */
    captured(): CapturedRecord[] {
        return Refusal.captured(this.world.telemetry, (record) => TestComputer.#isDial(record));
    }

    /** Take the failures the world captured as refusals, but the tunnel's retried dials. */
    takeRefusals(): Refusal[] {
        return Refusal.take(this.world.telemetry, (record) => TestComputer.#isDial(record));
    }

    /** Wait until the world captures a failure, but the tunnel's dials, answering the first. */
    async refused(): Promise<CapturedRecord> {
        await this.world.simulation.waitUntil(() => this.captured().length > 0);

        return nth(this.captured(), 0);
    }

    /** Report whether a captured failure is a tunnel's dial, which the tunnel retries. */
    static #isDial(record: CapturedRecord): boolean {
        return record.attributes["tag.event"] === "tunnel.failed";
    }

    /** The machine the computer runs. */
    get machine(): Computer["machine"] {
        return this.computer.machine;
    }

    /** The computer's native device. */
    get device(): Computer["device"] {
        return this.computer.device;
    }

    /** The clock every node of the world runs on. */
    get clock(): SimulatedClock {
        return this.world.simulation.clock;
    }

    /** Name a computer's node or machine by a prefix and its identifier's last group. */
    static nameOf(prefix: "computer" | "machine", id: Identifier<"machine">): string {
        return `${prefix}-${id.slice(-NAME_SUFFIX_LENGTH)}`;
    }

    /** Start a copy of the run's world, shipping the given builds too. */
    static async world(
        options: {
            /** The further builds the computers ship, such as a library a test builds against. */
            readonly bundled?: readonly BuildReader[];
        } = {},
        defer: Defer = onTestFinished,
    ): Promise<TestWorld> {
        // fork the template's world with its universe and owner, bridging host processes into it
        const image = await TestRun.image(TestComputer.#universe());
        const { simulation, databases, value } = await image.fork("copy", { isBridged: true });
        defer(() => databases.universe.close());
        const distribution = await TestDistribution.build(
            simulation.environment(FILES_NODE),
            options.bundled ?? [],
        );

        return await TestComputer.#start(
            distribution,
            simulation,
            { storage: databases.universe, world: value.universe },
            defer,
        );
    }

    /** Start the universe on a simulation, over a template's database and keys when given. */
    static async #start(
        distribution: TestDistribution,
        simulation: Simulation,
        restored: {
            readonly storage?: TestDatabase;
            readonly world?: UniverseWorld;
        },
        defer: Defer,
    ): Promise<TestWorld> {
        // refuse untaken exceptions
        defer(() => distribution[Symbol.asyncDispose]());
        const telemetry = TestTelemetry.of(simulation.telemetry);
        defer(() => telemetry[Symbol.dispose]());

        // answer as the public npm registry from the host's Bun cache, which host processes install from
        const host = testHost();
        const mirror = new NpmMirror(NpmMirror.cache(host.process.variables), host).serve(
            simulation,
        );
        defer(() => mirror[Symbol.dispose]());

        // start the universe on the simulation, answering at its origin on the network
        const universe = await TestUniverse.open({
            simulation,
            distribution: distribution.distribution,
            layer: distribution.layer,
            stacks: distribution.stacks,
            home: distribution.home,
            ...(restored.storage === undefined ? {} : { storage: restored.storage }),
            ...(restored.world === undefined ? {} : { world: restored.world }),
        });
        defer(() => universe[Symbol.asyncDispose]());

        // sign the owner up, or take the one the world signed in
        const owner =
            restored.world === undefined
                ? await universe.signIn(OWNER_EMAIL)
                : universe.person(OWNER_EMAIL);

        return { simulation, universe, distribution, owner, telemetry };
    }

    /** Describe the run's universe with its owner signed up, which the run writes once. */
    static #universe(): TestTemplate<
        UniverseTemplate,
        { readonly universe: typeof Universe.database }
    > {
        return new TestTemplate({
            name: "machine.universe",
            databases: { universe: Universe.database },
            value: UniverseTemplate,
            world: { seed: SEED, start: START },
            build: async ({ databases, simulation, variables }) => {
                // start the universe over the template's database
                await using lifetime = new AsyncDisposableStack();
                const distribution = await TestDistribution.build(
                    simulation.environment(FILES_NODE),
                    [],
                    variables,
                );
                const world = await TestComputer.#start(
                    distribution,
                    simulation,
                    { storage: databases.universe },
                    (cleanup) => {
                        lifetime.defer(cleanup);
                    },
                );

                return {
                    universe: await world.universe.world(),
                };
            },
        });
    }

    /** Start the region's managed machine, which hosts the region's relay. */
    static async relay(world: TestWorld, defer: Defer = onTestFinished): Promise<TestRegionRelay> {
        // host the relay on the region's managed machine, as its Worker does
        const computer = await TestComputer.open(
            { world, region: REGION, node: RELAY_NODE },
            defer,
        );

        return computer.relay();
    }

    /** Read the region's relay this managed computer hosts. */
    relay(): TestRegionRelay {
        const code = present(this.#options.region, "the region of a managed computer");
        const region = present(
            this.world.universe.universe.definition.regions.find((entry) => entry.code === code),
            `the region '${code}'`,
        );

        return { url: `${region.relay}${TUNNEL_PATH}`, computer: this };
    }

    /** List the templates the run writes once for the computers' worlds. */
    static templates() {
        return [TestComputer.#universe(), TestComputer.homeTemplate([])] as const;
    }

    /** Start a computer in a world, a new one by default, as its process composes it. */
    static async open(
        options: TestComputerOptions & { readonly world?: TestWorld } = {},
        defer: Defer = onTestFinished,
    ): Promise<TestComputer> {
        // start the world unless given
        const world = options.world ?? (await TestComputer.world({}, defer));
        const { simulation, universe, distribution } = world;

        // store the state in the given directory or a fresh one
        const directory =
            options.directory ??
            (await TestComputer.#directory(simulation.environment(FILES_NODE).files, defer));

        // store every keychain in memory, a cloud machine's key in the universe's
        const keychains = new Map(
            Object.entries(options.keychains ?? {}).map(
                ([service, secrets]) => [service, TestComputer.#keychain(secrets)] as const,
            ),
        );
        const cloud =
            options.region === undefined
                ? undefined
                : (await universe.world()).machines[options.region];
        if (cloud !== undefined) {
            keychains.set(MACHINE_KEYS, TestComputer.#keychain(cloud.secrets));
        }
        const keychain = (service: string) => {
            const found = keychains.get(service) ?? new MemoryKeychain();
            keychains.set(service, found);

            return found;
        };

        // listen on the simulated network as the computer's node, routing to the computer once it runs
        await using stack = new AsyncDisposableStack();
        const node =
            options.node ??
            TestComputer.nameOf(
                "computer",
                Identifier.create("machine", world.universe.environment),
            );
        const environment = simulation.environment(node);
        let listening: HostListening | undefined;
        const route = (request: Request) =>
            listening === undefined ? unavailable() : listening.fetch(request);
        const sockets = () => present(listening, "the computer's sockets").websocket;

        // publish the listener on the host as well for sandboxed runners, which are host processes, keeping the computer's sockets
        const serving = {
            hostname: "127.0.0.1",
            port: 0,
            fetch: route,
            websocket: {
                open: (socket: ServerSocket<HostSocketAttachment | SocketRefusal>) =>
                    sockets().open?.(socket),
                message: (
                    socket: ServerSocket<HostSocketAttachment | SocketRefusal>,
                    message: string | Uint8Array,
                ) => sockets().message(socket, message),
                close: (
                    socket: ServerSocket<HostSocketAttachment | SocketRefusal>,
                    code: number,
                    reason: string,
                ) => sockets().close?.(socket, code, reason),
            },
        };
        const listener =
            options.runtime === undefined
                ? simulation.network.publish(node, serving)
                : environment.server.http(serving);
        stack.defer(() => listener.stop(true));
        const port = Number(listener.url.port);

        // answer a cloud machine at the origin the universe declared it at as well
        const declared =
            cloud === undefined
                ? undefined
                : universe.universe.definition.platform.machines.find(
                      (machine) => machine.id === cloud.id,
                  )?.origin;
        if (cloud !== undefined && declared === undefined) {
            throw new TypeError(`the universe declares no origin of '${cloud.id}'`);
        }
        if (declared !== undefined) {
            // answer its origin, its region's relay and the names DNS sends it, as its Worker's routes do
            const region = present(
                universe.universe.definition.regions.find((entry) => entry.code === options.region),
                `the region '${String(options.region)}'`,
            );
            const { domains } = TEST_UNIVERSE;
            const hostnames = [
                new URL(declared).hostname,
                new URL(region.relay).hostname,
                `*.${domains.machine}`,
                `*.${domains.space}`,
            ];
            for (const hostname of hostnames) {
                const published = environment.server.http({
                    hostname,
                    port: HTTPS_PORT,
                    fetch: route,
                    websocket: serving.websocket,
                });
                stack.defer(() => published.stop(true));
            }

            // stand in front of the universe's origin as the platform's machine does, refusing upgrades
            stack.use(
                universe.serve(async (request) => {
                    const answer = await route(request);
                    if (!(answer instanceof Response)) {
                        throw new TypeError(
                            `the universe's origin cannot upgrade '${request.url}'`,
                        );
                    }

                    return answer;
                }),
            );
        }

        // start the computer, which captures the failures no caller sees in the world's telemetry
        const enrollment: MachineEnrollment | undefined =
            options.region === undefined
                ? undefined
                : {
                      kind: "managed",
                      accountId: Account.ofHome(await world.universe.platformHome()),
                      universe: TEST_UNIVERSE.toJSON(),
                      status: "enrolled",
                  };
        const computer = await Computer.start({
            // run instances in this process on the world's clock and network, deterministic under simulation
            isolation: "host",
            environment,
            directory,
            universe: universe.origin,
            port,
            ...(declared === undefined ? {} : { endpoint: declared }),
            ...(options.relay === undefined ? {} : { relay: { kind: "url", url: options.relay } }),
            localObjects: options.localObjects ?? {},
            keychain,
            toolchain: async () => Toolchain.workspace(),
            ...(cloud === undefined ? {} : { machineId: cloud.id }),
            process: {
                version: machinePackage.version,
                pid: environment.process.id,
                startedAt: simulation.clock.now(),
            },
            ...(options.runtime === undefined ? {} : { runtime: options.runtime }),
            bundled: distribution.manifest,
            layer: distribution.layer,
            ...(options.distribution === undefined ? {} : { distribution: options.distribution }),
            ...(enrollment === undefined ? {} : { enrollment }),
            shutdown: () => {},
        });
        stack.defer(() => computer[Symbol.asyncDispose]());
        listening = present(computer.machine.hibernation, "the computer's sockets").listen(
            (request) => computer.fetch(request),
        );

        // publish where local clients call the computer, as its process does
        const published: LoopbackEndpoint = {
            port,
            token: computer.authenticator.token,
            machine: computer.machine.id,
            device: computer.device.id,
        };
        await environment.files.writeTextFile(
            join(directory, ENDPOINT_FILE),
            JSON.stringify(published),
            {
                mode: ENDPOINT_MODE,
            },
        );

        // close the computer with the lifetime
        const started = new TestComputer({
            world,
            options,
            computer,
            environment,
            node,
            directory,
            endpoint: declared ?? `http://127.0.0.1:${port}`,
            keychains,
            stack: stack.move(),
        });
        defer(() => started[Symbol.asyncDispose]());

        return started;
    }

    /** Start the region's cloud computer serving the owner's home, from the run's template. */
    static async home(
        options: TestHomeOptions = {},
        defer: Defer = onTestFinished,
    ): Promise<{
        readonly home: TestComputer;
        readonly login: Identifier<"login">;
        readonly organizations: ReadonlyMap<string, Identifier<"account">>;
    }> {
        // fork the template of the owner's organizations with its universe, bridging host processes into it
        const { organizations = [], installs = [], published = [], ...started } = options;
        const template = TestComputer.homeTemplate(organizations, installs, published);
        const image = await TestRun.image(template);
        const { simulation, databases, value } = await image.fork("copy", { isBridged: true });
        defer(() => databases.universe.close());
        const distribution = await TestDistribution.build(simulation.environment(FILES_NODE));
        const world = await TestComputer.#start(
            distribution,
            simulation,
            { storage: databases.universe, world: value.universe },
            defer,
        );

        // copy the cloud computer's state, and start it over the copy, serving the owner's home
        const { files } = simulation.environment(FILES_NODE);
        const directory = await TestComputer.#directory(files, defer);
        await files.copy(value.directory, directory);
        const home = await TestComputer.open(
            {
                ...started,
                world,
                region: REGION,
                directory,
                keychains: value.keychains,
            },
            defer,
        );
        await home.ready(world.owner.home);

        return {
            home,
            login: value.login,
            organizations: new Map(Object.entries(value.organizations)),
        };
    }

    /** Describe the world of the owner's home and organizations, which the run writes once. */
    static homeTemplate(
        given: readonly TestOrganization[],
        installs: readonly TestInstall[] = [],
        published: readonly string[] = [],
    ): TestTemplate<HomeWorld, { readonly universe: typeof Universe.database }> {
        // create the organization publishing the fixtures in every world, beside the given ones
        const organizations = [
            PUBLISHER,
            ...given.filter((organization) => organization.handle !== PUBLISHER.handle),
        ];

        // name the template by what its world holds
        const handles = organizations.map((organization) => organization.handle);
        const installed = installs.map((install) => `${install.fixture}@${install.name}`);
        const released = published.map((fixture) => `${fixture}@latest`);

        return new TestTemplate({
            name: ["machine.home", ...handles, ...installed, ...released].join("."),
            databases: { universe: Universe.database },
            value: HomeWorld,
            world: { seed: SEED, start: START },
            build: async ({ databases, directory, simulation, variables }) => {
                // start the world over the template's database, closed once built
                await using lifetime = new AsyncDisposableStack();
                const defer: Defer = (cleanup) => {
                    lifetime.defer(cleanup);
                };
                const distribution = await TestDistribution.build(
                    simulation.environment(FILES_NODE),
                    [],
                    variables,
                );
                const world = await TestComputer.#start(
                    distribution,
                    simulation,
                    { storage: databases.universe },
                    defer,
                );

                // serve the owner's home on the cloud computer, the owner signed in on its device
                const state = join(directory, "home");
                await simulation.environment(FILES_NODE).files.mkdir(state);
                const home = await TestComputer.open(
                    { world, region: REGION, directory: state },
                    defer,
                );
                const login = await home.signIn(OWNER_EMAIL);
                await home.ready(world.owner.home);

                // serve the region's home and each organization's, their stacks applied
                const { owner } = world;
                await home.ready(Account.home(owner.account));
                const created: Record<string, Identifier<"account">> = {};
                for (const { handle, name } of organizations) {
                    const accountId = await owner.organize(handle, name);
                    created[handle] = accountId;
                    await home.ready(Account.home(accountId));
                }

                // publish each fixture the host lacks, then install it in the owner's home
                const sent = new Set<string>();
                for (const { fixture, name } of installs) {
                    const build = await TestDistribution.fixture(
                        fixture,
                        simulation.environment(FILES_NODE),
                        defer,
                    );
                    const { id } = PackageManifest.package(build.manifest);
                    if (!sent.has(fixture) && !home.machine.bundled.has(id)) {
                        sent.add(fixture);
                        await home.publish(TestDistribution.imageOf(fixture), owner);
                    }
                    await home.install(login.id, owner.home, build, name);
                }

                // publish each other fixture as the owner, for the organization its name gives
                for (const fixture of published) {
                    await home.publish(TestDistribution.imageOf(fixture), owner);
                }
                await home.close();

                // record the state, the keychains and the universe's keys
                const keychains = home.secrets();

                return {
                    universe: await world.universe.world(),
                    directory: state,
                    keychains,
                    login: login.id,
                    organizations: created,
                };
            },
        });
    }

    /** Read the secrets of the computer's keychains by service, as a restart starts with them. */
    secrets(): Record<string, Record<string, string>> {
        return Object.fromEntries(
            [...this.keychains].map(([service, keychain]) => [
                service,
                Object.fromEntries(keychain.secrets),
            ]),
        );
    }

    /** Open a keychain of the computer's by service, as the system stores it, empty when new. */
    keychain(service: string): MemoryKeychain {
        const keychain = this.keychains.get(service) ?? new MemoryKeychain();
        this.keychains.set(service, keychain);

        return keychain;
    }

    /** Connect to one of the computer's packages below its mount as its owner. */
    connect<Target extends Package>(target: Target) {
        return createClient<Target["router"]>(
            target,
            {
                url: PackageMount.url(this.endpoint, target.id),
                headers: { authorization: `Bearer ${this.computer.authenticator.token}` },
                fetch: (request: Request) => this.computer.fetch(request),
            },
            this.device.environment,
        );
    }

    /** Sign a person in on the computer's device with the code the universe mails them. */
    async signIn(email: string): Promise<Login["row"]> {
        // mail the code, signing a new person up in the region
        const started = await this.device.credential.startEmail(email, {
            name: email,
            handle: freshHandle(this.environment),
            regionId: REGION,
            timeZone: "UTC",
            locale: "en",
        });

        // redeem the last code mailed to them
        const codes = await this.world.universe.mail.codes();
        const code = codes.findLast((mailed) => mailed.email === email);
        if (code === undefined) {
            throw new TypeError(`the universe mailed no code to '${email}'`);
        }

        return started.verify(code.otp);
    }

    /** Send a request as the owner's browser over the world's network, as a launch opens. */
    browse(request: Request): Promise<Response> {
        return this.world.simulation.environment(BROWSER_NODE).network.fetch(request);
    }

    /** Sign the owner in again on the device in the browser, stepped up with an authenticator. */
    async elevate(stepUp: StepUp): Promise<Login["row"]> {
        // approve the sign-in at the universe, then hand the browser's return to the device's redirect
        const { universe, owner, simulation } = this.world;
        const browser = simulation.environment(BROWSER_NODE).network;

        return this.device.credential.browser(async (url) => {
            const back = await universe.approve(url, owner);
            await (await browser.fetch(new Request(back))).body?.cancel();
        }, stepUp);
    }

    /** Enroll the computer's machine in an account as a person signed in on its device. */
    async enroll(login: Login["row"], accountId: Identifier<"account">): Promise<void> {
        // enroll as the person
        const { id } = this.machine;
        await this.connect(machinePackage).computer.enroll({
            deviceId: this.device.id,
            requestId: RequestId.create(this.environment),
            id,
            login: login.id,
            accountId,
            name: TestComputer.nameOf("machine", id),
        });

        // wait until its partition received its scope
        await this.waitUntil(this.machine.database, async () => {
            const [received] = await Scope.read(this.machine.database.snapshot(), [id]);

            return received !== undefined;
        });
    }

    /** Create a space on this machine as a signed-in person, waiting until it serves it. */
    async space(
        login: Identifier<"login">,
        accountId: Identifier<"account">,
        name: string,
    ): Promise<Identifier<"space">> {
        // create the space on the machine as the person
        const session = await this.device.launcher(login);
        const created = await session.session.accounts().space.create({
            accountId,
            requestId: RequestId.create(this.environment),
            name,
            machine: this.machine.id,
        });

        // wait until the machine serves it
        await this.ready(created.id);

        return created.id;
    }

    /** Publish an image in the run's store on the computer's machine, the owner's by default. */
    async publish(image: Digest, publisher: TestPublisher = this.world.owner): Promise<void> {
        await this.world.distribution.publish(image, publisher, this.machine);
    }

    /** Install a build's bundled or published release in a space as a person, until applied. */
    async install(
        login: Identifier<"login">,
        spaceId: Identifier<"space">,
        build: BuildReader,
        name: string,
        options: { readonly isolation?: "sandbox" | "host" } = {},
    ): Promise<InstanceOf<typeof installation>> {
        // install the build's release as the person, until the space applies it
        const released = PackageManifest.package(build.manifest);
        const service = await this.#objects(login, spaceId);
        const created = await service.installation.create({
            scope: spaceId,
            requestId: RequestId.create(this.environment),
            packageId: released.id,
            ...(await this.#version(released.id, released.version)),
            name,
            ...options,
        });
        await this.#waitInstalled(spaceId, created.id, name);

        return await service.installation.get({ scope: spaceId, id: created.id });
    }

    /** Update a space's installation to a build's release as a person, until applied. */
    async update(
        login: Identifier<"login">,
        spaceId: Identifier<"space">,
        name: string,
        build: BuildReader,
    ): Promise<InstanceOf<typeof installation>> {
        // select the release for the named installation as the person, until the space applies it
        const { version } = PackageManifest.package(build.manifest);
        const service = await this.#objects(login, spaceId);
        const { items } = await service.installation.list({ scope: spaceId });
        const installed = present(
            items.find((each) => each.name === name),
            `an installation ${name} in space ${spaceId}`,
        );
        await service.installation.update({
            scope: spaceId,
            id: installed.id,
            requestId: RequestId.create(this.environment),
            ...(await this.#version(installed.packageId, version)),
        });
        await this.#waitInstalled(spaceId, installed.id, name);

        return await service.installation.get({ scope: spaceId, id: installed.id });
    }

    /** Resolve the version row of an installation of a release, the host's for a bundled one. */
    async #version(packageId: PackageId, version: string) {
        const { machine } = this.computer;
        const resolved = await InstallationTarget.resolveDeclared(
            packageId,
            { kind: "release", version },
            machine,
        );

        return ObjectReference.columns("version", resolved.version);
    }

    /** Call the object package of a space as a person signed in on this computer's device. */
    async #objects(login: Identifier<"login">, spaceId: Identifier<"space">) {
        const { device } = this;

        return createClient(
            stackPackage,
            (await device.launcher(login)).endpoint(
                stackPackage.id,
                await device.locate(spaceId).catch((error: unknown) => {
                    throw this.#failure(`space ${spaceId} did not locate`, error);
                }),
            ),
            device.environment,
        );
    }

    /** Wait until a space applies and serves an installation's targeted revision, or it fails. */
    async #waitInstalled(
        spaceId: Identifier<"space">,
        installationId: Identifier<"installation">,
        name: string,
    ): Promise<void> {
        // read the installation as the space has it now
        await this.machine.health.waitUp();
        const { database } = await this.machine.space(spaceId);
        const read = () =>
            database
                .snapshot()
                .read(installation.table)
                .findFirst({ where: { id: installationId } });

        // wait until the space applied and serves the targeted revision, or the installation failed
        const settled = async () => {
            const row = await read();

            return (
                row !== undefined &&
                (Installation.isServed(row) || Status.progress(row) === "failed")
            );
        };
        await this.waitUntil(database, settled, INSTALL_MILLISECONDS).catch(
            async (error: unknown) => {
                const conditions = JSON.stringify((await read())?.conditions);
                throw this.#failure(`${name} did not apply: ${conditions}`, error);
            },
        );

        // refuse an installation whose rollout failed for good, with its conditions
        const row = await read();
        if (row !== undefined && Status.progress(row) === "failed") {
            throw new TypeError(`'${name}' did not apply: '${Status.message(row)}'`);
        }
    }

    /** Wait until the machine serves a space with its stack and every installation applied. */
    async ready(spaceId: Identifier<"space">): Promise<void> {
        // wait on the world until the machine serves the space, then until the space reports itself up
        const deadline = this.clock.timeout(READY_MILLISECONDS);
        await this.world.simulation
            .wait(
                this.machine.health
                    .waitUp(deadline)
                    .then(async () => (await this.machine.space(spaceId)).health.waitUp(deadline)),
            )
            .catch(async (error: unknown) => {
                // name each installation not ready with its ready condition, beside the machine's reports
                const space = await this.machine.space(spaceId);
                const rows = await space.database
                    .select({
                        name: installation.table.name,
                        conditions: installation.table.conditions,
                    })
                    .from(installation.table);
                const unready = rows
                    .map((row) => ({ name: row.name, ready: row.conditions["Ready"] }))
                    .filter(({ ready }) => ready?.status !== "true")
                    .map(({ name, ready }) => `${name}: ${ready?.reason} ${ready?.message}`);
                const reports = this.captured().map((record) =>
                    String(record.attributes["exception.message"]),
                );

                // fail with both, the wait's error and the reports as causes
                const reason = space.health.reason;
                throw this.#failure(
                    `space ${spaceId} did not get ready: ${reason}; not ready: ${unready.join("; ")}; reports: ${reports.join("; ")}`,
                    [error, ...reports],
                );
            });
    }

    /** Wait until a check passes at a database's commits, within the receive limit. */
    async waitUntil(
        database: DatabaseConnection,
        check: () => Promise<boolean>,
        limit = RECEIVE_MILLISECONDS,
    ): Promise<void> {
        const deadline = this.clock.timeout(limit);
        if (!(await this.world.simulation.wait(database.log.waitUntil(check, deadline)))) {
            throw this.#failure(`the check did not pass within ${limit} ms`, [
                deadline.reason,
                ...this.captured().map((record) => String(record.attributes["exception.message"])),
            ]);
        }
    }

    /** Describe a failure with the exceptions the computer's world captured meanwhile. */
    #failure(message: string, cause: unknown): TypeError {
        // read what the world captured and what it has in flight
        const captured = this.world.telemetry.describe();
        const pending = this.world.simulation.scheduler.pending();

        return new TypeError(
            `${message}, its world capturing '${captured}' with ${pending.length} in flight:\n${pending.join("\n")}`,
            { cause },
        );
    }

    /** Stop the computer and start it again over its directory and keychains, as a restart does. */
    async restart(defer: Defer = onTestFinished): Promise<TestComputer> {
        // stop it, keeping what its keychains hold
        await this.close();
        const keychains = Object.fromEntries(
            [...this.keychains].map(([service, keychain]) => [
                service,
                Object.fromEntries(keychain.secrets),
            ]),
        );

        return TestComputer.open(
            { ...this.#options, world: this.world, directory: this.directory, keychains },
            defer,
        );
    }

    /** Stop the computer and its listener, as its process stops. */
    async close(): Promise<void> {
        await this.#stack.disposeAsync();
    }

    /** Stop the computer and its listener once. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Make a fresh state directory, moved to the run's trash once its lifetime ends. */
    static async #directory(files: FileSystem, defer: Defer): Promise<string> {
        // make the directory
        const directory = await files.realPath(
            await files.makeTemporaryDirectory("destack-machine-"),
        );

        // move it to the run's trash in one rename unless a test erased it, named by its simulation's root
        const trash = inject("trash");
        const name = `${basename(dirname(directory))}-${basename(directory)}`;
        defer(() =>
            trash === undefined
                ? files.remove(directory, { recursive: true, force: true })
                : files
                      .rename(directory, join(trash, name))
                      .catch((error: unknown) => FileSystemError.absent(error)),
        );

        return directory;
    }

    /** Open a keychain in memory with some secrets. */
    static #keychain(secrets: Readonly<Record<string, string>>): MemoryKeychain {
        const keychain = new MemoryKeychain();
        for (const [name, secret] of Object.entries(secrets)) {
            keychain.secrets.set(name, secret);
        }

        return keychain;
    }
}

/** Answer every request as unavailable, until the computer runs. */
async function unavailable(): Promise<Response> {
    return new Response(null, { status: 503 });
}
