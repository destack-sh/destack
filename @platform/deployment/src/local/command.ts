import { join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { CHECKOUT, developDistribution } from "@platform/distribution";
import { LocalUniverse } from "./local.ts";

/** Serve a local universe until an interrupt: `local serve [<flags of LocalUniverse>]`. */
export async function serveLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // read the local universe, recording through the host's telemetry
    const local = await LocalUniverse.of(arguments_, environment);

    // serve until the first interrupt or termination, also while it starts
    using listening = new DisposableStack();
    await local.serve(interruption(environment, listening));
    local.scope.log.info("platform.universe.stopped");
}

/** Abort at the first interrupt or termination, listening until the stack is disposed. */
function interruption(
    environment: Environment<"process">,
    listening: DisposableStack,
): AbortSignal {
    // abort with the signal's name as the reason
    const stopping = new AbortController();
    for (const name of ["SIGINT", "SIGTERM"] as const) {
        listening.use(environment.process.on(name, () => stopping.abort(name)));
    }

    return stopping.signal;
}

/** Run the desktop app on a local universe's device: `local desktop [<flags of LocalUniverse>]`. */
export async function desktopLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const local = await LocalUniverse.of(arguments_, environment);
    await developDistribution(environment, local.variables);
}

/** Run the cli on a local universe's device: `local cli [<flags of LocalUniverse>] <command>`. */
export async function cliLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // split the local universe's flags from the cli's own command
    const { local, rest: command } = await LocalUniverse.read(arguments_, environment);

    // run the cli from its sources with the terminal attached, and exit with its status
    const { process } = environment;
    const child = process.spawn([process.executable, "run", "src/main.ts", ...command], {
        workingDirectory: join(CHECKOUT, "@destack/cli"),
        variables: { ...process.variables, ...local.variables },
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
    });

    // pass an interrupt, a termination or a hangup to the cli while it runs, so it never outlives this process
    using forwarded = new DisposableStack();
    for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
        forwarded.use(process.on(signal, () => child.kill(signal)));
    }
    process.exitCode = await child.exited;
}

/** Drop a local universe's state: `local reset [<flags of LocalUniverse>]`. */
export async function resetLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const local = await LocalUniverse.of(arguments_, environment);
    await local.reset();
    local.scope.log.info("platform.universe.reset", {
        "db.namespace": local.database,
        "file.path": local.directory,
    });
}
