import { toJsonSchema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import { Observable } from "../observable/index.ts";
import type {
    Operation,
    OperationContext,
    OperationError,
    OperationDefinition,
} from "./operation.ts";
import { reportError } from "../server/error.ts";
import { MAX_TIMER_DELAY } from "../timer/index.ts";

/** The operations of a service, kept in memory until expiry. */
export class OperationStore<Result, Progress> implements AsyncDisposable {
    /** The operation definition. */
    readonly definition: OperationDefinition<Result, Progress>;
    /** The store limits. */
    readonly #options: OperationStoreOptions;
    /** The operations by identifier. */
    readonly #entries = new Map<string, Entry<Result, Progress>>();
    /** Whether new operations are refused. */
    #isClosed = false;

    /** Create the store. */
    constructor(definition: OperationDefinition<Result, Progress>, options: OperationStoreOptions) {
        // reject nonpositive limits
        for (const value of Object.values(options)) {
            if (!Number.isSafeInteger(value) || value <= 0) {
                throw new RangeError("operation limits must be positive safe integers");
            }
        }

        // require capacity for every runner
        if (options.capacity < options.concurrency) {
            throw new RangeError("operation capacity must cover concurrency");
        }

        // reject delays beyond the 32 bit timer
        if (options.timeout > MAX_TIMER_DELAY) {
            throw new RangeError("operation timeout exceeds the runtime timer limit");
        }

        // keep the schemas and limits
        toJsonSchema(definition.result);
        toJsonSchema(definition.progress);
        this.definition = definition;
        this.#options = { ...options };
    }

    /** Start an operation for an owner. */
    start(
        owner: string,
        progress: Progress,
        run: (context: OperationContext<Progress>) => Promise<Result>,
    ): Operation<Result, Progress> {
        // expire old records
        this.#expire();
        if (!owner) {
            throw new ServiceError("UNAUTHORIZED", { message: "an operation needs an owner" });
        }

        // refuse work after close
        if (this.#isClosed) {
            throw new ServiceError("UNAVAILABLE", { message: "the operation store is closed" });
        }

        // count active runners
        let active = 0;
        for (const entry of this.#entries.values()) {
            if (entry.watch.value.state === "running") {
                active++;
            }
        }

        // refuse work beyond the limits
        if (active >= this.#options.concurrency || this.#entries.size >= this.#options.capacity) {
            throw new ServiceError("RATE_LIMITED", {
                message: "the operation store is at its limit",
            });
        }

        // build the initial state
        const now = Date.now();
        const operation: Operation<Result, Progress> = {
            id: crypto.randomUUID(),
            createdAt: now,
            updatedAt: now,
            cancellationRequested: false,
            state: "running",
            progress: structuredClone(this.definition.progress.parse(progress)),
        };

        // register the entry and run it
        const entry: Entry<Result, Progress> = {
            owner,
            controller: new AbortController(),
            watch: new Observable(operation),
            done: Promise.resolve(),
        };
        this.#entries.set(operation.id, entry);
        entry.done = Promise.resolve().then(() => this.#run(entry, run));

        return structuredClone(operation);
    }

    /** Read an operation of an owner. */
    get(owner: string, id: string): Operation<Result, Progress> {
        return structuredClone(this.#entry(owner, id).watch.value);
    }

    /** List the operations of an owner. */
    list(owner: string): Operation<Result, Progress>[] {
        // expire old records
        this.#expire();

        return [...this.#entries.values()]
            .filter((entry) => entry.owner === owner)
            .map((entry) => structuredClone(entry.watch.value));
    }

    /** Yield an operation's state and changes. */
    async *watch(owner: string, id: string, signal?: AbortSignal) {
        // yield the entry's changes
        const entry = this.#entry(owner, id);
        for await (const value of entry.watch.watch(signal)) {
            yield structuredClone(value);
            if (value.state !== "running") {
                return;
            }
        }
    }

    /** Request cancellation. */
    cancel(owner: string, id: string): Operation<Result, Progress> {
        // cancel the entry
        const entry = this.#entry(owner, id);
        this.#cancel(entry);

        return structuredClone(entry.watch.value);
    }

    /** Delete a completed operation. */
    delete(owner: string, id: string): void {
        // refuse to delete a running operation
        const entry = this.#entry(owner, id);
        if (entry.watch.value.state === "running") {
            throw new ServiceError("CONFLICT", { message: `operation ${id} is running` });
        }

        // release the subscribers
        entry.watch.close();
        this.#entries.delete(id);
    }

    /** Cancel the runners, wait for them, and release the records. */
    async close(): Promise<void> {
        // refuse new work and cancel the runners
        this.#isClosed = true;
        const entries = [...this.#entries.values()];
        for (const entry of entries) {
            this.#cancel(entry);
        }

        // wait for the runners
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

    /** Find an operation of an owner. */
    #entry(owner: string, id: string): Entry<Result, Progress> {
        // match the owner
        this.#expire();
        const entry = this.#entries.get(id);
        if (!entry || entry.owner !== owner) {
            throw new ServiceError("NOT_FOUND", { message: `no operation ${id}` });
        }

        return entry;
    }

    /** Cancel a running operation. */
    #cancel(entry: Entry<Result, Progress>): void {
        // cancel once
        const value = entry.watch.value;
        if (value.state === "running" && !value.cancellationRequested) {
            entry.watch.set({ ...value, updatedAt: Date.now(), cancellationRequested: true });
            entry.controller.abort(new DOMException("operation cancelled", "AbortError"));
        }
    }

    /** Remove expired completed records. */
    #expire(): void {
        // remove records past retention
        const cutoff = Date.now() - this.#options.retention;
        for (const [id, entry] of this.#entries) {
            const value = entry.watch.value;
            if (value.state !== "running" && value.completedAt <= cutoff) {
                entry.watch.close();
                this.#entries.delete(id);
            }
        }
    }

    /** Run an operation and publish its outcome. */
    async #run(
        entry: Entry<Result, Progress>,
        run: (context: OperationContext<Progress>) => Promise<Result>,
    ): Promise<void> {
        // abort at the deadline
        const timer = setTimeout(() => {
            const value = entry.watch.value;
            entry.watch.set({ ...value, updatedAt: Date.now(), cancellationRequested: true });
            entry.controller.abort(new DOMException("operation deadline exceeded", "TimeoutError"));
        }, this.#options.timeout);

        // run with progress reports
        try {
            entry.controller.signal.throwIfAborted();
            const result = await run({
                signal: entry.controller.signal,
                report: (progress) => {
                    // reject reports after completion
                    if (entry.watch.value.state !== "running") {
                        throw new ServiceError("PRECONDITION_FAILED", {
                            message: "the operation has completed",
                        });
                    }

                    // publish a copy of the progress
                    entry.watch.set({
                        ...entry.watch.value,
                        updatedAt: Date.now(),
                        progress: structuredClone(this.definition.progress.parse(progress)),
                    });
                },
            });

            // publish the validated result
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
            // detect cancellation and deadline
            const signal = entry.controller.signal;
            const now = Date.now();
            const cancelled =
                signal.aborted && error === signal.reason && signal.reason?.name !== "TimeoutError";
            const timedOut =
                signal.aborted && error === signal.reason && signal.reason?.name === "TimeoutError";
            const value = { ...entry.watch.value, updatedAt: now, completedAt: now };

            // keep the cancellation
            if (cancelled) {
                entry.watch.set({ ...value, state: "cancelled" });
            }
            // keep the deadline failure
            else if (timedOut) {
                const failure = {
                    code: "DEADLINE_EXCEEDED",
                    message: "operation deadline exceeded",
                };
                entry.watch.set({ ...value, state: "failed", error: failure });
            }
            // keep other failures
            else {
                const failure = describeFailure(error);
                entry.watch.set({ ...value, state: "failed", error: failure });
            }
        } finally {
            clearTimeout(timer);
        }
    }
}

/** The limits of an operation store. */
export interface OperationStoreOptions {
    /** The most simultaneous runners. */
    concurrency: number;
    /** The most retained operations. */
    capacity: number;
    /** The retention of completed operations, in milliseconds. */
    retention: number;
    /** The deadline, in milliseconds. */
    timeout: number;
}

/** A retained operation. */
interface Entry<Result, Progress> {
    /** The owner. */
    owner: string;
    /** The cancellation controller. */
    controller: AbortController;
    /** The state and subscribers. */
    watch: Observable<Operation<Result, Progress>>;
    /** The runner's completion. */
    done: Promise<void>;
}

/** Describe a failure as a public operation error. */
function describeFailure(error: unknown): OperationError {
    const failure = reportError(error);

    return { code: failure.code, message: failure.message };
}
