import { resolve } from "node:path";
import { StageHome } from "@destack/machine/computer";
import { Stage } from "@destack/update/release";
import type { Environment, HostCapability } from "@destack/environment";
import { printStatus } from "../distribution/output.ts";
import { LOOPBACK_LAYOUT, REPOSITORY } from "../deployment/deployment.ts";
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
    // start the universe and its cloud machines, then the local stage's machine
    const options = localOptions(arguments_, environment);
    const universe = await LocalUniverse.start({
        ...options,
        root: REPOSITORY,
        print: (title, line) => printStatus(title, line, environment),
    });
    const machine = await universe.startMachine(options.home, LOOPBACK_LAYOUT.machine);

    // print where everything answers
    for (const endpoint of universe.endpoints) {
        printStatus(endpoint.name, endpoint.url, environment);
    }
    printStatus("machine", `http://127.0.0.1:${LOOPBACK_LAYOUT.machine}`, environment);
    const device = options.home.isCustom ? ` DESTACK_DIRECTORY=${options.home.directory}` : "";
    printStatus(
        "ready",
        `cd @destack/cli && DESTACK_STAGE=local${device} bun run src/main.ts runs the cli`,
        environment,
    );
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
): Omit<DevelopmentOptions, "root" | "print"> & { readonly home: StageHome } {
    // read the state's name
    const [flag, given, ...rest] = arguments_;
    const name = given ?? DEFAULT_STATE;
    const isFlagged = flag === undefined || (flag === "--state" && given !== undefined);
    if (!isFlagged || !STATE_NAME.test(name) || rest.length > 0) {
        throw new Error("use local <serve|reset> [--state <name>], a name of a-z, 0-9 and _");
    }

    // keep the default state in the local stage's own device directory, any other below its directory
    const directory = resolve(REPOSITORY, "target", name);
    const stage = Stage.of("local");
    const user = environment.host.home;

    return {
        directory,
        releases: resolve(directory, "releases"),
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
