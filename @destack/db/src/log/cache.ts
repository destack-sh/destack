/** Reads of one kind, kept until the log invalidates them or they are least recently used. */
export class ReadCache<Value> {
    /** The most values kept. */
    readonly #capacity: number;
    /** The values read, least recently used first. */
    readonly #values = new Map<string, Value>();
    /** The reads running, by key. */
    readonly #reading = new Map<string, Promise<Value>>();

    /** Keep at most some values. */
    constructor(capacity: number) {
        this.#capacity = capacity;
    }

    /** Read a value from memory, or once through a read that lookups of the same key share. */
    get(key: string, read: () => Promise<Value>): Promise<Value> {
        // answer a kept value, marking it used last
        if (this.#values.has(key)) {
            const value = this.#values.get(key) as Value;
            this.#values.delete(key);
            this.#values.set(key, value);

            return Promise.resolve(value);
        }

        // answer a running read
        const running = this.#reading.get(key);
        if (running !== undefined) {
            return running;
        }

        // keep the read's value unless the key was forgotten meanwhile
        const reading: Promise<Value> = read().then(
            (value) => {
                if (this.#reading.get(key) === reading) {
                    this.#reading.delete(key);
                    this.#keep(key, value);
                }

                return value;
            },
            (error: unknown) => {
                if (this.#reading.get(key) === reading) {
                    this.#reading.delete(key);
                }
                throw error;
            },
        );
        this.#reading.set(key, reading);

        return reading;
    }

    /** Forget the value under a key. */
    forget(key: string): void {
        this.#values.delete(key);
        this.#reading.delete(key);
    }

    /** Forget every value. */
    clear(): void {
        this.#values.clear();
        this.#reading.clear();
    }

    /** Keep a read value, dropping the least recently used one past the capacity. */
    #keep(key: string, value: Value): void {
        this.#values.set(key, value);
        if (this.#values.size > this.#capacity) {
            this.#values.delete(this.#values.keys().next().value!);
        }
    }
}
