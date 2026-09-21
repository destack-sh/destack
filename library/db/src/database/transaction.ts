import { DatabaseError } from "../error/index.ts";

/** The lifetime of a transaction callback and its retained queries. */
export class TransactionState {
    /** Whether the callback can still submit queries. */
    #isActive = true;
    /** Queries submitted before the callback finished. */
    readonly #pending = new Set<Promise<unknown>>();
    /** Failed queries that require rollback. */
    readonly #failures: unknown[] = [];
    /** Cancellation requested by the caller. */
    readonly signal?: AbortSignal;

    /** Retain cancellation without interrupting a shared physical connection. */
    constructor(signal?: AbortSignal) {
        this.signal = signal;
    }

    /** Run a callback and settle its queries before returning to the native transaction. */
    async execute<Value>(operation: () => Promise<Value>): Promise<Value> {
        this.assertActive();

        // drain work even when application code exits with an error
        let result: Value;
        try {
            result = await operation();
        } catch (error) {
            try {
                await this.finish();
            } catch (cleanup) {
                if (cleanup !== error) {
                    throw new AggregateError(
                        [error, cleanup],
                        "transaction callback and queries failed",
                    );
                }
            }
            throw error;
        }

        await this.finish();

        return result;
    }

    /** Reject queries after the callback has finished. */
    assertActive(): void {
        this.signal?.throwIfAborted();
        if (!this.#isActive) {
            throw new DatabaseError("TRANSACTION_CLOSED", "The transaction has finished.");
        }
    }

    /** Track submitted work until it settles, including work the callback did not await. */
    run<Value>(operation: () => PromiseLike<Value>, rollback = true): Promise<Value> {
        this.assertActive();

        // start the query before the callback can close its submission period
        let result: Promise<Value>;
        try {
            result = Promise.resolve(operation());
        } catch (error) {
            result = Promise.reject(error);
        }
        const settled = result.then(
            () => {
                this.#pending.delete(settled);
            },
            (error) => {
                this.#pending.delete(settled);
                if (rollback) {
                    this.#failures.push(error);
                }
            },
        );
        this.#pending.add(settled);

        return result;
    }

    /** Drain submitted queries before allowing commit or rollback. */
    async finish(): Promise<void> {
        this.close();
        await Promise.all(this.#pending);

        // report failed statements before checking cancellation at commit
        if (this.#failures.length === 1) {
            throw this.#failures[0];
        }
        if (this.#failures.length > 1) {
            throw new AggregateError(this.#failures, "transaction queries failed");
        }
        this.signal?.throwIfAborted();
    }

    /** End access when the callback completes or fails. */
    close(): void {
        this.#isActive = false;
    }
}

/** Transaction isolation shared by application queries. */
export interface TransactionOptions {
    /** The minimum isolation; SQLite transactions provide serializable isolation. */
    readonly isolationLevel?: "read committed" | "repeatable read" | "serializable";
    /** Cancel submissions and roll back after already submitted work settles. */
    readonly signal?: AbortSignal;
}
