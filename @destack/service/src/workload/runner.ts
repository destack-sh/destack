import { type ChainRelay } from "@destack/sync";
import type { ProviderContext, Resource } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import type { Identifier } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type { Telemetry, TelemetryOptions } from "@destack/telemetry/sdk";
import { OtlpExporter } from "@destack/telemetry/otlp";
import type {} from "@destack/package/import-meta";
import { Caller } from "../authentication/index.ts";
import type { Service } from "../declare/service.ts";
import { ServiceError } from "../error/index.ts";
import { ServiceMount } from "../service/mount.ts";
import { WorkloadInstance } from "./instance.ts";
import type { WorkloadRenewal, WorkloadStart } from "./start.ts";
import type { AuditHistory, Workload } from "./workload.ts";

/** How long a stopping runner drains its requests: below the host's fifteen-second stop timeout. */
const DRAIN_MILLISECONDS = 10_000;

/** The runner's log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** What a runner runs: one workload, its package's resources, and the clients of its space. */
export interface RunnerOptions {
    /** The workload with the package mount for its service and the name for its telemetry. */
    readonly workload: Workload;
    /** The package's resource declarations by name. */
    readonly resources: Readonly<Record<string, Resource<unknown>>>;
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
    ): ChainRelay;
}

/** A workload instance a host started, serving its package below its mount on any runtime. */
export class WorkloadRunner implements AsyncDisposable {
    /** The host's start message. */
    readonly start: WorkloadStart;
    /** The running instance. */
    readonly instance: WorkloadInstance;
    /** The package's one service. */
    readonly #service: Service;
    /** The running package with the mount that serves the service. */
    readonly #packageId: string;
    /** The telemetry exporting to the space's monitor. */
    readonly #telemetry: Telemetry;
    /** The installation's current credential for the runner's clients. */
    readonly #credential: { current: string };

    /** Hold a started instance, its telemetry and its credential. */
    private constructor(
        start: WorkloadStart,
        instance: WorkloadInstance,
        service: Service,
        running: Telemetry,
        credential: { current: string },
        packageId: string,
    ) {
        // hold the instance and what it runs with
        this.start = start;
        this.instance = instance;
        this.#service = service;
        this.#packageId = packageId;
        this.#telemetry = running;
        this.#credential = credential;
    }

    /** Start a workload as a host's start asks, with the runtime's telemetry and failure reporting. */
    static async start(
        runner: RunnerOptions,
        start: WorkloadStart,
        startTelemetry: (options: TelemetryOptions) => Promise<Telemetry>,
        report: (error: Error) => void,
    ): Promise<WorkloadRunner> {
        // export the package's telemetry to its space's monitor with the current credential
        const credential = { current: start.credential };
        const bearer = () => `Bearer ${credential.current}`;
        const exporter = OtlpExporter.http(start.monitor, bearer, report);
        const running = await startTelemetry(
            exporter.options(runner.workload.package, {
                attributes: { "service.instance.id": start.instance },
                ratio: start.sampling,
            }),
        );

        // start the instance on the bound resources, stopping telemetry when it fails
        try {
            const resources = await WorkloadRunner.#connect(runner, start, {
                credential: () => credential.current,
            });
            const instance = await WorkloadInstance.start(runner.workload, {
                resources,
                history: runner.history(start.audit, () => credential.current),
                access: runner.access(
                    start.space,
                    { spaceId: start.scope, installationId: start.installation },
                    () => credential.current,
                ),
                service: () => WorkloadRunner.#serve(runner, start),
            });

            // require the package's one service
            const [service, ...others] = instance.services;
            if (service === undefined || others.length > 0) {
                await instance.close();
                const found = instance.services.length;
                throw new ServiceError("PRECONDITION_FAILED", {
                    message: `a runner serves one service per package, found ${found}`,
                });
            }

            // hold the instance with the credential its clients read
            log.info("workload.started", {
                workload: runner.workload.name,
                instance: start.instance,
            });

            return new WorkloadRunner(
                start,
                instance,
                service,
                running,
                credential,
                runner.workload.package.id,
            );
        } catch (error) {
            await running.shutdown();
            throw error;
        }
    }

    /** Aborts once the instance shuts down. */
    get signal(): AbortSignal {
        return this.instance.signal;
    }

    /** Take the installation's next credential. */
    renew(renewal: WorkloadRenewal): void {
        this.#credential.current = renewal.credential;
    }

    /** Serve a forwarded request below the package's mount and refuse any other. */
    fetch(request: Request): Promise<Response> {
        // refuse a request without the host's secret before anything serves it
        if (request.headers.get("authorization") !== `Bearer ${this.start.secret}`) {
            const refusal = { code: "UNAUTHORIZED", message: "invalid host secret" };

            return Promise.resolve(Response.json(refusal, { status: 401 }));
        }

        // refuse other paths, and serve the package's mount
        const routed = ServiceMount.route(request);
        if (routed?.packageId !== this.#packageId) {
            return Promise.resolve(new Response(null, { status: 404 }));
        }

        return this.instance.fetch(this.#service, routed.request);
    }

    /** Request shutdown. */
    shutdown(): void {
        this.instance.shutdown();
    }

    /** Drain the instance, then export the telemetry left and stop it. */
    async close(): Promise<void> {
        try {
            await this.instance.close();
        } finally {
            await this.#telemetry.shutdown();
        }
    }

    /** Close the runner at the end of its scope. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Connect each bound resource through its provider. */
    static async #connect(
        runner: RunnerOptions,
        start: WorkloadStart,
        context: ProviderContext,
    ): Promise<ResourceContext> {
        const resources = new ResourceContext();
        for (const [name, binding] of Object.entries(start.bindings)) {
            // require the declaration and the provider of its kind
            const declaration = runner.resources[name];
            const provider = declaration?.providers[binding.provider];
            if (declaration === undefined || provider === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `no resource ${name} with provider ${binding.provider}`,
                });
            }

            // connect through the provider holding the resource
            const { resource, kind, reference, spec } = binding;
            const record = { id: resource, scope: start.scope, kind, spec, reference };
            const client = await (
                await provider(new URL(reference), context)
            ).connect(record, declaration);
            resources.bind(declaration, client as never);
        }

        return resources;
    }

    /** Serve the package's service to the callers the host forwards. */
    static #serve(runner: RunnerOptions, start: WorkloadStart) {
        const audience = runner.workload.package.id;

        return {
            audience,
            scope: start.scope,
            instance: start.instance,
            drainTimeout: DRAIN_MILLISECONDS,
            authenticate: async (request: Request) => {
                // read the caller the host forwards, the secret checked before serving
                const caller = Caller.forwarded(request);
                caller?.requireCurrent(audience, Date.now(), start.scope);

                return caller;
            },
            authorizeHost: async ({
                access,
            }: {
                readonly access: { readonly authentication: string };
            }) => {
                if (access.authentication === "host") {
                    throw new ServiceError("FORBIDDEN", {
                        message: "a workload serves no host procedures",
                    });
                }
            },
        };
    }
}
