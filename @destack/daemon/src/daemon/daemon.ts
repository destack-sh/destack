import { join } from "node:path";
import { writeFile, rename, rm } from "node:fs/promises";
import { localDirectory, createDirectory } from "./directory.ts";
import { DaemonError } from "../error/index.ts";
import { DaemonStore } from "./store.ts";
import { startTelemetry } from "@destack/telemetry/host";
import { SimpleLogRecordProcessor } from "@destack/telemetry/log";
import { LogExporter } from "../telemetry/log.ts";
import { TraceExporter } from "../telemetry/trace.ts";
import { SimpleSpanProcessor } from "@destack/telemetry/trace";
import { implementService } from "../server/index.ts";
import { Workload } from "@destack/service/workload";
import { runWorkload } from "@destack/service/workload/bun";
import { ResourceContext } from "@destack/resource/context";
import metadata from "../../package.json" with { type: "json" };
import definition from "../../destack.json" with { type: "json" };
import { Package } from "@destack/package";
import { FileLock } from "@destack/fs";
import { Authentication } from "../authentication/index.ts";

/** Package identity used by host authentication and telemetry. */
const manifest = Package.parse({
    id: definition.id,
    name: metadata.name,
    version: metadata.version,
});

/** Bootstrap local authority, then host its workload through the standard process adapter. */
export async function run(directory = localDirectory()): Promise<void> {
    // establish exclusive access to the local authority database
    await createDirectory(directory);
    await using cleanup = new AsyncDisposableStack();
    const lock = await FileLock.tryAcquire(join(directory, "daemon.lock"));
    if (!lock) {
        throw new DaemonError("ALREADY_RUNNING", "the Destack daemon is already running");
    }
    cleanup.defer(() => lock.close());
    const storage = await DaemonStore.open(directory);
    cleanup.defer(() => storage.close());

    // configure host telemetry before starting application services
    const telemetry = await startTelemetry({
        name: manifest.name,
        version: manifest.version,
        traces: { spanProcessors: [new SimpleSpanProcessor({ exporter: new TraceExporter() })] },
        metrics: {},
        logs: { processors: [new SimpleLogRecordProcessor({ exporter: new LogExporter() })] },
    });
    cleanup.defer(() => telemetry.shutdown());
    const logger = telemetry.scope(manifest).logger;
    const host = await storage.host.get();
    const authentication = new Authentication(host.hostId, manifest.id);
    const resources = new ResourceContext();

    // initialize domain implementations without binding sockets or installing signal handlers
    await using workload = await Workload.start(
        async (context) => {
            const controller = new AbortController();
            const delivering = storage.outbox.run(storage.history, {
                signal: controller.signal,
                report: (error) =>
                    logger.emit({
                        severityNumber: 17,
                        body: "audit delivery failed",
                        attributes: {
                            "error.type": error instanceof Error ? error.name : "unknown",
                        },
                    }),
            });
            context.defer(async () => {
                controller.abort();
                await delivering;
                await storage.outbox.flush(storage.history);
            });

            return {
                services: {
                    daemon: await implementService(storage, {
                        started: new Date().toISOString(),
                        version: manifest.version,
                        shutdown: context.shutdown,
                        shutdownSignal: context.signal,
                        credentials: authentication.credentials,
                    }),
                },
            };
        },
        {
            resources,
            service: () => ({
                audience: manifest.id,
                scope: host.hostId,
                drainTimeout: 10000,
                authenticate: async (request) => authentication.authenticate(request),
                authorizeHost: async (call) => authentication.authorize(call),
            }),
        },
    );

    // publish private discovery after the shared adapter opens the listener
    // TODO #Incomplete: persist daemon.start and daemon.stop with explicit system attribution
    const endpoint = join(directory, "daemon.json");
    const temporary = `${endpoint}.${process.pid}`;
    await runWorkload(workload, {
        services: { daemon: { hostname: "127.0.0.1", port: 0 } },
        ready: async (addresses) => {
            await writeFile(
                temporary,
                JSON.stringify({
                    port: Number(addresses.daemon.port),
                    token: authentication.token,
                }),
                { mode: 0o600, flag: "wx" },
            );
            cleanup.defer(() => rm(temporary, { force: true }));
            await rename(temporary, endpoint);
            cleanup.defer(() => rm(endpoint));
            logger.emit({
                body: "Daemon started",
                attributes: {
                    "process.pid": process.pid,
                    "server.port": Number(addresses.daemon.port),
                },
            });
        },
    });
    logger.emit({ body: "Daemon stopped" });
}
