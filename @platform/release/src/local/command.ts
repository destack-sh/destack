import { join } from "node:path";
import { Environment, type HostCapability, Telemetry } from "@destack/environment";
import { SeverityNumber } from "@destack/telemetry";
import { printStatus } from "../distribution/output.ts";
import { Deployment } from "../deployment/deployment.ts";
import { CHECKOUT } from "../distribution/distribution.ts";
import { developDistribution } from "../distribution/develop.ts";
import { LocalUniverse } from "./local.ts";
import { LocalState } from "./state.ts";

/** Serve a local universe until an interrupt: `local serve [<flags of LocalState>]`. */
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
    const state = LocalState.of(arguments_, staged);

    // serve the universe and the cloud machines as Workers under workerd, as production runs them
    if (state.runtime === "workerd") {
        const deployment = Deployment.of("local", environment);

        // start the state fresh, as each run serves a fresh universe and fresh Workers' storage
        await state.reset();
        const print = (title: string, line: string) => state.print(title, line);
        await deployment.serve(state.server, state.database, print, async (build) => {
            // ship the served build on the state's machine, started against the Workers
            await state.writeDistribution(build);
            await using machine = await state.startMachine(deployment.universeOrigin);

            // serve until an interrupt or a termination
            state.printReady();
            await interrupted(environment);
        });

        return;
    }

    // start the universe and its cloud machines, then the state's machine
    const universe = await LocalUniverse.start(state);
    const machine = await state.startMachine(universe.deployment.universeOrigin);

    // print where everything answers
    for (const endpoint of universe.endpoints) {
        state.print(endpoint.name, endpoint.url);
    }
    state.print("machine", `http://127.0.0.1:${state.layout.machine}`);
    state.printReady();

    // serve until an interrupt or a termination
    const signal = await interrupted(environment);

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

/** Wait for an interrupt or a termination, answering the signal that came. */
async function interrupted(environment: Environment<"process">): Promise<string> {
    // resolve at the first of the two signals, then stop listening
    const stopping = Promise.withResolvers<string>();
    const interrupt = environment.process.on("SIGINT", () => stopping.resolve("SIGINT"));
    const termination = environment.process.on("SIGTERM", () => stopping.resolve("SIGTERM"));
    const signal = await stopping.promise;
    interrupt[Symbol.dispose]();
    termination[Symbol.dispose]();

    return signal;
}

/** Run the desktop app on a local state's device: `local desktop [<flags of LocalState>]`. */
export async function desktopLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const state = LocalState.of(arguments_, environment);
    await developDistribution(environment, state.variables);
}

/** Run the cli on a local state's device: `local cli [<flags of LocalState>] <command>`. */
export async function cliLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // split the state from the cli's own command
    const { state, rest: command } = LocalState.read(arguments_, environment);

    // run the cli from its sources with the terminal attached, and exit with its status
    const { process } = environment;
    const child = process.spawn([process.executable, "run", "src/main.ts", ...command], {
        workingDirectory: join(CHECKOUT, "@destack/cli"),
        variables: { ...process.variables, ...state.variables },
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
    });
    process.exitCode = await child.exited;
}

/** Drop a local universe's state: `local reset [<flags of LocalState>]`. */
export async function resetLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const state = LocalState.of(arguments_, environment);
    await state.reset();
    state.print("reset", `dropped ${state.database}, removed ${state.directory} and its machine`);
}
