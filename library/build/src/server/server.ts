import {
    implement,
    implementOperation,
    OperationStore,
    ServiceHandler,
    type HandlerOptions,
    type OperationRequest,
} from "@destack/service/server";
import type { OperationStoreOptions } from "@destack/service/server";
import {
    buildPackage,
    PackageBuilder,
    type BuildOptions,
    type PackageBuild,
} from "../build/index.ts";
import { PackageInspection } from "@destack/package/inspect";
import { BuildError } from "../error/index.ts";
import { ServiceError } from "@destack/service/error";
import type { InspectOptions } from "../inspect/inspection.ts";
import {
    buildService,
    BuildOperation,
    type BuildRequest,
    type BuildResult,
    type BuildProgress,
    type InspectRequest,
} from "../service/index.ts";
import { implementPreview, PreviewStore, type PreviewHost, type PreviewLimits } from "./preview.ts";

/** Build and preview procedures with bounded, caller-scoped state. */
export class BuildServer implements AsyncDisposable {
    /** HTTP handler with shared authorization, audit, probes, and telemetry. */
    readonly handler: ServiceHandler<OperationRequest>;
    /** Retained operation records. */
    readonly builds: OperationStore<BuildResult, BuildProgress>;
    /** Retained previews and their live servers. */
    readonly #previews: PreviewStore;
    /** Active inspection requests retained until their compiler exits. */
    readonly #inspections = new Map<AbortController, Promise<void>>();
    /** Host access and limits shared by request handlers. */
    readonly #options: BuildServerOptions;
    /** Whether shutdown has begun. */
    #closed = false;

    /** Connect host services to the declared API. */
    constructor(options: BuildServerOptions) {
        // retain host access and bounded request state
        this.#options = options;
        this.builds = new OperationStore(BuildOperation, options.limits.build);
        this.#previews = new PreviewStore(options.previews, options.limits.preview);
        const service = implement(buildService).$context<OperationRequest>();

        // retain host source access until compilation and storage have settled
        const start = service.build.start.handler(({ input, context }) =>
            this.builds.start(context.owner, { phase: "preparing" }, async ({ signal, report }) => {
                await using source = await options.builds.open(context.owner, input, signal);
                report({ phase: "building" });

                // preserve operation cancellation across the compiler process
                let build: PackageBuild;
                try {
                    build = await buildPackage({ ...source.options, signal });
                } catch (error) {
                    if (
                        signal.aborted &&
                        error instanceof BuildError &&
                        error.cause === signal.reason
                    ) {
                        throw signal.reason;
                    }
                    throw error;
                }

                // publish the complete package before completing the operation
                report({ phase: "storing" });
                signal.throwIfAborted();
                const download = await options.builds.store(context.owner, build, signal);

                return { source: input.source, manifest: build.manifest, download };
            }),
        );

        // enforce access through the common handler before dispatching caller-scoped operations
        this.handler = new ServiceHandler(
            service.router({
                inspect: service.inspect.handler(async ({ input, context, signal }) => {
                    return await this.#inspect(context.owner, input, signal);
                }),
                build: { ...implementOperation(this.builds, "/builds"), start },
                preview: implementPreview(this.#previews),
            }),
            options,
        );
    }

    /** Stop accepting work and await build and preview cleanup. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.#closed = true;
        this.handler.health.set("draining");
        for (const controller of this.#inspections.keys()) {
            controller.abort();
        }

        // await all retained work before reporting shutdown failures
        const results = await Promise.allSettled([
            this.builds.close(),
            this.#previews[Symbol.asyncDispose](),
            ...this.#inspections.values(),
        ]);
        this.handler.health.set("stopped");
        const failures = results
            .filter((result) => result.status === "rejected")
            .map((result) => result.reason);
        if (failures.length) {
            throw new AggregateError(failures, "build service shutdown failed");
        }
    }

    /** Retain inspection cancellation and source lifetime until the compiler exits. */
    async #inspect(
        owner: string,
        request: InspectRequest,
        signal?: AbortSignal,
    ): Promise<ReturnType<typeof PackageInspection.parse>> {
        // bound compiler processes and reject requests during shutdown
        if (this.#closed) {
            throw new ServiceError("UNAVAILABLE");
        }
        if (this.#inspections.size >= this.#options.limits.build.concurrency) {
            throw new ServiceError("RATE_LIMITED");
        }
        const controller = new AbortController();
        const completed = Promise.withResolvers<void>();
        this.#inspections.set(controller, completed.promise);
        const cancellation = signal
            ? AbortSignal.any([signal, controller.signal])
            : controller.signal;

        // release source only after the isolated compiler has stopped
        try {
            await using source = await this.#options.builds.inspect(owner, request, cancellation);
            cancellation.throwIfAborted();
            await using compiler = await PackageBuilder.start(source.directory);
            const inspection = {
                target: source.target,
                runtime: source.runtime,
                configuration: source.configuration,
            };

            return PackageInspection.parse(await compiler.inspect(inspection, cancellation));
        } finally {
            this.#inspections.delete(controller);
            completed.resolve();
        }
    }
}

/** Host-retained immutable checkout and resolved build inputs. */
export interface BuildSource extends AsyncDisposable {
    /** Exact source, dependencies, and selected output settings. */
    options: Omit<BuildOptions, "signal">;
}

/** Host services for source authorization and completed package storage. */
export interface BuildHost {
    /** Authorize the source and retain its immutable checkout through compilation. */
    open(owner: string, request: BuildRequest, signal: AbortSignal): Promise<BuildSource>;
    /** Authorize source inspection and select its compiler settings. */
    inspect(
        owner: string,
        request: InspectRequest,
        signal?: AbortSignal,
    ): Promise<InspectOptions & AsyncDisposable>;
    /** Store all build files and return an authorized complete-archive download URL. */
    store(owner: string, build: PackageBuild, signal: AbortSignal): Promise<string>;
}

/** Host access, retention, and required shared service enforcement. */
export interface BuildServerOptions extends HandlerOptions<OperationRequest> {
    /** Immutable source access and result storage. */
    builds: BuildHost;
    /** Editable source access and preview routing. */
    previews: PreviewHost;
    /** Build and preview bounds; inspection uses the build concurrency as a separate limit. */
    limits: { build: OperationStoreOptions; preview: PreviewLimits };
}
