import { ServiceError } from "@destack/service/error";
import { reportError } from "@destack/service/server";
import { Watch } from "@destack/service/watch";
import { LocalServer, type LocalServerOptions } from "../local/server.ts";
import { Preview, type PreviewRequest } from "../service/preview.ts";

/** Preview lifecycle and subscriptions scoped to authenticated callers. */
export class PreviewPool implements AsyncDisposable {
    /** Preview process host. */
    readonly host: PreviewHost;
    /** Preview resource limits. */
    readonly options: PreviewLimits;

    /** Retained previews indexed by identifier. */
    readonly #entries = new Map<string, PreviewEntry>();
    /** Whether the service refuses new previews. */
    #closed = false;

    /** Configure host access and bounded preview retention. */
    constructor(host: PreviewHost, options: PreviewLimits) {
        this.host = host;
        this.options = options;

        // require finite bounds before accepting any work
        if (
            Object.values(options).some((value) => !Number.isSafeInteger(value) || value <= 0) ||
            options.capacity < options.concurrency
        ) {
            throw new ServiceError("PRECONDITION_FAILED", { message: "invalid preview limits" });
        }
    }

    /** Register a preview before asynchronously opening its source and server. */
    start(owner: string, request: PreviewRequest): Preview {
        // expire completed records and enforce admission limits
        this.#expire();
        if (!owner) {
            throw new ServiceError("UNAUTHORIZED");
        }
        if (this.#closed) {
            throw new ServiceError("UNAVAILABLE");
        }
        const active = [...this.#entries.values()].filter(
            (entry) =>
                entry.server ||
                entry.source ||
                !["stopped", "failed"].includes(entry.watch.value.state),
        );
        if (
            active.length >= this.options.concurrency ||
            this.#entries.size >= this.options.capacity
        ) {
            throw new ServiceError("RATE_LIMITED");
        }

        // publish starting state before running host code
        const now = Date.now();
        const value: Preview = {
            id: crypto.randomUUID(),
            source: request.source,
            application: request.application,
            createdAt: now,
            updatedAt: now,
            state: "starting",
        };
        const entry: PreviewEntry = {
            owner,
            watch: new Watch(value),
            controller: new AbortController(),
            ready: Promise.resolve(),
            stop: undefined,
        };
        this.#entries.set(value.id, entry);
        entry.ready = this.#start(entry, structuredClone(request));

        return structuredClone(value);
    }

    /** Read a preview visible to the authenticated caller. */
    get(owner: string, id: string): Preview {
        return structuredClone(this.#entry(owner, id).watch.value);
    }

    /** List retained previews visible to the authenticated caller. */
    list(owner: string): Preview[] {
        this.#expire();

        return [...this.#entries.values()]
            .filter((entry) => entry.owner === owner)
            .map((entry) => structuredClone(entry.watch.value));
    }

    /** Stream current state until the preview stops or the subscriber disconnects. */
    async *watch(owner: string, id: string, signal?: AbortSignal): AsyncGenerator<Preview> {
        const entry = this.#entry(owner, id);

        // end subscriptions after their terminal state has been delivered
        for await (const value of entry.watch.watch(signal)) {
            yield structuredClone(value);
            if (value.state === "stopped" || value.state === "failed") {
                return;
            }
        }
    }

    /** Stop serving and release the source before acknowledging completion. */
    async stop(owner: string, id: string): Promise<Preview> {
        const entry = this.#entry(owner, id);
        const stopping = (entry.stop ??= this.#stop(entry));
        try {
            await stopping;
        } finally {
            if (entry.stop === stopping) {
                entry.stop = undefined;
            }
        }

        return structuredClone(entry.watch.value);
    }

    /** Refuse new previews and release every active server and source. */
    async [Symbol.asyncDispose](): Promise<void> {
        // stop every active preview before releasing retained subscriptions
        this.#closed = true;
        const results = await Promise.allSettled(
            [...this.#entries.values()].map((entry) =>
                this.stop(entry.owner, entry.watch.value.id),
            ),
        );

        // close subscribers even when a host cleanup fails
        for (const entry of this.#entries.values()) {
            if (!entry.server && !entry.source) {
                entry.watch.close();
            }
        }
        const failures = results
            .filter((result) => result.status === "rejected")
            .map((result) => result.reason);
        if (failures.length) {
            throw new AggregateError(failures, "preview shutdown failed");
        }
        this.#entries.clear();
    }

    /** Start the selected application and publish its host-authorized URL. */
    async #start(entry: PreviewEntry, request: PreviewRequest): Promise<void> {
        try {
            // retain source before opening the listener
            entry.source = await this.host.open(entry.owner, request, entry.controller.signal);
            entry.controller.signal.throwIfAborted();
            entry.server = await LocalServer.start(entry.source.options);
            entry.controller.signal.throwIfAborted();

            // publish the URL only after host routing is ready
            const url = await entry.source.expose(entry.server);
            entry.controller.signal.throwIfAborted();
            const value = entry.watch.value;
            entry.watch.set(
                Preview.parse({
                    id: value.id,
                    source: value.source,
                    application: value.application,
                    createdAt: value.createdAt,
                    updatedAt: Date.now(),
                    state: "running",
                    url,
                }),
            );
        } catch (error) {
            // stop owns cleanup when cancellation races with startup
            if (entry.controller.signal.aborted) {
                if (error !== entry.controller.signal.reason) {
                    reportError(error);
                }
                return;
            }
            let failure = error;
            try {
                await this.#release(entry);
            } catch (cleanup) {
                failure = new AggregateError(
                    [error, cleanup],
                    "preview startup and cleanup failed",
                );
            }
            this.#fail(entry, failure);
        }
    }

    /** Serialize stop with startup and publish the terminal state after cleanup. */
    async #stop(entry: PreviewEntry): Promise<void> {
        // acknowledge repeated stops without reopening released resources
        const previous = entry.watch.value;
        if (
            (previous.state === "stopped" || previous.state === "failed") &&
            !entry.server &&
            !entry.source
        ) {
            return;
        }

        // cancel startup and wait for it before releasing retained resources
        entry.watch.set({
            id: previous.id,
            source: previous.source,
            application: previous.application,
            createdAt: previous.createdAt,
            updatedAt: Date.now(),
            state: "stopping",
        });
        entry.controller.abort();
        await entry.ready;

        // report completion after server and source cleanup
        try {
            await this.#release(entry);
            const now = Date.now();
            entry.watch.set({
                id: previous.id,
                source: previous.source,
                application: previous.application,
                createdAt: previous.createdAt,
                updatedAt: now,
                state: "stopped",
                stoppedAt: now,
            });
        } catch (error) {
            this.#fail(entry, error);
            throw error;
        }
    }

    /** Close the server before releasing its source and routing. */
    async #release(entry: PreviewEntry): Promise<void> {
        // close network listeners before releasing source and routing
        await entry.server?.close();
        entry.server = undefined;

        // retain failed handles so stop and shutdown can retry cleanup
        await entry.source?.[Symbol.asyncDispose]();
        entry.source = undefined;
    }

    /** Record startup or cleanup failure without exposing internal diagnostics. */
    #fail(entry: PreviewEntry, error: unknown): void {
        const failure = reportError(error);
        const value = entry.watch.value;
        const now = Date.now();
        entry.watch.set({
            id: value.id,
            source: value.source,
            application: value.application,
            createdAt: value.createdAt,
            updatedAt: now,
            stoppedAt: now,
            state: "failed",
            error: { code: failure.code, message: failure.message },
        });
    }

    /** Resolve a preview without revealing another caller's records. */
    #entry(owner: string, id: string): PreviewEntry {
        this.#expire();
        const entry = this.#entries.get(id);
        if (!entry || entry.owner !== owner) {
            throw new ServiceError("NOT_FOUND");
        }

        return entry;
    }

    /** Remove expired terminal records while retaining active previews. */
    #expire(): void {
        const cutoff = Date.now() - this.options.retention;
        for (const [id, entry] of this.#entries) {
            const value = entry.watch.value;
            if (
                (value.state === "stopped" || value.state === "failed") &&
                !entry.server &&
                !entry.source &&
                value.stoppedAt <= cutoff
            ) {
                entry.watch.close();
                this.#entries.delete(id);
            }
        }
    }
}

/** Resources retained through one preview's lifetime. */
interface PreviewEntry {
    /** Authenticated scope selected by the host. */
    owner: string;
    /** Current state and coalesced subscriptions. */
    watch: Watch<Preview>;
    /** Startup cancellation requested by stop or shutdown. */
    controller: AbortController;
    /** Startup completion, including failure cleanup. */
    ready: Promise<void>;
    /** Shared stop completion for concurrent callers. */
    stop: Promise<void> | undefined;
    /** Source retained until the server closes. */
    source?: PreviewSource;
    /** Running Vite server. */
    server?: LocalServer;
}

/** Authorized source and host-selected preview settings. */
export interface PreviewSource extends AsyncDisposable {
    /** The checkout and application selected by the host. */
    options: LocalServerOptions;
    /** Expose the running server through the host's authorized routing. */
    expose(server: LocalServer): Promise<string>;
}

/** Resolve editable source access without accepting filesystem paths from clients. */
export interface PreviewHost {
    /** Authorize the source, select application settings, and retain its checkout. */
    open(owner: string, request: PreviewRequest, signal: AbortSignal): Promise<PreviewSource>;
}

/** Bounds for active and retained previews. */
export interface PreviewLimits {
    /** Maximum previews starting, running, or stopping. */
    concurrency: number;
    /** Maximum records retained, including active previews. */
    capacity: number;
    /** Terminal record retention in milliseconds. */
    retention: number;
}
