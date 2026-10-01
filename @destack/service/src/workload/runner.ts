import { type ReplicaSource } from "@destack/sync";
import type { ResourceDeclaration } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { telemetry } from "@destack/telemetry";
import type { Telemetry, TelemetryOptions } from "@destack/telemetry/sdk";
import { OtlpExporter } from "@destack/telemetry/otlp";
import type {} from "@destack/package/import-meta";
import { Authentication } from "../authentication/index.ts";
import type { Service } from "../declare/service.ts";
import { ServiceError } from "../error/index.ts";
import { Egress } from "../service/egress.ts";
import { ServiceMount } from "../service/mount.ts";
import { WorkloadInstance } from "./instance.ts";
import { WEBHOOK_PATH, type WorkloadStart } from "./start.ts";
import type { PackageId } from "@destack/package";
import { WebhookOn, Trigger, type RunClient } from "../trigger/index.ts";
import { refusal } from "../server/error.ts";
import type { Alarm } from "../control/index.ts";
import type { AuditHistory, Workload } from "./workload.ts";
import { CallKey } from "../request/index.ts";

/** How long a stopping runner drains its requests: below the host's fifteen-second stop timeout. */
const DRAIN_MILLISECONDS = 10_000;

/** The address of the audit service of a workload's space. */
const AUDIT_ADDRESS = "@destack/audit";

/** The address of the monitor service of a workload's space, receiving its telemetry over OTLP. */
const MONITOR_ADDRESS = "@destack/monitor";

/** The address of the space service of a workload's holder, relaying the space's access. */
const SPACE_ADDRESS = "@destack/space";

/** The runner's log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** What a runner runs: one workload, its package's resources, and the clients of its space. */
export interface RunnerOptions {
    /** The workload with the package mount for its service and the name for its telemetry. */
    readonly workload: Workload;
    /** The package's resource declarations by name. */
    readonly resources: Readonly<Record<string, ResourceDeclaration<unknown>>>;
    /** Connect to an audit service through the host's egress with the runner's secret. */
    history(url: string, secret: string): AuditHistory;
    /** Connect to the source of an installation's copies, its space's cell, through the host's egress with the runner's secret. */
    replicas(url: string, secret: string): ReplicaSource;
    /** Connect to the cell recording the installation's runs, through the host's egress with the runner's secret. */
    runs(url: string, start: WorkloadStart): RunClient;
    // TODO #Incomplete: pass the Durable Object's storage as the alarm from a workerd entry, awaiting `idle()` in its alarm handler
    /** Keep a wake-up for the workload's earliest due controller key, such as the Durable Object's alarm running it. */
    readonly alarm?: Alarm;
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
    readonly #packageId: PackageId;
    /** The installation's resources, which webhooks read their secrets from. */
    readonly #resources: ResourceContext;
    /** The cell recording the installation's runs. */
    readonly #runs: RunClient;
    /** The telemetry exporting to the space's monitor. */
    readonly #telemetry: Telemetry;
    /** Hold a started instance and its telemetry. */
    private constructor(
        start: WorkloadStart,
        instance: WorkloadInstance,
        service: Service,
        running: Telemetry,
        packageId: PackageId,
        resources: ResourceContext,
        runs: RunClient,
    ) {
        // hold the instance and what it runs with
        this.start = start;
        this.instance = instance;
        this.#service = service;
        this.#packageId = packageId;
        this.#telemetry = running;
        this.#resources = resources;
        this.#runs = runs;
    }

    /** Start a workload as a host's start asks, with the runtime's telemetry and failure reporting. */
    static async start(
        runner: RunnerOptions,
        start: WorkloadStart,
        startTelemetry: (options: TelemetryOptions) => Promise<Telemetry>,
        report: (error: unknown) => void,
    ): Promise<WorkloadRunner> {
        // export the package's telemetry to its space's monitor through the host's egress
        const bearer = () => `Bearer ${start.secret}`;
        const monitor = Egress.url(start.egress, MONITOR_ADDRESS);
        const exporter = OtlpExporter.http(monitor, bearer, report);
        const running = await startTelemetry(
            exporter.options(runner.workload.package, {
                attributes: { "service.instance.id": start.instance },
                ratio: start.sampling,
            }),
        );

        // start the instance on the bound resources, stopping telemetry when it fails
        try {
            const resources = await WorkloadRunner.#connect(runner, start);
            const callKey = CallKey.import(Uint8Array.fromHex(start.callKey));
            const space = Egress.url(start.egress, SPACE_ADDRESS);
            const runs = runner.runs(space, start);
            const instance = await WorkloadInstance.start(runner.workload, {
                resources,
                history: runner.history(Egress.url(start.egress, AUDIT_ADDRESS), start.secret),
                replicas: {
                    scope: start.scope,
                    source: runner.replicas(space, start.secret),
                },
                runs,
                callKey: () => callKey,
                report,
                ...(runner.alarm === undefined ? {} : { alarm: runner.alarm }),
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

            // hold the instance
            log.info("workload.started", {
                workload: runner.workload.name,
                instance: start.instance,
            });

            return new WorkloadRunner(
                start,
                instance,
                service,
                running,
                runner.workload.package.id,
                resources,
                runs,
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

    /** Serve a forwarded request below the package's mount and refuse any other. */
    fetch(request: Request): Promise<Response> {
        // refuse a request without the host's secret before anything serves it
        if (request.headers.get("authorization") !== `Bearer ${this.start.secret}`) {
            return Promise.resolve(
                refusal(new ServiceError("UNAUTHORIZED", { message: "invalid host secret" })),
            );
        }

        // verify and record a webhook's delivery
        const { pathname } = new URL(request.url);
        if (pathname.startsWith(`${WEBHOOK_PATH}/`) && request.method === "POST") {
            return this.#receive(request, pathname.slice(WEBHOOK_PATH.length + 1));
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

    /** Settle once no controller key is due now or reconciling, as an alarm's handler waits before its instance may be evicted. */
    idle(): Promise<void> {
        return this.instance.idle();
    }

    /** Verify a request to one of the package's webhook triggers, and record the call its delivery runs once. */
    async #receive(request: Request, below: string): Promise<Response> {
        // find the webhook trigger the first segment names
        const slash = below.indexOf("/");
        const name = slash === -1 ? below : below.slice(0, slash);
        const found = this.instance.trigger(this.#packageId, name);
        const trigger = found && Trigger.of(found, "webhook");
        if (trigger === undefined) {
            return refusal(
                new ServiceError("NOT_FOUND", { message: `no webhook trigger ${name}` }),
            );
        }

        // verify the delivery below the trigger's route, and record its call once per route path and delivery
        try {
            const path = slash === -1 ? "/" : below.slice(slash);
            const delivery = await WebhookOn.receive(
                trigger.on.webhook,
                request,
                path,
                Date.now(),
                this.#resources,
            );
            await this.#runs.send({
                call: trigger.call(delivery),
                triggerName: name,
                event: { delivery: { id: delivery.id, path } },
            });

            return new Response(null, { status: 202 });
        }
        // answer a refusal as it is, and report an unexpected failure without its details
        catch (error) {
            return refusal(error);
        }
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

    /** Connect each bound resource through its declaration's connector for the resource's provider. */
    static async #connect(runner: RunnerOptions, start: WorkloadStart): Promise<ResourceContext> {
        const resources = new ResourceContext();
        for (const [name, binding] of Object.entries(start.bindings)) {
            // require the declaration of the binding's kind and its connector for the provider
            const declaration = runner.resources[name];
            const connector = declaration?.connectors[binding.provider];
            if (
                declaration === undefined ||
                declaration.kind !== binding.kind ||
                connector === undefined
            ) {
                throw new ServiceError("NOT_FOUND", {
                    message: `no ${binding.kind} ${name} connecting to provider ${binding.provider}`,
                });
            }

            // connect to the bound resource
            resources.bind(declaration, (await connector.connect(binding, declaration)) as never);
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
                const caller = Authentication.forwarded(request);
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
