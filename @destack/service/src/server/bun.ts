import { serve, type Server } from "bun";
import type { Provider, Resource } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { ServiceError } from "../error/index.ts";
import type { Package } from "@destack/package";
import type { Identifier } from "@destack/schema";
import type { AccessRelay } from "@destack/access";
import { telemetry } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/host";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { Caller } from "../authentication/index.ts";
import { ServiceMount } from "../service/mount.ts";
import {
    type AuditHistory,
    type Workload,
    WorkloadInstance,
    type WorkloadReady,
    WorkloadRenewal,
    WorkloadStart,
} from "../workload/index.ts";

/** How long a stopping runner drains its requests: below the host's fifteen-second stop timeout. */
const DRAIN_MILLISECONDS = 10_000;

/** The runner's log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** A request handler served on one network address. */
export interface Endpoint {
    /** The network interface. */
    readonly hostname: string;
    /** The port, or zero for an ephemeral one. */
    readonly port: number;
    /** Handle one request. */
    fetch(request: Request): Response | Promise<Response>;
}

/** The endpoints and lifecycle of one process. */
export interface ProcessOptions {
    /** The endpoints to listen on. */
    readonly endpoints: readonly Endpoint[];
    /** The signal aborted on shutdown. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Drain requests and release the runtime. */
    close(): Promise<void>;
    /** Publish the listener addresses, in endpoint order. */
    ready?(addresses: readonly URL[]): Promise<void>;
}

/** Serve each endpoint until shutdown, then drain and close. */
export async function serveProcess(options: ProcessOptions): Promise<void> {
    // shut down on process signals
    const listeners: Server<undefined>[] = [];
    const stopped = Promise.withResolvers<void>();
    const stop = () => stopped.resolve();
    const shutdown = () => options.shutdown();
    options.signal.addEventListener("abort", stop, { once: true });
    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
    try {
        // open every listener
        for (const endpoint of options.endpoints) {
            listeners.push(
                serve({
                    hostname: endpoint.hostname,
                    port: endpoint.port,
                    fetch: (request) => endpoint.fetch(request),
                }),
            );
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

/** What a runner holds of the package it runs: its identity, workloads, resources and providers. */
export interface RunnerPackage {
    /** The package, whose mount serves its service and whose name its telemetry carries. */
    readonly package: Package;
    /** The workloads by name. */
    readonly workloads: Readonly<Record<string, Workload>>;
    /** The package's resource declarations by name. */
    readonly resources: Readonly<Record<string, Resource<unknown>>>;
    /** Open the provider holding a resource at a reference, by provider code. */
    readonly providers: Readonly<Record<string, (reference: URL) => Provider>>;
    /** Connect to an audit service with the installation's current credential. */
    history(url: string, credential: () => string): AuditHistory;
    /** Connect to the relay of an installation's space access with its current credential. */
    access(
        url: string,
        installation: {
            readonly spaceId: Identifier<"space">;
            readonly installationId: Identifier<"installation">;
        },
        credential: () => string,
    ): AccessRelay;
}

/** Start the workload a host's first input line names and serve its service until shutdown. */
export async function runWorkload(
    runner: RunnerPackage,
    lines: AsyncIterator<string>,
    ready: (ready: WorkloadReady) => Promise<void>,
): Promise<void> {
    // read the start and select the workload
    await using cleanup = new AsyncDisposableStack();
    const first = await lines.next();
    if (first.done === true) {
        throw new ServiceError("BAD_REQUEST", { message: "the host sent no start" });
    }
    const start = WorkloadStart.parse(JSON.parse(first.value));
    const workload = runner.workloads[start.workload];
    if (workload === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `unknown workload: ${start.workload}` });
    }

    // export the package's telemetry to its space's monitor, reporting failed exports on standard error
    let credential = start.credential;
    const exporter = OtlpExporter.http(
        start.monitor,
        () => `Bearer ${credential}`,
        (error) => process.stderr.write(`telemetry export failed: ${error.message}\n`),
    );
    const exporting = await startTelemetry(
        exporter.options(runner.package, {
            attributes: { "service.instance.id": start.instance },
            ratio: start.sampling,
        }),
    );
    cleanup.defer(() => exporting.shutdown());

    // connect each bound resource through its provider
    const resources = new ResourceContext();
    for (const [name, binding] of Object.entries(start.bindings)) {
        const declaration = runner.resources[name];
        const provider = runner.providers[binding.provider];
        if (declaration === undefined || provider === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `no resource ${name} with provider ${binding.provider}`,
            });
        }
        const { resource, kind, reference, spec } = binding;
        const record = { id: resource, scope: start.scope, kind, spec, reference };
        const client = await provider(new URL(reference)).connect(record, declaration);
        resources.bind(declaration, client as never);
    }

    // trust the callers the host forwards with its secret, and refuse every other request
    const audience = runner.package.id;
    const instance = await WorkloadInstance.start(workload, {
        resources,
        history: runner.history(start.audit, () => credential),
        access: runner.access(
            start.space,
            { spaceId: start.scope, installationId: start.installation },
            () => credential,
        ),
        service: () => ({
            audience,
            scope: start.scope,
            instance: start.instance,
            drainTimeout: DRAIN_MILLISECONDS,
            authenticate: async (request) => {
                if (request.headers.get("authorization") !== `Bearer ${start.secret}`) {
                    throw new ServiceError("UNAUTHORIZED", { message: "invalid host secret" });
                }
                const caller = Caller.forwarded(request);
                caller?.requireCurrent(audience, Date.now(), start.scope);

                return caller;
            },
            authorizeHost: async ({ access }) => {
                if (access.authentication === "host") {
                    throw new ServiceError("FORBIDDEN", {
                        message: "a workload serves no host procedures",
                    });
                }
            },
        }),
    });

    // follow the host's credential renewals, stopping the workload on a malformed renewal
    void (async () => {
        for (let line = await lines.next(); line.done !== true; line = await lines.next()) {
            credential = WorkloadRenewal.parse(JSON.parse(line.value)).credential;
        }
    })().catch((error: unknown) => {
        log.error("workload.renewal.failed", { message: String(error) });
        instance.shutdown();
    });
    log.info("workload.started", { workload: start.workload, instance: start.instance });

    // serve the package's one service below its mount on a loopback port
    const [service, ...others] = instance.services;
    if (service === undefined || others.length > 0) {
        await instance.close();
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `a runner serves one service per package, found ${instance.services.length}`,
        });
    }
    await serveProcess({
        endpoints: [
            {
                hostname: "127.0.0.1",
                port: 0,
                fetch: (request) => {
                    // require the package's mount and pass the request below it
                    const routed = ServiceMount.route(request);
                    if (routed?.packageId !== audience) {
                        return new Response(null, { status: 404 });
                    }

                    return instance.fetch(service, routed.request);
                },
            },
        ],
        signal: instance.signal,
        shutdown: () => instance.shutdown(),
        close: () => instance.close(),
        ready: (addresses) => ready({ port: Number(addresses[0]!.port) }),
    });
}
