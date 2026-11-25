import { TelemetryScope } from "@destack/telemetry";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { IssuerCredential, MachineKey, Session } from "@destack/account/client";
import { LocalBucket } from "@destack/bucket/bun";
import { S3Server } from "@destack/bucket/s3";
import { Toolchain } from "@destack/check/toolchain";
import { PostgresPool } from "@destack/db/postgres";
import { DESKTOP_OBJECTS } from "@destack/desktop/object";
import {
    type Environment,
    FileSystemError,
    type HostCapability,
    untilAborted,
    waitAborted,
} from "@destack/environment";
import { LOG_LEVEL_VARIABLE } from "@destack/environment/line";
import { type Keychain, KeyPair, LocalKeyring, MemoryKeychain } from "@destack/identity";
import { BunKeychain } from "@destack/identity/bun";
import {
    Computer,
    type ComputerOptions,
    MachineProcess,
    MACHINE_KEYS,
    ROOT_KEYS,
    UniverseHome,
} from "@destack/machine/computer";
import { Device, type MachineEnrollment } from "@destack/machine/device";
import { PrintTransport } from "@destack/mail/print";
import { Bookmark } from "@destack/object/client";
import type { HostListening } from "@destack/object/workload";
import { DISTRIBUTION_LAYER, DistributionManifest, DistributionPlatform } from "@destack/package";
import { BunDistribution } from "@destack/package/bun";
import { BuildLayer } from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import type { UniverseDefinition } from "@destack/universe";
import { BunUniverse } from "@destack/universe/bun";
import { LATEST_TAG } from "@destack/registry/object";
import { Profile } from "@destack/update/release";
import { CHECKOUT, PlatformBuild, printStatus, readCommit } from "@platform/distribution";
import { PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import {
    type SuppliedModules,
    WorkerScript,
    type WranglerConfiguration,
} from "../cloudflare/script.ts";
import { Custody } from "../deployment/custody.ts";
import { Deployment, LocalPorts, LOOPBACK } from "../deployment/deployment.ts";
import {
    Root,
    ROOT,
    type ResidencyOutputs,
    type UniverseOutputs,
    type WorkerOutputs,
    WRANGLER_VARIABLES,
} from "../deployment/root.ts";
import { PrintedMail } from "./mail.ts";
import { LocalPublisher } from "./publisher.ts";
import { LocalProcess } from "./process.ts";

/** The local PostgreSQL server, the test server of `just postgres up`. */
export const SERVER = "postgres://postgres@127.0.0.1:55432";

/** The default local universe's universe port, which its other ports follow. */
const UNIVERSE_PORT = 4100;

/** The local universe without `--universe`: database destack_local, target/local. */
const DEFAULT_NAME = "local";

/** A local universe's name, a PostgreSQL identifier's tail and a directory name. */
const NAME = /^[a-z][a-z0-9_]*$/u;

/** The ports between two local universes' universe ports: their five ports, rounded up. */
const PORT_BLOCK = 10;

/** The file in a local universe's directory with the universe port it took at its first open. */
const PORTS_FILE = "ports.json";

/** The ports file: the universe's port, which its other services follow. */
const Ports = schema.object({ universe: schema.number().int().positive() });

/** The operator a local universe declares as its platform organization's owner. */
const OPERATOR = "operator@destack.test";

/** The root keys of the local universe, which only this computer uses. */
const UNIVERSE_ROOT_KEY =
    '{"active":1,"keys":{"1":"DHDaqs60kZjIDOmpchcC6lggMf8L+sHPgayD5fqCuN0="}}';

/** The root keys of the local cloud machines, which only this computer uses, by residency code. */
const MACHINE_ROOT_KEYS: ReadonlyMap<string, string> = new Map([
    ["eu", '{"active":1,"keys":{"1":"TO1o+AlghKysP7TyW2QfPhOJOOBa/NRh7/a02fsCgQs="}}'],
    ["us", '{"active":1,"keys":{"1":"w20+AFE9NraWIlc9hkKZfGfhO062wFMYkoG5HJo8K2o="}}'],
]);

/** The runtimes of a local universe's cloud machines: Bun, or workerd as production. */
const LOCAL_RUNTIMES = ["bun", "workerd"] as const;

/** The usage of a local universe's flags. */
const USAGE =
    "use local <serve|reset|publish|desktop|cli> [--runtime <bun|workerd>] [--universe <name>], a name of a-z, 0-9 and _";

/** The directories of a local universe's state, which a reset removes. */
const STATE_DIRECTORIES = ["universe", "machines", "workers"] as const;

/** How long the local universe's machine may take to answer after it starts, in milliseconds. */
const MACHINE_START_MILLISECONDS = 30_000;

/** How long to wait between asks whether the local universe's machine answers, in milliseconds. */
const MACHINE_POLL_MILLISECONDS = 100;

/** How long a started cloud machine may take to replicate its enabled status, in milliseconds. */
const MACHINE_SERVE_MILLISECONDS = 5_000;

/** The variables of this process the local universe's machine runs with. */
const INHERITED = ["HOME", "PATH", "TMPDIR", "LANG", "LC_ALL", "USER"] as const;

/** The directory below a local universe's Worker storage where its Workers find each other. */
const WORKER_REGISTRY = "registry";

/** The identifier a run under workerd binds where a deployment's state has a real one. */
const PLACEHOLDER_IDENTIFIER = "0".repeat(32);

/** The offset of a script's inspector port from its own port, clear of the loopback ports. */
const INSPECTOR_OFFSET = 5000;

/** The configuration keys a run under workerd leaves out: routes and tails. */
const DEPLOYED_KEYS = new Set(["routes", "tail_consumers"]);

/** The directory below a local universe's directory a run under workerd keeps its files in. */
const WORKERD_DIRECTORY = "workerd";

/** The password local Hyperdrive requires, which the trusting local server ignores. */
const HYPERDRIVE_PASSWORD = "local";

/** The prefix of wrangler's own information lines, such as its access log. */
const WRANGLER_INFO_PREFIX = "[wrangler:info]";

/** The region a run under workerd's files bucket signs for. */
const PLACEHOLDER_REGION = "local";

/** The access key a local run binds in place of a provisioned one. */
const PLACEHOLDER_KEY = { accessKeyId: "local", secretAccessKey: "local" };

/** What a local run's roots provision for a Worker, whose local custody envelopes nothing. */
const PLACEHOLDER_WORKER: WorkerOutputs = { envelope: { kind: "local" }, access: PLACEHOLDER_KEY };

/** Where a local universe's cloud machines run. */
export type LocalRuntime = (typeof LOCAL_RUNTIMES)[number];

/** A universe on this computer: its state, its loopback ports and its device. */
export class LocalUniverse {
    /** The universe's name as `--universe` gives it. */
    readonly name: string;
    /** Where the cloud machines run. */
    readonly runtime: LocalRuntime;
    /** The directory keeping the universe's history and the cloud machines' state. */
    readonly directory: string;
    /** The database keeping the account service's tables. */
    readonly database: string;
    /** The loopback port each of the universe's services listens on. */
    readonly ports: LocalPorts;
    /** The platform deployed at the universe's loopback ports. */
    readonly deployment: Deployment;
    /** The device the universe's machine, desktop app and cli share. */
    readonly home: UniverseHome;
    /** The host the universe and the machines run on. */
    readonly environment: Environment<HostCapability>;
    /** The universe's telemetry, whose records print as status lines. */
    readonly scope: TelemetryScope;

    /** Create a local universe, its workerd state apart from the Bun state of its name. */
    private constructor(
        name: string,
        runtime: LocalRuntime,
        ports: LocalPorts,
        environment: Environment<HostCapability>,
    ) {
        // keep the default Bun universe's device in its own home directory, any other below its directory
        const stored = LocalUniverse.#stored(name, runtime);
        const directory = resolve(CHECKOUT, "target", stored);
        const deployment = Deployment.local(ports, environment);
        const profile = new Profile("local");
        const user = environment.host.home;
        this.name = name;
        this.runtime = runtime;
        this.directory = directory;
        this.database = `destack_${stored}`;
        this.ports = ports;
        this.deployment = deployment;
        this.home =
            stored === DEFAULT_NAME
                ? new UniverseHome(profile, deployment.universe.origin, user)
                : new UniverseHome(
                      profile,
                      deployment.universe.origin,
                      user,
                      resolve(directory, "device"),
                  );
        this.environment = environment;
        this.scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Read a local universe from `[--runtime <runtime>] [--universe <name>]`, refusing more. */
    static async of(
        arguments_: readonly string[],
        environment: Environment<HostCapability>,
    ): Promise<LocalUniverse> {
        // read the flags, refusing any word after them
        const { local, rest } = await LocalUniverse.read(arguments_, environment);
        if (rest.length > 0) {
            throw new Error(USAGE);
        }

        return local;
    }

    /** Read a local universe from leading `--runtime` and `--universe`, answering the rest. */
    static async read(
        arguments_: readonly string[],
        environment: Environment<HostCapability>,
    ): Promise<{ readonly local: LocalUniverse; readonly rest: readonly string[] }> {
        // read each leading flag and its value, refusing a flag given twice or without a value
        const flags = new Map<string, string>();
        let index = 0;
        for (
            let flag = arguments_[index];
            flag === "--runtime" || flag === "--universe";
            flag = arguments_[index]
        ) {
            const value = arguments_[index + 1];
            if (value === undefined || flags.has(flag)) {
                throw new Error(USAGE);
            }
            flags.set(flag, value);
            index += 2;
        }

        // refuse a runtime or a name outside their sets
        const runtime = LOCAL_RUNTIMES.find((known) => known === (flags.get("--runtime") ?? "bun"));
        const name = flags.get("--universe") ?? DEFAULT_NAME;
        if (runtime === undefined || !NAME.test(name)) {
            throw new Error(USAGE);
        }

        // lay it out from its universe's port
        const port = await LocalUniverse.#port(name, runtime, environment);
        const local = new LocalUniverse(name, runtime, LocalPorts.at(port), environment);

        return { local, rest: arguments_.slice(index) };
    }

    /** The directory of the distribution the universe's machines ship. */
    get distribution(): string {
        return join(this.directory, "distribution");
    }

    /** The variables that put the desktop app and the cli on the universe's device and origin. */
    get variables(): Record<string, string> {
        return {
            DESTACK_PROFILE: this.home.profile.name,
            DESTACK_UNIVERSE_ORIGIN: this.home.universe,
            ...(this.home.isCustom ? { DESTACK_DIRECTORY: this.home.directory } : {}),
            DESTACK_DISTRIBUTION: this.distribution,
        };
    }

    /** Print a status line under the name of the process it concerns. */
    print(name: string, line: string): void {
        printStatus(name, line, this.environment);
    }

    /** Serve the universe, its cloud machines and this computer's machine until a signal aborts. */
    async serve(signal: AbortSignal): Promise<void> {
        // create the database once
        const { deployment, environment } = this;
        await this.#administer(async (sql) => {
            const existing = await sql<
                { readonly found: number }[]
            >`SELECT 1 AS found FROM pg_database WHERE datname = ${this.database}`;
            if (existing.length === 0) {
                await sql.unsafe(`CREATE DATABASE "${this.database}"`);
            }
        });

        // read the fixed keys, then build the checkout
        const custody = await this.custody();
        await using stack = new AsyncDisposableStack();
        const build = stack.use(
            await PlatformBuild.build(CHECKOUT, environment, { isUnreleased: true }),
        );

        // issue the local release client's assertions, as the release workflow's issuer does
        const publisher = await this.#publisher();
        const issuing = environment.server.http({
            hostname: LOOPBACK,
            port: this.ports.publisher,
            fetch: (request) => publisher.fetch(request),
        });
        stack.defer(async () => issuing.stop(true));

        // declare the universe trusting that client to publish, and write the distribution its machines ship
        const declared = deployment.definition(OPERATOR, custody, build);
        const definition = {
            ...declared,
            platform: { ...declared.platform, clients: [publisher.client] },
        };
        await build.writeDistribution(
            this.distribution,
            DistributionPlatform.of(environment.host),
            true,
        );
        this.scope.log.info("platform.distribution.written", { "file.path": this.distribution });

        // start the universe and its cloud machines, the platform residency's at the universe's port, until a signal
        const mail = new PrintedMail();
        const started = async () =>
            stack.use(
                this.runtime === "bun"
                    ? await this.#startBun(build, definition, custody, mail)
                    : await this.#startWorkerd(build, definition, custody, mail),
            );
        await untilAborted(started(), signal);
        if (signal.aborted) {
            return;
        }

        // start this computer's machine against the universe, and serve until a signal
        stack.use(await this.#startMachine());
        this.printReady();
        await waitAborted(signal);
        this.scope.log.info("platform.universe.signaled", {
            "platform.signal": String(signal.reason),
        });
    }

    /** Publish the checkout's releases into the universe's forge, as a deploy's workflow does. */
    async publish(): Promise<void> {
        // sign in as the local release client, whose issuer the serving universe answers
        const { deployment, environment } = this;
        const publisher = await this.#publisher();
        const send = (request: Request) => environment.network.fetch(request);
        const session = new Session(
            new IssuerCredential(publisher.signer, deployment.universe.issuer, send, environment),
            { universe: deployment.universe.origin, send, bookmark: new Bookmark(), environment },
        );

        // build the checkout's releases at the version the forge releases next, as a deploy's prepare reads it
        const version = await PlatformBuild.next(session, deployment.universe.origin, environment);
        await using build = await PlatformBuild.build(CHECKOUT, environment, { version });

        // publish them at the checkout's commit, pointing latest
        await build.publish(session, deployment.universe.origin, deployment.tag, {
            commit: await readCommit(CHECKOUT, environment),
            tags: [LATEST_TAG],
        });
    }

    /** Open the local release client's issuer at its loopback port. */
    #publisher(): Promise<LocalPublisher> {
        return LocalPublisher.open(`http://${LOOPBACK}:${this.ports.publisher}`, this.environment);
    }

    /** Drop the universe's database, universe and cloud machines, and erase its machine. */
    async reset(): Promise<void> {
        // refuse while the universe runs
        const { environment } = this;
        const isRunning = await environment.network
            .fetch(new Request(this.deployment.universe.issuer))
            .then(
                () => true,
                () => false,
            );
        if (isRunning) {
            throw new Error("stop just local before resetting it");
        }

        // erase the universe's machine, refused while it runs, then the database
        await Computer.erase(
            this.home.directory,
            (service) => new BunKeychain(service, environment.files, { kind: "system" }),
            environment,
        );
        await this.#administer(async (sql) => {
            await sql.unsafe(`DROP DATABASE IF EXISTS "${this.database}" WITH (FORCE)`);
        });

        // remove the universe's state, leaving the build store and the distribution, which content addresses
        for (const state of STATE_DIRECTORIES) {
            const path = join(this.directory, state);
            await environment.files.remove(path, { recursive: true, force: true });
        }
    }

    /** Print how to open the universe in a browser, the desktop app and the cli. */
    printReady(): void {
        // show the runtime and the name in the commands of a universe other than the default
        const runtime = this.runtime === "bun" ? "" : ` --runtime ${this.runtime}`;
        const name = this.name === DEFAULT_NAME ? "" : ` --universe ${this.name}`;
        const flags = `${runtime}${name}`;
        this.print("ready", `${this.deployment.universe.origin}/ opens Destack in a browser`);
        this.print("ready", `just local desktop${flags} runs the desktop app`);
        this.print("ready", `just local cli${flags} <command> runs the cli`);
        this.print("ready", "Ctrl-C stops everything");
    }

    /** Keep the fixed local root keys in the clear with each cloud machine's key they derive. */
    async custody(): Promise<Custody> {
        const { environment } = this;
        const machines = await Promise.all(
            RESIDENCIES.map(async (residency) => {
                // derive the machine's key from its fixed root keys
                const { id } = residency.machine;
                const rootKey = LocalUniverse.#rootKey(residency);
                const keychain = new MemoryKeychain();
                await MachineKey.save(id, await LocalKeyring.read(rootKey, environment), keychain);
                const key = present(await keychain.load(id), `the key of ${id}`);
                const { publicKey } = await KeyPair.read(key, environment.crypto);

                return [id, { envelope: { kind: "local" }, rootKey, publicKey, key }] as const;
            }),
        );

        return new Custody({
            universe: { envelope: { kind: "local" }, rootKey: UNIVERSE_ROOT_KEY },
            machines: Object.fromEntries(machines),
        });
    }

    /** Start the universe and the cloud machines on Bun in this process. */
    async #startBun(
        build: PlatformBuild,
        definition: UniverseDefinition,
        custody: Custody,
        mail: PrintedMail,
    ): Promise<AsyncDisposableStack> {
        // read the distribution the universe and its cloud machines ship
        const { deployment, environment } = this;
        await using stack = new AsyncDisposableStack();
        const target = DistributionPlatform.of(environment.host);
        const shipped = await BunDistribution.read(this.distribution, environment.files);
        const layer = await BuildLayer.open(
            join(this.distribution, DISTRIBUTION_LAYER),
            environment,
        );

        // start the universe, which the platform residency's machine serves at its origin
        const universe = stack.use(
            await BunUniverse.start({
                environment,
                definition,
                origin: deployment.universe,
                database: `${SERVER}/${this.database}`,
                rootKey: custody.record.universe.rootKey,
                providers: deployment.providers(
                    (id) => environment.process.variables[Deployment.providerSecret(id)],
                ),
                mail: new PrintTransport((text) => {
                    mail.push(text);
                    this.print("mail", text);
                }),
                directory: join(this.directory, "universe"),
                report: (error) => Computer.capture(environment.telemetry, "universe", error),
                distribution: DistributionManifest.releases(shipped, target),
                layer,
            }),
        );
        this.scope.log.info("platform.universe.started", {
            "url.full": deployment.universe.origin,
        });

        // start each cloud machine on the distribution, and wait until it serves
        const distribution = { shipped, layer };
        for (const residency of RESIDENCIES) {
            const served = residency.code === PLATFORM.residencyId ? universe.listening : undefined;
            const machine = await this.#startCloud(residency, distribution, custody, served, stack);
            const window = environment.clock.timeout(MACHINE_SERVE_MILLISECONDS);
            await machine.machine.health.waitUp(window);
            this.scope.log.info("platform.machine.started", {
                "platform.machine.name": residency.machine.name,
                "url.full": deployment.machineOrigin(residency),
            });
        }

        return stack.move();
    }

    /** Start a residency's cloud machine on Bun, serving the universe when given. */
    async #startCloud(
        residency: PlatformResidency,
        distribution: Pick<ComputerOptions, "shipped" | "layer">,
        custody: Custody,
        universe: HostListening | undefined,
        stack: AsyncDisposableStack,
    ): Promise<Computer> {
        // keep the machine's root keys and its key in memory, one keychain per service
        const { deployment, environment } = this;
        const { id, name } = residency.machine;
        const keys = custody.machine(id);
        const keychains = new Map<string, MemoryKeychain>();
        const keychain = (service: string): Keychain => {
            const kept = keychains.get(service) ?? new MemoryKeychain();
            keychains.set(service, kept);

            return kept;
        };
        await keychain(ROOT_KEYS).save(id, keys.rootKey);
        await keychain(MACHINE_KEYS).save(id, keys.key);

        // listen first, unavailable until it starts
        let started: Computer | undefined;
        const origin = new URL(deployment.machineOrigin(residency));
        const listener = environment.server.http({
            hostname: origin.hostname,
            port: Number(origin.port),
            idleTimeout: Computer.idleTimeout,
            ...this.#listen(() => started, universe),
        });
        stack.defer(async () => listener.stop(true));

        // start it enrolled as the residency's cloud machine
        const directory = join(this.directory, "machines", name);
        await new MachineProcess(directory, environment).createDirectory();
        const enrollment: MachineEnrollment = {
            kind: "cloud",
            universe: deployment.universe.toJSON(),
            residencyId: residency.code,
            status: "enrolled",
        };
        started = stack.use(
            await Computer.start({
                isolation: "sandbox",
                directory: await environment.files.realPath(directory),
                universe: deployment.universe.origin,
                port: Number(origin.port),
                environment,
                localObjects: DESKTOP_OBJECTS,
                ...distribution,
                keychain,
                toolchain: async () => Toolchain.workspace(),
                machineId: id,
                enrollment,
                process: {
                    version: deployment.tag,
                    pid: environment.process.id,
                    startedAt: environment.clock.now(),
                },
                shutdown: () =>
                    this.scope.log.info("platform.machine.signaled", {
                        "platform.machine.name": name,
                    }),
            }),
        );

        return started;
    }

    /** Start the universe and cloud machines as Workers under workerd, as production does. */
    async #startWorkerd(
        build: PlatformBuild,
        definition: UniverseDefinition,
        custody: Custody,
        mail: PrintedMail,
    ): Promise<AsyncDisposableStack> {
        // serve the universe's files from a local bucket over S3
        const { deployment, environment } = this;
        await using stack = new AsyncDisposableStack();
        const directory = join(this.directory, WORKERD_DIRECTORY);
        const files = stack.use(
            await LocalBucket.open(join(directory, "files"), "universe", environment),
        );
        const bucket = `destack-${deployment.name}-files-universe`;
        const s3 = new S3Server({
            region: PLACEHOLDER_REGION,
            credentials: async (id) =>
                id === PLACEHOLDER_KEY.accessKeyId ? PLACEHOLDER_KEY : undefined,
            open: async (name) => (name === bucket ? files : undefined),
            environment,
        });
        const listener = environment.server.http({
            hostname: LOOPBACK,
            port: 0,
            fetch: (request) => s3.fetch(request),
        });
        stack.defer(() => listener.stop(true));

        // configure the scripts over placeholder outputs, leaving out the tails, which only Cloudflare runs
        const releases = await build.writeAssets(join(directory, "assets"));
        const universe: UniverseOutputs = {
            account: PLACEHOLDER_IDENTIFIER,
            hyperdrive: PLACEHOLDER_IDENTIFIER,
            files: { endpoint: listener.url, bucket, region: PLACEHOLDER_REGION },
            ...PLACEHOLDER_WORKER,
        };
        const residency = (served: PlatformResidency): ResidencyOutputs => ({
            account: PLACEHOLDER_IDENTIFIER,
            bucket: `destack-${deployment.name}-${served.code}`,
            host: new URL(deployment.machineOrigin(served)).host,
            hyperdrive: PLACEHOLDER_IDENTIFIER,
            ...PLACEHOLDER_WORKER,
        });
        const scripts = WorkerScript.all(deployment, universe, residency, releases).filter(
            ([script]) => script.target.kind !== "tail",
        );
        const modules = WorkerScript.modules(build, definition);

        // start every script at once, the universe reached through the platform residency's machine
        await Promise.all(
            scripts.map(([script, configuration]) =>
                this.#startWorker(script, configuration, modules, custody, mail, stack),
            ),
        );

        return stack.move();
    }

    /** Start a script's local workerd and wait until it answers, recording its start. */
    async #startWorker(
        script: WorkerScript,
        configuration: WranglerConfiguration,
        modules: SuppliedModules,
        custody: Custody,
        mail: PrintedMail,
        stack: AsyncDisposableStack,
    ): Promise<void> {
        // start it in a span, which records the start once the Worker answers
        const { deployment, environment } = this;
        const storage = join(this.directory, "workers");
        await this.scope.span(
            "platform.worker.start",
            { "platform.worker.name": script.service, "url.full": script.origin },
            async () => {
                // write the configuration without what only a deployment binds, and placeholder secrets beside it
                const local = Object.fromEntries(
                    Object.entries(configuration).filter(([key]) => !DEPLOYED_KEYS.has(key)),
                );

                // log at this process's level, as the servers of a universe on Bun do
                const level = environment.process.variables[LOG_LEVEL_VARIABLE];
                const vars = schema.record(schema.string(), schema.unknown()).parse(local["vars"]);
                const leveled =
                    level === undefined
                        ? local
                        : { ...local, vars: { ...vars, [LOG_LEVEL_VARIABLE]: level } };
                const path = await script.write(leveled, modules);
                const placeholders = deployment.providers(() => PLACEHOLDER_KEY.secretAccessKey);
                const secrets = script.secrets(custody, PLACEHOLDER_KEY, placeholders);
                const variables = Object.entries(secrets).map(
                    ([name, value]) => `${name}=${value}\n`,
                );
                await environment.files.writeTextFile(
                    resolve(path, "..", ".dev.vars"),
                    variables.join(""),
                    { mode: 0o600 },
                );

                // connect local Hyperdrive to the universe's database with the password it requires
                const hyperdrive = new URL(`${SERVER}/${this.database}`);
                hyperdrive.password ||= HYPERDRIVE_PASSWORD;

                // start local workerd, the universe's on its own port, waiting until it serves or exits
                const port =
                    script.target.kind === "universe"
                        ? this.ports.worker
                        : Number(new URL(script.origin).port);
                const child = stack.use(
                    LocalProcess.start(
                        script.service,
                        {
                            command: [
                                resolve(ROOT, "node_modules/.bin/wrangler"),
                                "dev",
                                "--local",
                                // keep each Worker's storage in the state between runs, apart from its rebuilt bundle
                                "--persist-to",
                                join(storage, script.service),
                                "--config",
                                path,
                                "--ip",
                                LOOPBACK,
                                "--port",
                                String(port),
                                "--inspector-port",
                                String(port + INSPECTOR_OFFSET),
                            ],
                            directory: ROOT,
                            variables: {
                                ...Root.inherited(
                                    environment.process.variables,
                                    WRANGLER_VARIABLES,
                                ),
                                CLOUDFLARE_HYPERDRIVE_LOCAL_CONNECTION_STRING_DATABASE:
                                    hyperdrive.href,
                                WRANGLER_SEND_METRICS: "false",
                                // register the universe's Workers with each other alone, apart from any other universe on this computer
                                WRANGLER_REGISTRY_PATH: join(storage, WORKER_REGISTRY),
                            },
                            print: (name, line) => {
                                // print every line but wrangler's access log, which spans record
                                if (!line.startsWith(WRANGLER_INFO_PREFIX)) {
                                    this.print(name, line);
                                }

                                // read the universe's sign-in codes from its lines
                                if (script.target.kind === "universe") {
                                    mail.push(line);
                                }
                            },
                        },
                        environment,
                    ),
                );
                const answered = await Promise.race([
                    script.waitReady(),
                    child.exited.then(() => undefined),
                ]);
                if (answered === undefined) {
                    throw new Error(`'${script.name}' exited before it served`);
                }
            },
            { level: "info" },
        );
    }

    /** Start this computer's machine on loopback against the universe, until it answers. */
    async #startMachine(): Promise<LocalProcess> {
        // run the checkout's machine entry against the universe, printing its lines
        const { environment, home } = this;
        const inherited = INHERITED.flatMap((variable) => {
            const value = environment.process.variables[variable];

            return value === undefined ? [] : [[variable, value] as const];
        });
        const child = LocalProcess.start(
            "machine",
            {
                command: [
                    environment.process.executable,
                    "run",
                    "--preload",
                    fileURLToPath(import.meta.resolve("@destack/package/bun/preload")),
                    "--no-env-file",
                    join(CHECKOUT, "@destack/desktop/src/machine.ts"),
                ],
                directory: join(CHECKOUT, "@destack/desktop"),
                variables: {
                    ...Object.fromEntries(inherited),
                    ...this.variables,
                    DESTACK_DIRECTORY: home.directory,
                    DESTACK_PORT: String(this.ports.machine),
                },
                print: (name, line) => this.print(name, line),
            },
            environment,
        );

        // wait until it answers, refusing a machine that exits or stays silent
        const { clock } = environment;
        const deadline = clock.now() + MACHINE_START_MILLISECONDS;
        while ((await Device.inspect(home.directory, environment)).kind !== "running") {
            const code = await Promise.race([child.exited, clock.sleep(MACHINE_POLL_MILLISECONDS)]);
            if (typeof code === "number" || clock.now() > deadline) {
                await child[Symbol.asyncDispose]();
                throw new Error("the local universe's machine did not start: read its lines above");
            }
        }
        this.print("machine", `http://${LOOPBACK}:${this.ports.machine}`);

        return child;
    }

    /** Serve a computer's requests and sockets once it starts, and the universe's at its origin. */
    #listen(
        computer: () => Computer | undefined,
        universe: HostListening | undefined,
    ): HostListening {
        // listen through the computer's host
        const listening = () => {
            const started = computer();

            return started === undefined
                ? undefined
                : present(started.machine.hibernation, "the machine's sockets").listen((request) =>
                      started.fetch(request),
                  );
        };

        // take each socket's events where it was accepted: the universe, else the computer
        const accepted = new WeakSet<object>();
        const handler = (attachment: object) =>
            universe !== undefined && accepted.has(attachment)
                ? universe.websocket
                : present(listening(), "the started machine's listening").websocket;

        return {
            fetch: async (request) => {
                // hand a request at the universe's origin to the universe, as its binding does on Workers
                if (
                    universe !== undefined &&
                    this.deployment.universe.serves(request.url) !== undefined
                ) {
                    const handed = await universe.fetch(request);
                    if (!(handed instanceof Response)) {
                        accepted.add(handed.attachment);
                    }

                    return handed;
                }

                // refuse with 503 while the computer starts
                const started = listening();
                if (started === undefined) {
                    return new Response("the machine is still starting", {
                        status: 503,
                        headers: { "retry-after": "1" },
                    });
                }

                return started.fetch(request);
            },
            // pass the accepted sockets their events
            websocket: {
                open: (socket) => handler(socket.attachment).open?.(socket),
                message: (socket, message) => handler(socket.attachment).message(socket, message),
                close: (socket, code, reason) =>
                    handler(socket.attachment).close?.(socket, code, reason),
            },
        };
    }

    /** Read the root keys of a residency's local cloud machine. */
    static #rootKey(residency: PlatformResidency): string {
        return present(MACHINE_ROOT_KEYS.get(residency.code), `the root keys of ${residency.code}`);
    }

    /** Read the universe port a local universe took at its first open, else take its block. */
    static async #port(
        name: string,
        runtime: LocalRuntime,
        environment: Environment<HostCapability>,
    ): Promise<number> {
        // read the port the universe took before
        const { files } = environment;
        const target = resolve(CHECKOUT, "target");
        const directory = resolve(target, LocalUniverse.#stored(name, runtime));
        const recorded = await LocalUniverse.#recorded(directory, environment);
        if (recorded !== undefined) {
            return recorded;
        }

        // take the default port for the default universe, else the first block no other holds
        await files.mkdir(target, { recursive: true });
        const taken = new Set<number>();
        for (const entry of await files.readDirectory(target)) {
            const other = await LocalUniverse.#recorded(resolve(target, entry.name), environment);
            if (other !== undefined) {
                taken.add(other);
            }
        }
        let port = UNIVERSE_PORT;
        if (name !== DEFAULT_NAME) {
            port += PORT_BLOCK;
            while (taken.has(port)) {
                port += PORT_BLOCK;
            }
        }

        // record it for the universe's later opens
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(
            resolve(directory, PORTS_FILE),
            JSON.stringify({ universe: port }),
        );

        return port;
    }

    /** Read the universe port a local universe recorded, absent before its first open. */
    static async #recorded(
        directory: string,
        environment: Environment<HostCapability>,
    ): Promise<number | undefined> {
        const text = await environment.files
            .readTextFile(resolve(directory, PORTS_FILE))
            .catch((error: unknown) => {
                if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                    return undefined;
                }
                throw error;
            });

        return text === undefined ? undefined : Ports.parse(JSON.parse(text)).universe;
    }

    /** Name a local universe's directory and database: its name, and its runtime unless Bun. */
    static #stored(name: string, runtime: LocalRuntime): string {
        return runtime === "bun" ? name : `${name}_${runtime}`;
    }

    /** Run work on the server's maintenance database over one connection, closed after it. */
    async #administer(work: Parameters<PostgresPool["run"]>[1]): Promise<void> {
        const { environment } = this;
        const pool = new PostgresPool(`${SERVER}/postgres`, environment, { connections: 1 });
        try {
            await pool.run(environment, work);
        } finally {
            await pool.end();
        }
    }
}
