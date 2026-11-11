import { PackageManifest } from "@destack/package/manifest";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { inspect } from "node:util";
import { MachineKey } from "@destack/account/client";
import { PostgresPool } from "@destack/db/postgres";
import { DESKTOP_OBJECTS } from "@destack/desktop/object";
import { KeyPair, type Keychain, LocalKeyring, MemoryKeychain } from "@destack/identity";
import { BunKeychain } from "@destack/identity/bun";
import {
    Computer,
    type ComputerOptions,
    MachineProcess,
    MACHINE_KEYS,
    type StageHome,
    ROOT_KEYS,
} from "@destack/machine/computer";
import { Device, type MachineEnrollment } from "@destack/machine/device";
import { PrintTransport } from "@destack/mail/print";
import { present } from "@destack/schema";
import { Stage } from "@destack/update/release";
import { Toolchain } from "@destack/check/toolchain";
import { BunUniverse } from "./universe.ts";
import { Custody, PlatformBuild } from "../deployment/index.ts";
import { DISTRIBUTION_LAYER, DistributionManifest } from "@destack/package";
import { BunDistribution } from "@destack/package/bun";
import { BuildLayer } from "@destack/build/store";
import { Deployment, type LoopbackLayout } from "../deployment/index.ts";
import { PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { DOMAINS } from "@destack/account/address";
import type { Environment, HostCapability } from "@destack/environment";
import { LocalProcess } from "./process.ts";
import { Publication } from "./publication.ts";
import { Issuer } from "@destack/account/client";
import { Bookmark } from "@destack/object/client";
import { PrintedMail } from "./mail.ts";
import { Operator } from "./operator.ts";

/** The state a reset removes below the local directory, keeping the store and distribution. */
const STATE = ["universe", "machines", "publication.json"] as const;

/** The local stage. */
const STAGE = Stage.of("local");

/** The root keys of the local universe, which only this computer uses. */
const UNIVERSE_ROOT_KEY =
    '{"active":1,"keys":{"1":"DHDaqs60kZjIDOmpchcC6lggMf8L+sHPgayD5fqCuN0="}}';

/** The root keys of the local cloud machines, which only this computer uses, by residency code. */
const MACHINE_ROOT_KEYS: ReadonlyMap<string, string> = new Map([
    ["eu", '{"active":1,"keys":{"1":"TO1o+AlghKysP7TyW2QfPhOJOOBa/NRh7/a02fsCgQs="}}'],
    ["us", '{"active":1,"keys":{"1":"w20+AFE9NraWIlc9hkKZfGfhO062wFMYkoG5HJo8K2o="}}'],
]);

/** How long the local stage's machine may take to answer after it starts, in milliseconds. */
const MACHINE_START_MILLISECONDS = 30_000;

/** How long a started cloud machine may take to replicate its enabled status, in ms. */
const MACHINE_SERVE_MILLISECONDS = 5_000;

/** How long to wait between asks whether the local stage's machine answers, in milliseconds. */
const MACHINE_POLL_MILLISECONDS = 100;

/** The variables of this process the local stage's machine runs with. */
const INHERITED = ["HOME", "PATH", "TMPDIR", "LANG", "LC_ALL", "USER"] as const;

/** Where the local universe keeps its state and where its processes listen. */
export interface DevelopmentOptions {
    /** The repository's root, whose machine entry runs. */
    readonly root: string;
    /** The directory keeping the universe's history and the cloud machines' state. */
    readonly directory: string;
    /** The PostgreSQL server's URL, without a database. */
    readonly server: string;
    /** The database keeping the account service's tables. */
    readonly database: string;
    /** The loopback port each of the universe's services listens on. */
    readonly layout: LoopbackLayout;
    /** Print a status line under the name of the process it concerns. */
    readonly print: (name: string, line: string) => void;
    /** The host the universe and the machines run on. */
    readonly environment: Environment<HostCapability>;
}

/** A process of the local universe and the URL it answers at. */
export interface DevelopmentEndpoint {
    /** The process's name, such as universe or eu-central. */
    readonly name: string;
    /** The URL it answers at. */
    readonly url: string;
}

/** The local universe of `just local`. */
export class LocalUniverse implements AsyncDisposable {
    /** The processes and the URLs they answer at, in start order. */
    readonly endpoints: readonly DevelopmentEndpoint[];
    /** The local stage's deployment on its loopback layout. */
    readonly deployment: Deployment;
    /** Where the universe keeps its state and how it prints. */
    readonly #options: DevelopmentOptions;
    /** Close what started, last started first. */
    readonly #stack: AsyncDisposableStack;

    /** Start from a universe and what stops it. */
    private constructor(
        endpoints: readonly DevelopmentEndpoint[],
        deployment: Deployment,
        options: DevelopmentOptions,
        stack: AsyncDisposableStack,
    ) {
        // keep the endpoints, the deployment, the options and the stack
        this.endpoints = endpoints;
        this.deployment = deployment;
        this.#options = options;
        this.#stack = stack;
    }

    /** Create the database, and start the universe and its cloud machines. */
    static async start(options: DevelopmentOptions): Promise<LocalUniverse> {
        // create the database once
        const local = Deployment.loopback(options.layout, options.environment);
        await using stack = new AsyncDisposableStack();
        await LocalUniverse.#create(options);

        // build the local releases from the checkout, a package's version moving on only where it changed
        const publication = await Publication.read(options.directory, options.environment.files);
        options.print("build", "building the first-party releases");
        const build = stack.use(
            await PlatformBuild.build(options.root, options.environment, publication.build),
        );

        // record the build's local versions, then report the build
        await publication.assign(build);
        options.print("build", `built ${build.readers.length} releases`);

        // start the universe as its declaration declares it over the fixed local keys, printing its mail
        const mail = new PrintedMail();
        const custody = await LocalUniverse.#custody(options.environment);
        const declaration = await local.universe(
            Operator.LOCAL,
            custody,
            build.stacks(STAGE.tag),
            await local.homes(build),
        );
        // serve the origin's other paths from the platform residency's machine once it started, as the edge does
        const clouds = new Map<string, Computer>();
        const edge = async (request: Request) => {
            // address the request at the universe's domain, which the platform's routes name
            const platform = clouds.get(PLATFORM.residencyId);
            const requested = new URL(request.url);
            const path = `${requested.pathname}${requested.search}`;
            const routed = new URL(path, `https://${DOMAINS.universe}`);

            // present the browser's own origin as the universe's, as a reverse proxy changing the origin does
            const headers = new Headers(request.headers);
            if (headers.get("origin") === requested.origin) {
                headers.set("origin", routed.origin);
            }
            const referer = headers.get("referer");
            if (referer?.startsWith(`${requested.origin}/`) === true) {
                headers.set("referer", `${routed.origin}${referer.slice(requested.origin.length)}`);
            }

            return platform === undefined
                ? new Response("the platform residency's machine is still starting", {
                      status: 503,
                      headers: { "retry-after": "1" },
                  })
                : await platform.fetch(new Request(new Request(routed, request), { headers }));
        };
        const universe = stack.use(
            await BunUniverse.start({
                edge,
                environment: options.environment,
                definition: declaration,
                origin: local.universeOrigin,
                database: `${options.server}/${options.database}`,
                rootKey: UNIVERSE_ROOT_KEY,
                providers: local.providers(
                    (id) => options.environment.process.variables[Deployment.providerSecret(id)],
                ),
                mail: new PrintTransport((text) => {
                    mail.push(text);
                    options.print("mail", text);
                }),
                directory: join(options.directory, "universe"),
                report: (error) =>
                    options.print("universe", inspect(error, { getters: true, depth: 6 })),
                shipped: {
                    releases: DistributionManifest.releases(await build.distribution()),
                    open: (manifest) => build.store.contents(manifest),
                },
            }),
        );
        const endpoints: DevelopmentEndpoint[] = [{ name: "universe", url: universe.url.origin }];

        // report the universe
        options.print("universe", "started");

        // write the distribution once, then start each residency's cloud machine shipping it
        const directory = LocalUniverse.#distribution(options);
        await build.writeDistribution(directory);
        await build.writeLayer(directory);
        const { environment } = build.store;
        const distribution = {
            shipped: await BunDistribution.read(directory, environment.files),
            layer: await BuildLayer.open(join(directory, DISTRIBUTION_LAYER), environment),
        };
        options.print("build", "wrote the distribution the machines ship");
        for (const residency of RESIDENCIES) {
            const started = await LocalUniverse.#startCloud(
                residency,
                distribution,
                local,
                custody,
                options,
            );
            stack.use(started.stack);
            clouds.set(residency.code, started.machine);

            // wait until it serves, refusing at the bound with why it does not
            const window = options.environment.clock.timeout(MACHINE_SERVE_MILLISECONDS);
            await started.machine.machine.health.waitUp(window);
            options.print(residency.machine.name, "up");
            endpoints.push({
                name: residency.machine.name,
                url: local.machineOrigin(residency),
            });
        }

        // sign the operator in, then publish the releases into the forge as the platform's owner
        const send = (request: Request) => options.environment.network.fetch(request);
        const issuer = new Issuer(local.issuer, send, options.environment, new Bookmark());
        const operator = await Operator.signIn(issuer, mail, options.environment);
        const print = (line: string) => options.print("publish", line);
        await Publication.release(
            build,
            STAGE.tag,
            local.forges(custody, build, operator),
            async (reader) => {
                const { name, version } = PackageManifest.package(reader.manifest);
                print(`${name}@${version}`);
            },
        );

        return new LocalUniverse(endpoints, local, options, stack.move());
    }

    /** Drop the local universe's database and state, and the local stage's machine. */
    static async reset(
        options: Pick<
            DevelopmentOptions,
            "directory" | "server" | "database" | "layout" | "environment"
        > & { readonly home: StageHome },
    ) {
        // refuse while the universe answers
        const { environment } = options;
        const { issuer } = Deployment.loopback(options.layout, environment);
        const isRunning = await environment.network.fetch(new Request(issuer)).then(
            () => true,
            () => false,
        );
        if (isRunning) {
            throw new Error("stop just local before resetting it");
        }

        // erase the local stage's machine, refused while it runs, then the database and the state files
        await Computer.erase(
            options.home.directory,
            (service) => new BunKeychain(service, environment.files, { kind: "system" }),
            environment,
        );
        await LocalUniverse.drop(options);

        // remove the state, leaving the build store and the distribution, which content addresses
        for (const state of STATE) {
            const path = join(options.directory, state);
            await environment.files.remove(path, { recursive: true, force: true });
        }
    }

    /** Drop the local universe's database, ending its connections. */
    static async drop(
        options: Pick<DevelopmentOptions, "server" | "database" | "environment">,
    ): Promise<void> {
        await LocalUniverse.#administer(options, async (sql) => {
            await sql.unsafe(`DROP DATABASE IF EXISTS "${options.database}" WITH (FORCE)`);
        });
    }

    /** Stop the local stage's machine, the cloud machines and the universe, last started first. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
    }

    /** Read the root keys of a residency's local cloud machine. */
    static #rootKey(residency: PlatformResidency): string {
        return present(MACHINE_ROOT_KEYS.get(residency.code), `the root keys of ${residency.code}`);
    }

    /** Create the database once. */
    static async #create(options: Pick<DevelopmentOptions, "server" | "database" | "environment">) {
        await LocalUniverse.#administer(options, async (sql) => {
            // create the database unless the server has it
            const existing = await sql<
                { readonly found: number }[]
            >`SELECT 1 AS found FROM pg_database WHERE datname = ${options.database}`;
            if (existing.length === 0) {
                await sql.unsafe(`CREATE DATABASE "${options.database}"`);
            }
        });
    }

    /** Run work on the server's maintenance database over one connection, closed after it. */
    static async #administer(
        options: Pick<DevelopmentOptions, "server" | "environment">,
        work: Parameters<PostgresPool["run"]>[1],
    ): Promise<void> {
        const { environment } = options;
        const pool = new PostgresPool(`${options.server}/postgres`, environment, {
            connections: 1,
        });
        try {
            await pool.run(environment, work);
        } finally {
            await pool.end();
        }
    }

    /** Keep the fixed local root keys in the clear with each cloud machine's key they derive. */
    static async #custody(environment: DevelopmentOptions["environment"]): Promise<Custody> {
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

    /** Keep a residency cloud machine's root keys and its key in memory. */
    static async #keychain(
        residency: PlatformResidency,
        custody: Custody,
    ): Promise<(service: string) => Keychain> {
        // open one memory keychain per service
        const keychains = new Map<string, MemoryKeychain>();
        const keychain = (service: string) => {
            const kept = keychains.get(service) ?? new MemoryKeychain();
            keychains.set(service, kept);

            return kept;
        };

        // keep the root keys and the machine key of its custody
        const { id } = residency.machine;
        const keys = custody.machine(id);
        await keychain(ROOT_KEYS).save(id, keys.rootKey);
        await keychain(MACHINE_KEYS).save(id, keys.key);

        return keychain;
    }

    /** Read the directory of the distribution the machines ship. */
    static #distribution(options: Pick<DevelopmentOptions, "directory">): string {
        return join(options.directory, "distribution");
    }

    /** Read the enrollment the universe declares for a residency's cloud machine. */
    static #enrollment(residency: PlatformResidency, local: Deployment): MachineEnrollment {
        return {
            kind: "cloud",
            universe: local.universeOrigin,
            residencyId: residency.code,
            status: "enrolled",
        };
    }

    /** Start a residency's cloud machine in this process, with the stack stopping it. */
    static async #startCloud(
        residency: PlatformResidency,
        distribution: Pick<ComputerOptions, "shipped" | "layer">,
        local: Deployment,
        custody: Custody,
        options: DevelopmentOptions,
    ): Promise<{ readonly machine: Computer; readonly stack: AsyncDisposableStack }> {
        // read the machine's root keys and its key
        await using stack = new AsyncDisposableStack();
        const { id } = residency.machine;
        const keychain = await LocalUniverse.#keychain(residency, custody);

        // listen at its origin first, answering as unavailable until it started, as the universe and other machines call it while it starts
        let started: Computer | undefined;
        const { environment } = options;
        const origin = new URL(local.machineOrigin(residency));
        const listener = environment.server.http({
            hostname: origin.hostname,
            port: Number(origin.port),
            fetch: (request) =>
                started === undefined
                    ? new Response(null, { status: 503 })
                    : started.fetch(request),
        });
        stack.defer(async () => listener.stop(true));

        // start it enrolled in the universe as the declared cloud machine of the residency, in a directory of its own
        const name = residency.machine.name;
        const now = environment.clock.now();
        const directory = join(options.directory, "machines", name);
        await new MachineProcess(directory, environment).createDirectory();
        const machine = stack.use(
            await Computer.start({
                isolation: "sandbox",
                registry: FORGE_INSTALLATION,
                directory: await environment.files.realPath(directory),
                universe: local.universeOrigin,
                port: Number(origin.port),
                environment,
                localObjects: DESKTOP_OBJECTS,
                ...distribution,
                keychain,
                toolchain: async () => Toolchain.workspace(),
                machineId: id,
                enrollment: LocalUniverse.#enrollment(residency, local),
                process: {
                    version: STAGE.tag,
                    pid: environment.process.id,
                    startedAt: now,
                },
                report: (event, error) =>
                    options.print(name, `${event}: ${inspect(error, { getters: true, depth: 6 })}`),
                shutdown: () =>
                    options.print(name, "asked to stop, which just local does on Ctrl-C"),
            }),
        );

        // serve its origin
        started = machine;

        return { machine, stack: stack.move() };
    }

    /** Start a home's local stage machine on a loopback port, shipping the build's releases. */
    async startMachine(home: StageHome, port: number): Promise<LocalProcess> {
        // run the checkout's machine entry against the local universe, printing its lines
        const options = this.#options;
        const { environment } = options;
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
                    join(options.root, "@destack/desktop/src/machine.ts"),
                ],
                directory: join(options.root, "@destack/desktop"),
                variables: {
                    ...Object.fromEntries(inherited),
                    DESTACK_STAGE: home.stage.name,
                    DESTACK_DIRECTORY: home.directory,
                    DESTACK_UNIVERSE: this.deployment.universeOrigin,
                    DESTACK_PORT: String(port),
                    DESTACK_DISTRIBUTION: LocalUniverse.#distribution(options),
                },
                print: (name, line) => options.print(name, line),
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
                throw new Error("the local stage's machine did not start: read its lines above");
            }
        }

        return child;
    }
}
