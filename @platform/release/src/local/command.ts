import { join, resolve } from "node:path";
import { StageHome } from "@destack/machine/computer";
import { Stage } from "@destack/update/release";
import { Environment, type HostCapability, Telemetry } from "@destack/environment";
import { SeverityNumber } from "@destack/telemetry";
import { printStatus } from "../distribution/output.ts";
import { LOOPBACK_LAYOUT } from "../deployment/deployment.ts";
import { CHECKOUT } from "../distribution/distribution.ts";
import { developDistribution } from "../distribution/develop.ts";
import { type DevelopmentOptions, LocalUniverse } from "./local.ts";

/** The local PostgreSQL server, the test server of `just postgres up`. */
export const SERVER = "postgres://postgres@127.0.0.1:55432";

/** The local state without `--state`: database destack_local, target/local. */
const DEFAULT_STATE = "local";

/** A local state's name, a PostgreSQL identifier's tail and a directory name. */
const STATE_NAME = /^[a-z][a-z0-9_]*$/u;

/** Serve a local universe until an interrupt: `local serve [--state <name>]`. */
export async function serveLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // print the warnings and failures of the universe and the cloud machines this process runs
    const telemetry = Telemetry.local(
        environment.clock,
        (line, severity) => {
            if (severity >= SeverityNumber.WARN) {
                printStatus("stage", line.trimEnd(), environment);
            }
        },
        environment.telemetry.context,
    );
    const staged = Environment.instrument({ ...environment, telemetry }, telemetry);

    // start the universe and its cloud machines, then the local stage's machine
    const options = localOptions(arguments_, staged);
    const universe = await LocalUniverse.start({
        ...options,
        root: CHECKOUT,
        print: (title, line) => printStatus(title, line, environment),
    });
    const machine = await universe.startMachine(options.home, LOOPBACK_LAYOUT.machine);

    // print where everything answers
    for (const endpoint of universe.endpoints) {
        printStatus(endpoint.name, endpoint.url, environment);
    }
    printStatus("machine", `http://127.0.0.1:${LOOPBACK_LAYOUT.machine}`, environment);
    // print how to open the stage in a browser, the desktop app and the cli, each on this state
    const state = options.name === DEFAULT_STATE ? "" : ` --state ${options.name}`;
    printStatus(
        "ready",
        `http://127.0.0.1:${LOOPBACK_LAYOUT.universe}/ opens Destack in a browser`,
        environment,
    );
    printStatus("ready", `just local desktop${state} runs the desktop app`, environment);
    printStatus("ready", `just local cli${state} <command> runs the cli`, environment);
    printStatus("ready", "Ctrl-C stops everything", environment);

    // serve until an interrupt or a termination
    const stopping = Promise.withResolvers<string>();
    const interrupt = environment.process.on("SIGINT", () => stopping.resolve("SIGINT"));
    const termination = environment.process.on("SIGTERM", () => stopping.resolve("SIGTERM"));
    const signal = await stopping.promise;
    interrupt[Symbol.dispose]();
    termination[Symbol.dispose]();

    // stop everything
    printStatus(
        "stop",
        `${signal}: stopping the machine, the cloud machines and the universe`,
        environment,
    );
    await machine[Symbol.asyncDispose]();
    await universe[Symbol.asyncDispose]();
    printStatus("stop", "stopped", environment);
}

/** Run the desktop app on a local state's device: `local desktop [--state <name>]`. */
export async function desktopLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const options = localOptions(arguments_, environment);
    await developDistribution(environment, deviceVariables(options));
}

/** Run the cli on a local state's device: `local cli [--state <name>] <command>`. */
export async function cliLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // split the state from the cli's own command
    const isFlagged = arguments_[0] === "--state";
    const options = localOptions(isFlagged ? arguments_.slice(0, 2) : [], environment);
    const command = arguments_.slice(isFlagged ? 2 : 0);

    // run the cli from its sources with the terminal attached, and exit with its status
    const { process } = environment;
    const child = process.spawn([process.executable, "run", "src/main.ts", ...command], {
        workingDirectory: join(CHECKOUT, "@destack/cli"),
        variables: { ...process.variables, ...deviceVariables(options) },
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
    });
    process.exitCode = await child.exited;
}

/** Drop a local universe's state: `local reset [--state <name>]`. */
export async function resetLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const options = localOptions(arguments_, environment);
    await LocalUniverse.reset(options);
    printStatus(
        "reset",
        `dropped ${options.database}, removed ${options.directory} and its machine`,
        environment,
    );
}

/** Read a local state's options from `[--state <name>]`, refusing anything else. */
function localOptions(
    arguments_: readonly string[],
    environment: Environment<HostCapability>,
): Omit<DevelopmentOptions, "root" | "print"> & {
    readonly home: StageHome;
    readonly name: string;
} {
    // read the state's name
    const [flag, given, ...rest] = arguments_;
    const name = given ?? DEFAULT_STATE;
    const isFlagged = flag === undefined || (flag === "--state" && given !== undefined);
    if (!isFlagged || !STATE_NAME.test(name) || rest.length > 0) {
        throw new Error(
            "use local <serve|reset|desktop|cli> [--state <name>], a name of a-z, 0-9 and _",
        );
    }

    // keep the default state in the local stage's own device directory, any other below its directory
    const directory = resolve(CHECKOUT, "target", name);
    const stage = Stage.of("local");
    const user = environment.host.home;

    return {
        name,
        directory,
        server: SERVER,
        database: `destack_${name}`,
        home:
            name === DEFAULT_STATE
                ? new StageHome(stage, user)
                : new StageHome(stage, user, resolve(directory, "device")),
        layout: LOOPBACK_LAYOUT,
        environment,
    };
}

/** The variables that put the desktop app and the cli on a local state's device. */
function deviceVariables(options: ReturnType<typeof localOptions>): Record<string, string> {
    return {
        DESTACK_STAGE: options.home.stage.name,
        ...(options.home.isCustom ? { DESTACK_DIRECTORY: options.home.directory } : {}),
        DESTACK_DISTRIBUTION: LocalUniverse.distribution(options),
    };
}
