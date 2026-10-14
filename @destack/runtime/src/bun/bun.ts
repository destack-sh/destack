import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { serve, type Server } from "bun";
import { type BunHibernation } from "./socket.ts";
import { SOCKET_HEADER } from "../workload/socket.ts";
import { BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";

import { observeProcess, startTelemetry } from "@destack/telemetry/bun";
import type { Telemetry, TelemetryOptions } from "@destack/telemetry/sdk";
import { CallError } from "../error/index.ts";
import {
    type RunnerOptions,
    type WorkloadReady,
    WorkloadRunner,
    WorkloadStart,
} from "../workload/index.ts";

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
    fetch(request: Request): Response | Promise<Response>;
    /** The WebSockets the endpoint keeps open for instances. */
    readonly hibernation?: BunHibernation;
};

/** A listener of one endpoint, as a process publishes and stops it. */
type Listener = Pick<Server<unknown>, "url" | "stop">;

/** The endpoints and lifecycle of one process. */
export interface BunProcessOptions {
    /** The endpoints to listen on. */
    readonly endpoints: readonly BunEndpoint[];
    /** The signal aborted on shutdown. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Drain requests and release the runtime. */
    close(): Promise<void>;
    /** Publish the listener addresses, in endpoint order. */
    ready?(addresses: readonly URL[]): Promise<void>;
}

/** Serve each endpoint until shutdown and drain it on close. */
export async function serveProcess(options: BunProcessOptions): Promise<void> {
    // shut down on process signals
    const listeners: Listener[] = [];
    const stopped = Promise.withResolvers<void>();
    const stop = () => stopped.resolve();
    const shutdown = () => options.shutdown();
    options.signal.addEventListener("abort", stop, { once: true });
    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
    try {
        // open every listener, on its socket or its interface's port
        for (const endpoint of options.endpoints) {
            listeners.push(listen(endpoint));
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
                process.off("SIGINT", shutdown);
                process.off("SIGTERM", shutdown);
            }
        }
    }
}

/** Listen on an endpoint's socket or port, keeping open the WebSockets its answers accept. */
function listen(endpoint: BunEndpoint): Listener {
    // answer each request plainly on an endpoint without sockets
    const { hibernation } = endpoint;
    const address =
        "unix" in endpoint
            ? { unix: endpoint.unix }
            : { hostname: endpoint.hostname, port: endpoint.port };
    if (hibernation === undefined) {
        return serve({ ...address, fetch: (request) => endpoint.fetch(request) });
    }

    // keep open the WebSockets the instances accept
    return serve({
        ...address,
        fetch: async (request, server) => {
            // answer the request, and pass on anything but an accepted upgrade
            const answer = await endpoint.fetch(request);
            const isAccepted =
                request.headers.get("upgrade") === "websocket" && answer.headers.has(SOCKET_HEADER);
            if (!isAccepted) {
                return answer;
            }

            // hold the accepted socket, refusing an upgrade Bun does not take
            return hibernation.upgrade(server, request, answer)
                ? undefined
                : new Response(null, { status: 426 });
        },
        websocket: hibernation.websocket,
    });
}

/** Run a workload as a host's first input line starts it, until shutdown. */
export async function runWorkload(
    runner: RunnerOptions,
    lines: AsyncIterator<string>,
    ready: (ready: WorkloadReady) => Promise<void>,
    socket: string,
): Promise<void> {
    // read the host's start
    const first = await lines.next();
    if (first.done === true) {
        throw new CallError("BAD_REQUEST", { message: "the host sent no start" });
    }
    const start = WorkloadStart.parse(JSON.parse(first.value));

    // start the workload of the build its host wrote into the working directory
    const build = await readBuild(process.cwd());
    await using workload = await WorkloadRunner.start(
        runner,
        start,
        build,
        startRunnerTelemetry,
        (error) => process.stderr.write(`workload failed: ${String(error)}\n`),
    );

    // serve the workload on the socket only its host reaches
    await serveProcess({
        endpoints: [{ unix: socket, fetch: (request) => workload.fetch(request) }],
        signal: workload.signal,
        shutdown: () => workload.shutdown(),
        close: () => workload.close(),
        ready: () => ready({}),
    });
}

/** Start a runner's telemetry, reporting the runner process's CPU time to its host. */
async function startRunnerTelemetry(options: TelemetryOptions): Promise<Telemetry> {
    // start the telemetry and observe the process's CPU time
    const telemetry = await startTelemetry(options);
    observeProcess(telemetry, options);

    return telemetry;
}

/** Read the build a host wrote into a directory: its manifest at the root and its files below. */
async function readBuild(directory: string): Promise<BuildReader> {
    const manifest = PackageManifest.parse(
        JSON.parse(await readFile(join(directory, MANIFEST_PATH), "utf8")),
    );

    return new BuildReader(manifest, async (path) => Bun.file(join(directory, path)).stream());
}
