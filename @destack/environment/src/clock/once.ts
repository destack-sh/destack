/** A value read on first use and read again after a failure. */
export class Once<Value> {
    /** The current read, if any. */
    #read: Promise<Value> | undefined;

    /** Return the value, reading it on first use and after a failure. */
    get(read: () => Promise<Value>): Promise<Value> {
        this.#read ??= read().catch((error: unknown) => {
            this.#read = undefined;
            throw error;
        });

        return this.#read;
    }
}
