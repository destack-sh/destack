import { join } from "node:path";
import { type Environment, type HostCapability, waitAborted } from "@destack/environment";
import { Deployment } from "../deployment/deployment.ts";
import { CHECKOUT } from "../distribution/distribution.ts";
import { developDistribution } from "../distribution/develop.ts";
import { BunCloud } from "./cloud.ts";
import { LocalUniverse } from "./local.ts";

/** Serve a local universe until an interrupt: `local serve [<flags of LocalUniverse>]`. */
export async function serveLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // read the universe's local, recording through the host's telemetry
    const local = await LocalUniverse.of(arguments_, environment);

    // serve the universe and the cloud machines as Workers under workerd, as production runs them
    if (local.runtime === "workerd") {
        const deployment = Deployment.loopback(local.layout, environment);

        // keep the local between runs, as a Bun local universe does: its database, its keys and its Workers' storage
        await local.createDatabase();
        const custody = await local.custody();
        const storage = join(local.directory, "workers");
        const print = (title: string, line: string) => local.print(title, line);

        // stop at the first interrupt or termination, also while the Workers start
        using listening = new DisposableStack();
        const stopping = interruption(environment, listening);
        await deployment.serve(
            local.server,
            local.database,
            custody,
            storage,
            print,
            async (build) => {
                // ship the served build on the state's machine, started against the Workers
                await local.writeDistribution(build);
                await using machine = await local.startMachine(deployment.universe.origin);

                // serve until an interrupt or a termination
                local.printReady();
                await waitAborted(stopping);
            },
            stopping,
        );

        return;
    }

    // start the universe and its cloud machines, then the state's machine
    const universe = await BunCloud.start(local);
    const machine = await local.startMachine(universe.deployment.universe.origin);

    // print where everything answers
    for (const endpoint of universe.endpoints) {
        local.print(endpoint.name, endpoint.url);
    }
    local.print("machine", `http://127.0.0.1:${local.layout.machine}`);
    local.printReady();

    // serve until an interrupt or a termination
    using listening = new DisposableStack();
    const stopping = interruption(environment, listening);
    await waitAborted(stopping);

    // stop everything
    local.scope.log.info("platform.universe.signaled", {
        "platform.signal": String(stopping.reason),
    });
    await machine[Symbol.asyncDispose]();
    await universe[Symbol.asyncDispose]();
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

/** Run the desktop app on a local state's device: `local desktop [<flags of LocalUniverse>]`. */
export async function desktopLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const local = await LocalUniverse.of(arguments_, environment);
    await developDistribution(environment, local.variables);
}

/** Run the cli on a local state's device: `local cli [<flags of LocalUniverse>] <command>`. */
export async function cliLocal(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // split the local from the cli's own command
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

/** Drop a local universe's local: `local reset [<flags of LocalUniverse>]`. */
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
