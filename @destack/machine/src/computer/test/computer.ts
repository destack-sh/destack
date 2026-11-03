import { FORGE_INSTALLATION } from "@destack/forge/package";
import { type Environment, type FileSystem, FileSystemError } from "@destack/environment";
import { TestTelemetry } from "@destack/telemetry/test";
import { basename, dirname, join } from "node:path";
import type { ResidencyCode } from "@destack/account/object";
import { DOMAINS } from "@destack/account/address";
import type { StepUp } from "@destack/access";
import type { Login } from "@destack/account/native";
import { freshHandle, type TestPerson } from "@destack/account/test";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { TestDatabase, TestTemplate } from "@destack/db/test";
import { Simulation, SimulatedClock } from "@destack/environment/simulation";
import { reference, repository } from "@destack/registry/object";
import { MemoryKeychain } from "@destack/identity";
import { type ObjectType, type PackageHandle, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import { Identifier, schema } from "@destack/schema";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { rollout, Rollout } from "@destack/stack/object";
import type { Installation } from "@destack/object";
import objectPackage from "@destack/object/package";
import { TestRegistry } from "@destack/stack/test";
import { RequestId, Scope } from "@destack/sync";
import { inject, onTestFinished } from "@destack/test";
import { Universe } from "@destack/universe";
import { TestUniverse, UniverseWorld } from "@destack/universe/test";
import relayPackage from "@destack/relay/package";
import { TUNNEL_TOKEN_LIFETIME_MILLISECONDS } from "@destack/relay/server";
import { TestRelay } from "@destack/relay/test";
import type { MachineEnrollment } from "../device/object/index.ts";
import machinePackage from "../../package.ts";
import { Computer, type ComputerOptions, MACHINE_KEYS } from "../computer.ts";
import { Kernel } from "../kernel.ts";
import { testHost } from "@destack/test/host";
import { fileURLToPath } from "node:url";
import { LoopbackEndpoint, ENDPOINT_FILE, ENDPOINT_MODE } from "../process/index.ts";
import { TestDistribution } from "./distribution.ts";

/** The packages the computer provides: its module's closure, walked once as the kit loads. */
const COMPUTER_KERNEL = await Kernel.read(
    fileURLToPath(new URL("../index.ts", import.meta.url)),
    testHost(),
);
/** How long a scenario of machines in a universe takes at most. */
export const SCENARIO_MILLISECONDS = 30_000;

/** When every simulation starts: 16 October 2026, UTC. */
const START = Date.UTC(2026, 9, 16);

/** The seed of every simulation's randomness. */
const SEED = 1;

/** The email the computers' owner signs up with. */
export const OWNER_EMAIL = "owner@example.test";

/** The node the owner's browser runs as in the simulated network. */
const BROWSER_NODE = "browser";

/** The node of the computers' state directories, over the host's files. */
export const FILES_NODE = "files";

/** The port HTTPS origins answer at, which the universe's origin leaves implicit. */
const HTTPS_PORT = 443;

/** The node the residency's relay runs as in the simulated network. */
export const RELAY_NODE = "relay";

/** The host name the residency's relay answers at, where computers open their tunnels. */
const RELAY_HOSTNAME = "eu.relay.test";

/** The residency the owner and the cloud machine live in. */
const RESIDENCY: ResidencyCode = "eu";

/** The characters of a machine identifier a test name uses: its UUID's last group. */
const NAME_SUFFIX_LENGTH = 12;

/** How long a computer waits for rows to arrive: 10 s, far above their milliseconds. */
const RECEIVE_MILLISECONDS = 10_000;

/** The wait for a space to get ready: 30 s, nine restarts of about a second, tripled under load. */
const READY_MILLISECONDS = 30_000;

/** How long an install may take to apply: 2 min, as a space pulls an app's closure in ~30 s. */
const INSTALL_MILLISECONDS = 120_000;

/** The policies of the forge's objects, which the universe decides repository access with. */
const POLICIES = [repository.policy, reference.policy];

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
});
/** The world a home template wrote. */
type HomeWorld = schema.Infer<typeof HomeWorld>;

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
    /** The residency whose cloud machine the computer runs, a personal machine by default. */
    readonly residency?: ResidencyCode;
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

/** A Destack computer as its process composes it, in a test universe on a simulated network. */
export class TestComputer implements AsyncDisposable {
    /** The world the computer runs in. */
    readonly world: TestWorld;
    /** The computer. */
    readonly computer: Computer;
    /** The computer's state directory. */
    readonly directory: string;
    /** The origin the computer answers at on the simulated network. */
    readonly endpoint: string;
    /** The computer's keychains, by service. */
    readonly keychains: Map<string, MemoryKeychain>;
    /** The registry the machine pulls releases from, with every shipped build pushed. */
    readonly registry: TestRegistry;
    /** The failures the computer reported to no caller. */
    readonly reports: unknown[];
    /** The first failure the computer reported to no caller, once it does. */
    readonly reported: Promise<unknown>;
    /** Stop the computer and its listener. */
    readonly #stack: AsyncDisposableStack;

    /** The world the computer runs in, its own. */
    get environment(): Environment<"files" | "host" | "server"> {
        return this.computer.device.environment;
    }

    /** Take a started computer. */
    private constructor(fields: {
        readonly world: TestWorld;
        readonly computer: Computer;
        readonly directory: string;
        readonly endpoint: string;
        readonly keychains: Map<string, MemoryKeychain>;
        readonly registry: TestRegistry;
        readonly reports: unknown[];
        readonly reported: Promise<unknown>;
        readonly stack: AsyncDisposableStack;
    }) {
        // set the computer, its world and its state
        this.world = fields.world;
        this.computer = fields.computer;
        this.directory = fields.directory;
        this.endpoint = fields.endpoint;
        this.keychains = fields.keychains;
        this.registry = fields.registry;
        this.reports = fields.reports;
        this.reported = fields.reported;
        this.#stack = fields.stack;
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
        // fork the template's world with its universe and owner
        const { simulation, databases, value } = await TestDatabase.copy(TestComputer.#universe(), {
            label: "copy",
        });
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
        const telemetry = new TestTelemetry(simulation.telemetry);
        defer(() => telemetry[Symbol.dispose]());

        // start the universe on the simulation, answering at its origin on the network
        const universe = await TestUniverse.open({
            simulation,
            releases: distribution,
            stacks: distribution.stacks,
            home: distribution.home,
            policies: POLICIES,
            lifetimes: new Map([[relayPackage.id, TUNNEL_TOKEN_LIFETIME_MILLISECONDS]]),
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
            write: async (databases, _directory, simulation) => {
                // start the universe over the template's database
                await using lifetime = new AsyncDisposableStack();
                const distribution = await TestDistribution.build(
                    simulation.environment(FILES_NODE),
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

    /** Start the residency's relay on a node of its own, waiting until it follows the universe. */
    static async relay(world: TestWorld, defer: Defer = onTestFinished): Promise<TestRelay> {
        // host the relay as the residency's cloud machine does, the names DNS sends it answered on its node
        const { simulation, universe } = world;
        const host = await universe.machine(RESIDENCY);
        const relay = await TestRelay.open({
            universe,
            host,
            environment: simulation.environment(RELAY_NODE),
            hostname: RELAY_HOSTNAME,
            names: [`*.${DOMAINS.machine}`, `*.${DOMAINS.space}`],
        });
        defer(() => relay[Symbol.asyncDispose]());

        // wait until its replicas reached the universe
        const deadline = simulation.clock.timeout(RECEIVE_MILLISECONDS);
        if (!(await relay.waitCaughtUp(deadline))) {
            throw new TypeError(`the relay did not catch up in ${RECEIVE_MILLISECONDS} ms`, {
                cause: deadline.reason,
            });
        }

        return relay;
    }

    /** List the templates the run writes once for the computers' worlds. */
    static templates() {
        return [TestComputer.#universe(), TestComputer.#home()] as const;
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
            options.residency === undefined
                ? undefined
                : (await universe.world()).machines[options.residency];
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
        let route: (request: Request) => Promise<Response> = unavailable;

        // publish the listener on the host as well for sandboxed runners, which are host processes
        const serving = {
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request: Request) => route(request),
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
            throw new TypeError(`the universe declares no origin of ${cloud.id}`);
        }
        if (declared !== undefined) {
            const published = environment.server.http({
                hostname: new URL(declared).hostname,
                port: HTTPS_PORT,
                fetch: (request) => route(request),
            });
            stack.defer(() => published.stop(true));
        }

        // pull releases from the distribution's registry
        const { registry } = distribution;

        // start the computer, recording the failures it reports but the tunnel's dials, which it retries
        const reports: unknown[] = [];
        const reported = Promise.withResolvers<unknown>();
        const enrollment: MachineEnrollment | undefined =
            options.residency === undefined
                ? undefined
                : {
                      kind: "cloud",
                      residencyId: options.residency,
                      issuer: universe.origin,
                      status: "enrolled",
                  };
        const computer = await Computer.start({
            // run instances in this process on the world's clock and network, deterministic under simulation
            isolation: "process",
            kernel: COMPUTER_KERNEL,
            registry: FORGE_INSTALLATION,
            environment,
            directory,
            issuer: universe.origin,
            port,
            ...(declared === undefined ? {} : { endpoint: declared }),
            ...(options.relay === undefined ? {} : { relay: { kind: "url", url: options.relay } }),
            localObjects: options.localObjects ?? {},
            keychain,
            ...(cloud === undefined ? {} : { machineId: cloud.id }),
            process: {
                version: machinePackage.version,
                pid: environment.process.id,
                startedAt: simulation.clock.now(),
            },
            ...(options.runtime === undefined ? {} : { runtime: options.runtime }),
            bundled: distribution.layer,
            releases: registry,
            ...(options.distribution === undefined ? {} : { distribution: options.distribution }),
            ...(enrollment === undefined ? {} : { enrollment }),
            report: (event, error) => {
                if (event !== "tunnel.failed") {
                    reports.push(error);
                    reported.resolve(error);
                }
            },
            shutdown: () => {},
        });
        stack.defer(() => computer[Symbol.asyncDispose]());
        route = (request) => computer.fetch(request);

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
            computer,
            directory,
            endpoint: declared ?? `http://127.0.0.1:${port}`,
            keychains,
            registry,
            reports,
            reported: reported.promise,
            stack: stack.move(),
        });
        defer(() => started[Symbol.asyncDispose]());

        return started;
    }

    /** Start the residency's cloud computer serving the owner's home, from the run's template. */
    static async home(
        started: Omit<TestComputerOptions, "residency" | "directory" | "keychains"> = {},
        defer: Defer = onTestFinished,
    ): Promise<{ readonly home: TestComputer; readonly login: Identifier<"login"> }> {
        // fork the template's world with its universe
        const { simulation, databases, value } = await TestDatabase.copy(TestComputer.#home(), {
            label: "copy",
        });
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
                residency: RESIDENCY,
                directory,
                keychains: value.keychains,
            },
            defer,
        );
        await home.ready(world.owner.home);

        return { home, login: value.login };
    }

    /** Describe the world of the owner's home on its cloud computer, which the run writes once. */
    static #home(): TestTemplate<HomeWorld, { readonly universe: typeof Universe.database }> {
        return new TestTemplate({
            name: "machine.home",
            databases: { universe: Universe.database },
            value: HomeWorld,
            world: { seed: SEED, start: START },
            write: async (databases, directory, simulation) => {
                // start the world over the template's database, closed once written
                await using lifetime = new AsyncDisposableStack();
                const defer: Defer = (cleanup) => {
                    lifetime.defer(cleanup);
                };
                const distribution = await TestDistribution.build(
                    simulation.environment(FILES_NODE),
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
                    { world, residency: RESIDENCY, directory: state },
                    defer,
                );
                const login = await home.signIn(OWNER_EMAIL);
                await home.ready(world.owner.home);
                await home.close();

                // record the state, the keychains and the universe's keys
                const keychains = home.secrets();

                return {
                    universe: await world.universe.world(),
                    directory: state,
                    keychains,
                    login: login.id,
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
    connect<Target extends PackageHandle>(target: Target) {
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
        // mail the code, signing a new person up in the residency
        const started = await this.device.signIns.startEmail(email, {
            name: email,
            handle: freshHandle(this.environment),
            residencyId: RESIDENCY,
            timeZone: "UTC",
            locale: "en",
        });

        // redeem the last code mailed to them
        const codes = await this.world.universe.mail.codes();
        const code = codes.findLast((mailed) => mailed.email === email);
        if (code === undefined) {
            throw new TypeError(`the universe mailed no code to ${email}`);
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

        return this.device.signIn(
            async (url) => {
                const back = await universe.approve(url, owner);
                await (await browser.fetch(new Request(back))).body?.cancel();
            },
            undefined,
            stepUp,
        );
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
            const [received] = await Scope.read(Snapshot.live(this.machine.database), [id]);

            return received !== undefined;
        });
    }

    /** Create a space on this machine as a signed-in person, waiting until it serves it. */
    async space(
        login: Login["row"],
        accountId: Identifier<"account">,
        name: string,
    ): Promise<Identifier<"space">> {
        // create the space on the machine as the person
        const session = await this.device.session(login.id);
        const created = await session.login.accounts().space.create({
            accountId,
            requestId: RequestId.create(this.environment),
            name,
            machine: this.machine.id,
        });

        // wait until the machine serves it
        await this.ready(created.id);

        return created.id;
    }

    /** Install a build's release in a space as a signed-in person, waiting until applied. */
    async install(
        login: Identifier<"login">,
        spaceId: Identifier<"space">,
        build: BuildReader,
        name: string,
    ): Promise<Installation> {
        // publish the build as its package's release, with the releases it was built against
        await this.world.distribution.publish(build);
        const released = PackageManifest.package(build.manifest);

        // install the release as the person
        const { device } = this;
        const service = createClient(
            objectPackage,
            (await device.session(login)).endpoint(
                objectPackage.id,
                await device.locate(spaceId).catch((error: unknown) => {
                    throw this.#failure(`space ${spaceId} did not locate`, error);
                }),
            ),
            device.environment,
        );
        const created = await service.installation.create({
            scope: spaceId,
            requestId: RequestId.create(this.environment),
            packageId: released.id,
            selection: { kind: "release", version: released.version },
            name,
        });

        // wait until the space applies it, naming its conditions when it does not
        await this.machine.health.waitUp();
        const { database } = await this.machine.space(spaceId);
        const read = async () => {
            // read its rollout with the installation
            const snapshot = Snapshot.live(database);
            const rows = await snapshot.read(rollout.table).findMany({ where: { id: created.id } });
            const [row] = await Rollout.read(snapshot, rows);

            return row;
        };
        const applied = async () => {
            const row = await read();

            return row?.appliedRevisionId !== null && row?.appliedRevisionId !== undefined;
        };
        await this.waitUntil(database, applied, INSTALL_MILLISECONDS).catch(
            async (error: unknown) => {
                const conditions = JSON.stringify((await read())?.installation.conditions);
                throw this.#failure(`${name} did not apply: ${conditions}`, error);
            },
        );

        return await service.installation.get({ scope: spaceId, id: created.id });
    }

    /** Wait until the machine serves a space with its stack and every installation applied. */
    async ready(spaceId: Identifier<"space">): Promise<void> {
        // wait on the world until the machine serves the space with every installation applied and served
        await this.world.simulation.wait(this.machine.health.waitUp());
        const space = await this.machine.space(spaceId);
        await this.waitUntil(
            space.database,
            async () => space.health.status === "up",
            READY_MILLISECONDS,
        ).catch((error: unknown) => {
            throw this.#failure(
                `space ${spaceId} did not get ready: ${space.health.reason}`,
                error,
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
                ...this.reports,
            ]);
        }
    }

    /** Describe a failure with the exceptions the computer's world captured meanwhile. */
    #failure(message: string, cause: unknown): TypeError {
        // read what the world captured and what it has in flight
        const captured = this.world.telemetry.describe();
        const pending = this.world.simulation.scheduler.pending();

        return new TypeError(
            `${message}, its world capturing ${captured} with ${pending.length} in flight:\n${pending.join("\n")}`,
            { cause },
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
