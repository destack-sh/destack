import type { Environment, Listener, Server } from "@destack/environment";
import { ObjectError } from "../error/error.ts";
import { type HibernationHost, SOCKET_HEADER } from "../workload/socket.ts";

import { observeProcess, startTelemetry } from "@destack/telemetry/bun";
import type { Telemetry, TelemetryOptions } from "@destack/telemetry/sdk";

import {
    type RunnerOptions,
    type WorkloadReady,
    WorkloadRunner,
    WorkloadStart,
} from "../workload/index.ts";

import type {} from "@destack/package/import-meta";
import { BuildReader } from "@destack/package/manifest";

/** A request handler served on a network interface's port or on a Unix socket. */
export type BunEndpoint = (
    | {
          /** The network interface. */
          readonly hostname: string;
          /** The port, or zero for an ephemeral one. */
          readonly port: number;
      }
    | {
          /** The Unix socket's path. */
          readonly unix: string;
      }
) & {
    /** Handle one request. */
    readonly fetch: (request: Request) => Response | Promise<Response>;
    /** The WebSockets the endpoint hibernates for instances. */
    readonly hibernation?: HibernationHost;
};

/** The endpoints and lifecycle of one process. */
export interface BunProcessOptions {
    /** The endpoints to listen on. */
    readonly endpoints: readonly BunEndpoint[];
    /** The process's environment: the server it listens with and the signals it stops on. */
    readonly environment: Environment<"files" | "process" | "server">;
    /** The signal aborted on shutdown. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    readonly shutdown: () => void;
    /** Drain requests and release the runtime. */
    readonly close: () => Promise<void>;
    /** Publish the listener addresses, in endpoint order. */
    readonly ready?: (addresses: readonly URL[]) => Promise<void>;
}

/** Serve each endpoint until shutdown and drain it on close. */
export async function serveProcess(options: BunProcessOptions): Promise<void> {
    // shut down on process signals
    const listeners: Pick<Listener<unknown>, "url" | "stop">[] = [];
    const stopped = Promise.withResolvers<void>();
    const stop = () => stopped.resolve();
    const shutdown = () => options.shutdown();
    options.signal.addEventListener("abort", stop, { once: true });
    const { process: host, server } = options.environment;
    using signals = new DisposableStack();
    signals.use(host.on("SIGINT", shutdown));
    signals.use(host.on("SIGTERM", shutdown));
    try {
        // open every listener, on its socket or its interface's port
        for (const endpoint of options.endpoints) {
            listeners.push(listen(server, endpoint));
        }
        await options.ready?.(listeners.map((listener) => listener.url));
        if (!options.signal.aborted) {
            await stopped.promise;
        }
    } finally {
        // drain requests and close
        try {
            await options.close();
        } finally {
            try {
                await Promise.all(listeners.map((listener) => listener.stop(true)));
            } finally {
                options.signal.removeEventListener("abort", stop);
            }
        }
    }
}

/** Listen on an endpoint's socket or port, hibernating the WebSockets its answers accept. */
function listen(server: Server, endpoint: BunEndpoint): Pick<Listener<unknown>, "url" | "stop"> {
    // answer each request plainly on an endpoint without sockets
    const { hibernation } = endpoint;
    const address =
        "unix" in endpoint
            ? { unix: endpoint.unix }
            : { hostname: endpoint.hostname, port: endpoint.port };
    if (hibernation === undefined) {
        return server.http({ ...address, fetch: (request) => endpoint.fetch(request) });
    }

    // hibernate the WebSockets the instances accept
    return server.http({
        ...address,
        fetch: async (request, listener) => {
            // answer the request, and pass on anything but an accepted upgrade
            const answer = await endpoint.fetch(request);
            const isAccepted =
                request.headers.get("upgrade") === "websocket" && answer.headers.has(SOCKET_HEADER);
            if (!isAccepted) {
                return answer;
            }

            // hibernate the accepted socket, refusing an upgrade Bun does not take
            return hibernation.upgrade(listener, request, answer)
                ? undefined
                : new Response(null, { status: 426 });
        },
        websocket: hibernation.websocket,
    });
}

/** Run a workload as a host's first input line starts it, until shutdown. */
export async function runWorkload(
    runner: RunnerOptions & { readonly environment: Environment<"files" | "process" | "server"> },
    lines: AsyncIterator<string>,
    ready: (ready: WorkloadReady) => Promise<void>,
    socket: string,
): Promise<void> {
    // read the host's start
    const first = await lines.next();
    if (first.done === true) {
        throw ObjectError.of("START_MISSING", "the host sent no start message");
    }
    const start = WorkloadStart.parse(JSON.parse(first.value));

    // start the workload of the build its host wrote into the working directory
    const { process: host } = runner.environment;
    const build = await BuildReader.openDirectory(
        host.workingDirectory,
        runner.environment.files,
        runner.environment.crypto,
    );
    await using workload = await WorkloadRunner.start(
        runner,
        start,
        build,
        (options) => startRunnerTelemetry(options, runner.environment),
        (error) => host.stderr.writeSync(`workload failed: ${String(error)}\n`),
    );

    // serve the workload on the socket only its host can open
    await serveProcess({
        endpoints: [{ unix: socket, fetch: (request) => workload.fetch(request) }],
        environment: runner.environment,
        signal: workload.signal,
        shutdown: () => workload.shutdown(),
        close: () => workload.close(),
        ready: () => ready({}),
    });
}

/** Start a runner's telemetry, reporting the runner process's CPU time to its host. */
async function startRunnerTelemetry(
    options: TelemetryOptions,
    environment: Environment<"process">,
): Promise<Telemetry> {
    // start the telemetry and observe the process's CPU time
    const telemetry = await startTelemetry(options, environment);
    observeProcess(telemetry, options, environment);

    return telemetry;
}
