import { DatabaseError, wraps } from "../error/index.ts";

/** The lifetime of a transaction callback and its queries. */
export class TransactionState {
    /** Whether the callback can still submit queries. */
    #isActive = true;
    /** The queries submitted before the callback finished. */
    readonly #pending = new Set<Promise<unknown>>();
    /** The failed queries. */
    readonly #failures: unknown[] = [];
    /** The caller's cancellation signal. */
    readonly signal?: AbortSignal;

    /** Create the state. */
    constructor(signal?: AbortSignal) {
        this.signal = signal;
    }

    /** Run a callback and settle its queries. */
    async execute<Value>(operation: () => Promise<Value>): Promise<Value> {
        // reject work after the callback finishes
        this.assertActive();

        // drain work even after an error
        let result: Value;
        try {
            result = await operation();
        } catch (error) {
            // settle the queries and keep unreported failures
            this.close();
            await Promise.all(this.#pending);
            const unreported = this.#failures.filter((failure) => !wraps(error, failure));
            if (unreported.length > 0) {
                throw new AggregateError(
                    [error, ...unreported],
                    "transaction callback and queries failed",
                );
            }
            throw error;
        }

        // settle pending queries
        await this.finish();

        return result;
    }

    /** Reject queries after the callback finished. */
    assertActive(): void {
        this.signal?.throwIfAborted();
        if (!this.#isActive) {
            throw new DatabaseError("TRANSACTION_CLOSED", "the transaction has finished");
        }
    }

    /** Track submitted work until it settles. */
    run<Value>(
        operation: () => PromiseLike<Value>,
        failure: "rollback" | "report" = "rollback",
    ): Promise<Value> {
        // reject work after the callback finishes
        this.assertActive();

        // start the query
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
                if (failure === "rollback") {
                    this.#failures.push(error);
                }
            },
        );
        this.#pending.add(settled);

        return result;
    }

    /** Drain submitted queries. */
    async finish(): Promise<void> {
        // close submissions and wait
        this.close();
        await Promise.all(this.#pending);

        // report failures before checking cancellation
        if (this.#failures.length === 1) {
            throw this.#failures[0];
        }
        if (this.#failures.length > 1) {
            throw new AggregateError(this.#failures, "transaction queries failed");
        }
        this.signal?.throwIfAborted();
    }

    /** End submissions. */
    close(): void {
        this.#isActive = false;
    }
}

/** The options of a transaction. */
export interface TransactionOptions {
    /** The minimum isolation; SQLite is serializable. */
    readonly isolationLevel?: "read committed" | "repeatable read" | "serializable";
    /** Cancel submissions and roll back. */
    readonly signal?: AbortSignal;
    /** Whether the transaction only reads. */
    readonly isReadOnly?: boolean;
    /** Check foreign keys per statement or at commit. */
    readonly constraints?: "immediate" | "deferred";
}
