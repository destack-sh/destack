import { TelemetryScope } from "@destack/telemetry";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { PostgresPool } from "@destack/db/postgres";
import { type Environment, FileSystemError, type HostCapability } from "@destack/environment";
import { BunKeychain } from "@destack/identity/bun";
import { Device } from "@destack/machine/device";
import { Computer, UniverseHome } from "@destack/machine/computer";
import { DistributionPlatform } from "@destack/package";
import { Universe } from "@destack/update/release";
import { printStatus } from "../distribution/output.ts";
import { CHECKOUT } from "../distribution/distribution.ts";
import { KeyPair, LocalKeyring, MemoryKeychain } from "@destack/identity";
import { MachineKey } from "@destack/account/client";
import { present, schema } from "@destack/schema";
import { RESIDENCIES, type PlatformResidency } from "@platform/stack";
import {
    Custody,
    Deployment,
    LOOPBACK_LAYOUT,
    LoopbackLayout,
    type PlatformBuild,
} from "../deployment/index.ts";
import { LocalProcess } from "./process.ts";

/** The local PostgreSQL server, the test server of `just postgres up`. */
export const SERVER = "postgres://postgres@127.0.0.1:55432";

/** The local universe without `--universe`: database destack_local, target/local. */
const DEFAULT_NAME = "local";

/** A local universe's name, a PostgreSQL identifier's tail and a directory name. */
const NAME = /^[a-z][a-z0-9_]*$/u;

/** The ports between two local universes' universe ports: their five services, rounded up. */
const PORT_BLOCK = 10;

/** The file in a local universe's directory with the universe port it took at its first open. */
const PORTS_FILE = "ports.json";

/** The ports file: the universe's port, which its other services follow. */
const Ports = schema.object({ universe: schema.number().int().positive() });

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
    "use local <serve|reset|desktop|cli> [--runtime <bun|workerd>] [--universe <name>], a name of a-z, 0-9 and _";

/** The directories of a local universe's state, which a reset removes. */
const STATE_DIRECTORIES = ["universe", "machines", "workers"] as const;

/** How long the local universe's machine may take to answer after it starts, in milliseconds. */
const MACHINE_START_MILLISECONDS = 30_000;

/** How long to wait between asks whether the local universe's machine answers, in milliseconds. */
const MACHINE_POLL_MILLISECONDS = 100;

/** The variables of this process the local universe's machine runs with. */
const INHERITED = ["HOME", "PATH", "TMPDIR", "LANG", "LC_ALL", "USER"] as const;

/** Where a local universe's cloud machines run. */
export type LocalRuntime = (typeof LOCAL_RUNTIMES)[number];

/** A universe on this computer: its state, its loopback ports and its device. */
export class LocalUniverse {
    /** The universe's name as `--universe` gives it. */
    readonly name: string;
    /** Where the cloud machines run. */
    readonly runtime: LocalRuntime;
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
        layout: LoopbackLayout,
        environment: Environment<HostCapability>,
    ) {
        // keep the default Bun universe's device in its own home directory, any other below its directory
        const stored = LocalUniverse.#stored(name, runtime);
        const directory = resolve(CHECKOUT, "target", stored);
        const universe = new Universe("local", `http://127.0.0.1:${layout.universe}`);
        const user = environment.host.home;
        this.name = name;
        this.runtime = runtime;
        this.root = CHECKOUT;
        this.directory = directory;
        this.server = SERVER;
        this.database = `destack_${stored}`;
        this.layout = layout;
        this.home =
            stored === DEFAULT_NAME
                ? new UniverseHome(universe, user)
                : new UniverseHome(universe, user, resolve(directory, "device"));
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
        const local = new LocalUniverse(name, runtime, LoopbackLayout.at(port), environment);

        return { local, rest: arguments_.slice(index) };
    }

    /** The directory of the distribution the universe's machines ship. */
    get distribution(): string {
        return join(this.directory, "distribution");
    }

    /** The variables that put the desktop app and the cli on the universe's device and origin. */
    get variables(): Record<string, string> {
        return {
            DESTACK_UNIVERSE: this.home.universe.name,
            DESTACK_UNIVERSE_ORIGIN: this.home.universe.origin,
            ...(this.home.isCustom ? { DESTACK_DIRECTORY: this.home.directory } : {}),
            DESTACK_DISTRIBUTION: this.distribution,
        };
    }

    /** Print a status line under the name of the process it concerns. */
    print(name: string, line: string): void {
        printStatus(name, line, this.environment);
    }

    /** Write a build's distribution for this host, with the layer its machines read in place. */
    async writeDistribution(build: PlatformBuild): Promise<void> {
        await build.writeDistribution(
            this.distribution,
            DistributionPlatform.of(this.environment.host),
        );
        await build.writeLayer(this.distribution);
    }

    /** Start the universe's machine on its loopback port against the universe's origin. */
    async startMachine(universe: string): Promise<LocalProcess> {
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
                    join(this.root, "@destack/desktop/src/machine.ts"),
                ],
                directory: join(this.root, "@destack/desktop"),
                variables: {
                    ...Object.fromEntries(inherited),
                    DESTACK_UNIVERSE: home.universe.name,
                    DESTACK_DIRECTORY: home.directory,
                    DESTACK_UNIVERSE_ORIGIN: universe,
                    DESTACK_PORT: String(this.layout.machine),
                    DESTACK_DISTRIBUTION: this.distribution,
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

        return child;
    }

    /** Create the universe's database unless the server has it. */
    async createDatabase(): Promise<void> {
        await this.#administer(async (sql) => {
            const existing = await sql<
                { readonly found: number }[]
            >`SELECT 1 AS found FROM pg_database WHERE datname = ${this.database}`;
            if (existing.length === 0) {
                await sql.unsafe(`CREATE DATABASE "${this.database}"`);
            }
        });
    }

    /** Drop the universe's database, ending its connections. */
    async dropDatabase(): Promise<void> {
        await this.#administer(async (sql) => {
            await sql.unsafe(`DROP DATABASE IF EXISTS "${this.database}" WITH (FORCE)`);
        });
    }

    /** Drop the universe's database, universe and cloud machines, and erase its machine. */
    async reset(): Promise<void> {
        // refuse while the universe answers
        const { environment } = this;
        const { issuer } = Deployment.loopback(this.layout, environment);
        const isRunning = await environment.network.fetch(new Request(issuer)).then(
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
        await this.dropDatabase();

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
        this.print("ready", `http://127.0.0.1:${this.layout.universe}/ opens Destack in a browser`);
        this.print("ready", `just local desktop${flags} runs the desktop app`);
        this.print("ready", `just local cli${flags} <command> runs the cli`);
        this.print("ready", "Ctrl-C stops everything");
    }

    /** Read the root keys of a residency's local cloud machine. */
    static #rootKey(residency: PlatformResidency): string {
        return present(MACHINE_ROOT_KEYS.get(residency.code), `the root keys of ${residency.code}`);
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
        let port = LOOPBACK_LAYOUT.universe;
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
        const pool = new PostgresPool(`${this.server}/postgres`, environment, { connections: 1 });
        try {
            await pool.run(environment, work);
        } finally {
            await pool.end();
        }
    }
}
