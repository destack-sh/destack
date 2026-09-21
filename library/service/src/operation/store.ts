import { toJsonSchema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import { Watch } from "../watch/index.ts";
import type {
    Operation,
    OperationContext,
    OperationError,
    OperationDefinition,
} from "./operation.ts";
import { reportError } from "../server/error.ts";

/** Authorized, bounded operation state retained until expiry or service shutdown. */
export class OperationStore<Result, Progress> implements AsyncDisposable {
    /** The definition shared with the HTTP procedures. */
    readonly definition: OperationDefinition<Result, Progress>;
    /** Configured resource limits. */
    readonly #options: OperationStoreOptions;
    /** Operations indexed by identifier. */
    readonly #entries = new Map<string, Entry<Result, Progress>>();
    /** Whether new operations are refused. */
    #closed = false;

    /** Configure schemas and retention for this operation type. */
    constructor(definition: OperationDefinition<Result, Progress>, options: OperationStoreOptions) {
        // reject limits that cannot bound work or retain active operations
        for (const value of Object.values(options)) {
            if (!Number.isSafeInteger(value) || value <= 0) {
                throw new RangeError("Operation limits must be positive safe integers.");
            }
        }

        // reserve enough retained records for all active runners
        if (options.capacity < options.concurrency) {
            throw new RangeError("Operation capacity must cover concurrency.");
        }

        // reject delays that overflow the runtime's signed 32 bit timer
        if (options.timeout > 2 ** 31 - 1) {
            throw new RangeError("Operation timeout exceeds the runtime timer limit.");
        }

        // retain portable schemas and store limits
        toJsonSchema(definition.result);
        toJsonSchema(definition.progress);
        this.definition = definition;
        this.#options = { ...options };
    }

    /** Start work under an authenticated scope that callers cannot choose themselves. */
    start(
        owner: string,
        progress: Progress,
        run: (context: OperationContext<Progress>) => Promise<Result>,
    ): Operation<Result, Progress> {
        // admit work only within configured limits
        this.#expire();
        if (!owner) {
            throw new ServiceError("UNAUTHORIZED");
        }
        if (this.#closed) {
            throw new ServiceError("UNAVAILABLE");
        }

        // count active runners within the bounded store
        let active = 0;
        for (const entry of this.#entries.values()) {
            if (entry.watch.value.state === "running") {
                active++;
            }
        }

        // refuse work that would exceed concurrency or retained record limits
        if (active >= this.#options.concurrency || this.#entries.size >= this.#options.capacity) {
            throw new ServiceError("RATE_LIMITED");
        }

        // retain the initial state before scheduling the runner
        const now = Date.now();
        const operation: Operation<Result, Progress> = {
            id: crypto.randomUUID(),
            createdAt: now,
            updatedAt: now,
            cancellationRequested: false,
            state: "running",
            progress: structuredClone(this.definition.progress.parse(progress)),
        };

        // register cancellation and subscriptions before scheduling work
        const entry: Entry<Result, Progress> = {
            owner,
            controller: new AbortController(),
            watch: new Watch(operation),
            done: Promise.resolve(),
        };
        this.#entries.set(operation.id, entry);
        entry.done = Promise.resolve().then(() => this.#run(entry, run));

        return structuredClone(operation);
    }

    /** Read an operation visible to the authenticated scope. */
    get(owner: string, id: string): Operation<Result, Progress> {
        return structuredClone(this.#entry(owner, id).watch.value);
    }

    /** List retained operations visible to the authenticated scope. */
    list(owner: string): Operation<Result, Progress>[] {
        // omit expired records before selecting the caller's operations
        this.#expire();

        return [...this.#entries.values()]
            .filter((entry) => entry.owner === owner)
            .map((entry) => structuredClone(entry.watch.value));
    }

    /** Yield current state and coalesced updates until completion or subscriber cancellation. */
    async *watch(owner: string, id: string, signal?: AbortSignal) {
        // authorize once and retain the subscription through its terminal outcome
        const entry = this.#entry(owner, id);
        for await (const value of entry.watch.watch(signal)) {
            yield structuredClone(value);
            if (value.state !== "running") {
                return;
            }
        }
    }

    /** Request cancellation without claiming the runner has stopped. */
    cancel(owner: string, id: string): Operation<Result, Progress> {
        // publish cancellation before notifying the active runner
        const entry = this.#entry(owner, id);
        const value = entry.watch.value;
        if (value.state === "running" && !value.cancellationRequested) {
            entry.watch.set({ ...value, updatedAt: Date.now(), cancellationRequested: true });
            entry.controller.abort(new DOMException("Operation cancelled.", "AbortError"));
        }

        return structuredClone(entry.watch.value);
    }

    /** Delete a completed record without deleting its external output. */
    delete(owner: string, id: string): void {
        // retain records while their runners can still publish progress
        const entry = this.#entry(owner, id);
        if (entry.watch.value.state === "running") {
            throw new ServiceError("CONFLICT");
        }

        // release subscribers and the retained result
        entry.watch.close();
        this.#entries.delete(id);
    }

    /** Cancel active runners and wait for their cleanup before releasing retained records. */
    async close(): Promise<void> {
        // refuse new work and cancel active runners
        this.#closed = true;
        const entries = [...this.#entries.values()];
        for (const entry of entries) {
            const value = entry.watch.value;
            if (value.state === "running" && !value.cancellationRequested) {
                entry.watch.set({ ...value, updatedAt: Date.now(), cancellationRequested: true });
                entry.controller.abort(new DOMException("Operation cancelled.", "AbortError"));
            }
        }

        // wait for cleanup before releasing records and subscribers
        await Promise.all(entries.map((entry) => entry.done));
        for (const entry of entries) {
            entry.watch.close();
        }
        this.#entries.clear();
    }

    /** Close the store. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Resolve an authorized operation without exposing other owners' records. */
    #entry(owner: string, id: string): Entry<Result, Progress> {
        // resolve only records visible to the authenticated caller
        this.#expire();
        const entry = this.#entries.get(id);
        if (!entry || entry.owner !== owner) {
            throw new ServiceError("NOT_FOUND");
        }

        return entry;
    }

    /** Remove expired terminal records while keeping active work addressable. */
    #expire(): void {
        // release completed records after the configured retention period
        const cutoff = Date.now() - this.#options.retention;
        for (const [id, entry] of this.#entries) {
            const value = entry.watch.value;
            if (value.state !== "running" && value.completedAt <= cutoff) {
                entry.watch.close();
                this.#entries.delete(id);
            }
        }
    }

    /** Run work once and publish a terminal outcome after the runner settles. */
    async #run(
        entry: Entry<Result, Progress>,
        run: (context: OperationContext<Progress>) => Promise<Result>,
    ): Promise<void> {
        // notify the runner at its deadline without detaching unfinished work
        const timer = setTimeout(() => {
            const value = entry.watch.value;
            entry.watch.set({ ...value, updatedAt: Date.now(), cancellationRequested: true });
            entry.controller.abort(
                new DOMException("Operation deadline exceeded.", "TimeoutError"),
            );
        }, this.#options.timeout);

        // run with validated progress updates and cooperative cancellation
        try {
            entry.controller.signal.throwIfAborted();
            const result = await run({
                signal: entry.controller.signal,
                report: (progress) => {
                    // reject reports after the runner has completed
                    if (entry.watch.value.state !== "running") {
                        throw new ServiceError("PRECONDITION_FAILED");
                    }

                    // publish an independent snapshot of the latest progress
                    entry.watch.set({
                        ...entry.watch.value,
                        updatedAt: Date.now(),
                        progress: structuredClone(this.definition.progress.parse(progress)),
                    });
                },
            });

            // publish success only after the runner and result validation finish
            const value = structuredClone(this.definition.result.parse(result));
            const now = Date.now();
            entry.watch.set({
                ...entry.watch.value,
                state: "succeeded",
                completedAt: now,
                updatedAt: now,
                result: value,
            });
        } catch (error) {
            // acknowledge cancellation only when the runner fails with its abort reason
            const signal = entry.controller.signal;
            const now = Date.now();
            const cancelled =
                signal.aborted && error === signal.reason && signal.reason?.name !== "TimeoutError";
            const timedOut =
                signal.aborted && error === signal.reason && signal.reason?.name === "TimeoutError";
            const value = { ...entry.watch.value, updatedAt: now, completedAt: now };

            // retain acknowledged cancellation
            if (cancelled) {
                entry.watch.set({ ...value, state: "cancelled" });
            }
            // retain the deadline failure
            else if (timedOut) {
                const failure = {
                    code: "DEADLINE_EXCEEDED",
                    message: "Operation deadline exceeded.",
                };
                entry.watch.set({ ...value, state: "failed", error: failure });
            }
            // report other failures through the service error policy
            else {
                const failure = describeFailure(error);
                entry.watch.set({ ...value, state: "failed", error: failure });
            }
        } finally {
            clearTimeout(timer);
        }
    }
}

/** Explicit limits for an in-memory operation store. */
export interface OperationStoreOptions {
    /** Maximum simultaneous runners. */
    concurrency: number;
    /** Maximum retained operations, including active runners. */
    capacity: number;
    /** Completed operation retention in milliseconds. */
    retention: number;
    /** Deadline in milliseconds, enforced through the runner's cancellation signal. */
    timeout: number;
}

/** Runtime controls for one retained operation. */
interface Entry<Result, Progress> {
    /** Authenticated authorization scope. */
    owner: string;
    /** Cooperative cancellation controller. */
    controller: AbortController;
    /** Current state and waiting subscribers. */
    watch: Watch<Operation<Result, Progress>>;
    /** Runner completion, including cleanup. */
    done: Promise<void>;
}

/** Retain public service failures and record unexpected exceptions privately. */
function describeFailure(error: unknown): OperationError {
    const failure = reportError(error);

    return { code: failure.code, message: failure.message };
}
