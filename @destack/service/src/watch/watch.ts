/** A current value with bounded, coalesced change notifications. */
export class Watch<Value> {
    /** Reopen completed snapshot subscriptions with fresh authentication until cancellation. */
    static async *observe<Value>(
        open: (signal: AbortSignal) => Promise<AsyncIterable<Value>>,
        signal: AbortSignal,
    ): AsyncGenerator<Value> {
        while (!signal.aborted) {
            // each successful subscription must publish its current snapshot
            let isReceived = false;
            for await (const value of await open(signal)) {
                isReceived = true;
                yield value;
            }
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
    #closed = false;
    /** Subscribers waiting for a changed value. */
    readonly #subscribers = new Set<() => void>();

    /** Start with an initial value. */
    constructor(value: Value) {
        this.#value = value;
    }

    /** Read the latest published value. */
    get value(): Value {
        return this.#value;
    }

    /** Replace the value and wake waiting subscribers. */
    set(value: Value): void {
        if (this.#closed) {
            throw new Error("watch is closed");
        }

        // publish the latest value and wake current subscribers
        this.#value = value;
        this.#revision++;
        for (const wake of this.#subscribers) {
            wake();
        }
    }

    /** Stop waiting subscribers. */
    close(): void {
        // release all waiting subscribers
        this.#closed = true;
        for (const wake of this.#subscribers) {
            wake();
        }
    }

    /** Yield the current value, then the latest value after each observed change. */
    async *watch(signal?: AbortSignal): AsyncGenerator<Value> {
        // deliver the current value before waiting for changes
        let revision = -1;
        while (!this.#closed && !signal?.aborted) {
            // subscribe before reading to retain changes made while the consumer is suspended
            const pending = Promise.withResolvers<void>();
            const wake = () => pending.resolve();
            this.#subscribers.add(wake);
            signal?.addEventListener("abort", wake, { once: true });

            // emit a changed value or wait without accumulating previous values
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
