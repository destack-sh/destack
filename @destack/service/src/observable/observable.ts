/** A current value with change notifications. */
export class Observable<Value> {
    /** Resubscribe after each completed subscription until cancelled. */
    static async *observe<Value>(
        open: (signal: AbortSignal) => Promise<AsyncIterable<Value>>,
        signal: AbortSignal,
    ): AsyncGenerator<Value> {
        while (!signal.aborted) {
            // yield each value of the subscription
            let isReceived = false;
            for await (const value of await open(signal)) {
                isReceived = true;
                yield value;
            }

            // reject a subscription that ends without its snapshot
            if (!isReceived && !signal.aborted) {
                throw new Error("snapshot subscription closed without a value");
            }
        }
    }

    /** The latest value. */
    #value: Value;
    /** The current change number. */
    #revision = 0;
    /** Whether the producer has stopped. */
    #isClosed = false;
    /** The waiting subscribers. */
    readonly #subscribers = new Set<() => void>();

    /** Create the observable. */
    constructor(value: Value) {
        this.#value = value;
    }

    /** The latest value. */
    get value(): Value {
        return this.#value;
    }

    /** Set the value. */
    set(value: Value): void {
        // reject values after close
        if (this.#isClosed) {
            throw new Error("observable is closed");
        }

        // wake the subscribers
        this.#value = value;
        this.#revision++;
        for (const wake of this.#subscribers) {
            wake();
        }
    }

    /** Close the observable. */
    close(): void {
        // release the subscribers
        this.#isClosed = true;
        for (const wake of this.#subscribers) {
            wake();
        }
    }

    /** Yield the value, then the latest value after each change. */
    async *watch(signal?: AbortSignal): AsyncGenerator<Value> {
        // start before the first revision
        let revision = -1;
        while (!this.#isClosed && !signal?.aborted) {
            // subscribe before reading
            const pending = Promise.withResolvers<void>();
            const wake = () => pending.resolve();
            this.#subscribers.add(wake);
            signal?.addEventListener("abort", wake, { once: true });

            // yield a changed value or wait
            try {
                if (revision !== this.#revision) {
                    revision = this.#revision;
                    yield this.value;
                } else {
                    await pending.promise;
                }
            } finally {
                this.#subscribers.delete(wake);
                signal?.removeEventListener("abort", wake);
            }
        }
    }
}
