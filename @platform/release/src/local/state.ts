import { TelemetryScope } from "@destack/telemetry";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { PostgresPool } from "@destack/db/postgres";
import type { Environment, HostCapability } from "@destack/environment";
import { BunKeychain } from "@destack/identity/bun";
import { Device } from "@destack/machine/device";
import { Computer, StageHome } from "@destack/machine/computer";
import { DistributionPlatform } from "@destack/package";
import { Stage } from "@destack/update/release";
import { printStatus } from "../distribution/output.ts";
import { CHECKOUT } from "../distribution/distribution.ts";
import {
    Deployment,
    LOOPBACK_LAYOUT,
    type LoopbackLayout,
    type PlatformBuild,
} from "../deployment/index.ts";
import { LocalProcess } from "./process.ts";

/** The local PostgreSQL server, the test server of `just postgres up`. */
export const SERVER = "postgres://postgres@127.0.0.1:55432";

/** The local state without `--state`: database destack_local, target/local. */
const DEFAULT_STATE = "local";

/** A local state's name, a PostgreSQL identifier's tail and a directory name. */
const STATE_NAME = /^[a-z][a-z0-9_]*$/u;

/** The runtimes of a local stage's cloud machines: Bun, or workerd as production. */
const LOCAL_RUNTIMES = ["bun", "workerd"] as const;

/** The usage of a local state's flags. */
const USAGE =
    "use local <serve|reset|desktop|cli> [--runtime <bun|workerd>] [--state <name>], a name of a-z, 0-9 and _";

/** The directories of a state the universe and the cloud machines keep, which a reset removes. */
const STATE_DIRECTORIES = ["universe", "machines", "workers"] as const;

/** How long the local stage's machine may take to answer after it starts, in milliseconds. */
const MACHINE_START_MILLISECONDS = 30_000;

/** How long to wait between asks whether the local stage's machine answers, in milliseconds. */
const MACHINE_POLL_MILLISECONDS = 100;

/** The variables of this process the local stage's machine runs with. */
const INHERITED = ["HOME", "PATH", "TMPDIR", "LANG", "LC_ALL", "USER"] as const;

/** Where a local stage's cloud machines run. */
export type LocalRuntime = (typeof LOCAL_RUNTIMES)[number];

/** A state of the local stage: its universe's state, its loopback ports and its device. */
export class LocalState {
    /** The state's name as `--state` gives it. */
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
    /** The device the state's machine, desktop app and cli share. */
    readonly home: StageHome;
    /** The host the universe and the machines run on. */
    readonly environment: Environment<HostCapability>;
    /** The stage's telemetry, whose records print as status lines. */
    readonly scope: TelemetryScope;

    /** Create a state of the local stage, a workerd state apart from the Bun state of its name. */
    private constructor(
        name: string,
        runtime: LocalRuntime,
        environment: Environment<HostCapability>,
    ) {
        // keep the default Bun state in the local stage's own device directory, any other below its directory
        const stored = runtime === "bun" ? name : `${name}_${runtime}`;
        const directory = resolve(CHECKOUT, "target", stored);
        const stage = Stage.of("local");
        const user = environment.host.home;
        this.name = name;
        this.runtime = runtime;
        this.root = CHECKOUT;
        this.directory = directory;
        this.server = SERVER;
        this.database = `destack_${stored}`;
        this.layout = LOOPBACK_LAYOUT;
        this.home =
            stored === DEFAULT_STATE
                ? new StageHome(stage, user)
                : new StageHome(stage, user, resolve(directory, "device"));
        this.environment = environment;
        this.scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Read a local state from `[--runtime <bun|workerd>] [--state <name>]`, refusing more. */
    static of(arguments_: readonly string[], environment: Environment<HostCapability>): LocalState {
        // read the flags, refusing any word after them
        const { state, rest } = LocalState.read(arguments_, environment);
        if (rest.length > 0) {
            throw new Error(USAGE);
        }

        return state;
    }

    /** Read a local state from leading `--runtime` and `--state` flags, answering the rest. */
    static read(
        arguments_: readonly string[],
        environment: Environment<HostCapability>,
    ): { readonly state: LocalState; readonly rest: readonly string[] } {
        // read each leading flag and its value, refusing a flag given twice or without a value
        const flags = new Map<string, string>();
        let index = 0;
        for (
            let flag = arguments_[index];
            flag === "--runtime" || flag === "--state";
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
        const name = flags.get("--state") ?? DEFAULT_STATE;
        if (runtime === undefined || !STATE_NAME.test(name)) {
            throw new Error(USAGE);
        }
        const state = new LocalState(name, runtime, environment);

        return { state, rest: arguments_.slice(index) };
    }

    /** The directory of the distribution the state's machines ship. */
    get distribution(): string {
        return join(this.directory, "distribution");
    }

    /** The variables that put the desktop app and the cli on the state's device. */
    get variables(): Record<string, string> {
        return {
            DESTACK_STAGE: this.home.stage.name,
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

    /** Start the state's machine on its loopback port against a universe. */
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
                    DESTACK_STAGE: home.stage.name,
                    DESTACK_DIRECTORY: home.directory,
                    DESTACK_UNIVERSE: universe,
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
                throw new Error("the local stage's machine did not start: read its lines above");
            }
        }

        return child;
    }

    /** Create the state's database unless the server has it. */
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

    /** Drop the state's database, ending its connections. */
    async dropDatabase(): Promise<void> {
        await this.#administer(async (sql) => {
            await sql.unsafe(`DROP DATABASE IF EXISTS "${this.database}" WITH (FORCE)`);
        });
    }

    /** Drop the state's database, universe and cloud machines, and erase its machine. */
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

        // erase the state's machine, refused while it runs, then the database
        await Computer.erase(
            this.home.directory,
            (service) => new BunKeychain(service, environment.files, { kind: "system" }),
            environment,
        );
        await this.dropDatabase();

        // remove the state, leaving the build store and the distribution, which content addresses
        for (const state of STATE_DIRECTORIES) {
            const path = join(this.directory, state);
            await environment.files.remove(path, { recursive: true, force: true });
        }
    }

    /** Print how to open the state in a browser, the desktop app and the cli. */
    printReady(): void {
        // show the runtime and the state in the commands of a state other than the default
        const runtime = this.runtime === "bun" ? "" : ` --runtime ${this.runtime}`;
        const name = this.name === DEFAULT_STATE ? "" : ` --state ${this.name}`;
        const state = `${runtime}${name}`;
        this.print("ready", `http://127.0.0.1:${this.layout.universe}/ opens Destack in a browser`);
        this.print("ready", `just local desktop${state} runs the desktop app`);
        this.print("ready", `just local cli${state} <command> runs the cli`);
        this.print("ready", "Ctrl-C stops everything");
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
