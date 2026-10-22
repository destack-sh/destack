import { LogPosition } from "./position.ts";

/** Reads of one kind, each stored at its read's log position, least recently used dropped. */
export class ReadCache<Value> {
    /** The most weight the stored values have together. */
    readonly #capacity: number;
    /** The most weight one stored value has. */
    readonly #maximum: number;
    /** Weigh a value, such as by its rows. */
    readonly #weigh: (value: Value) => number;
    /** The values read with their positions and weights, least recently used first. */
    readonly #values = new Map<string, Entry<Value> & { readonly weight: number }>();
    /** The reads running, by key. */
    readonly #reading = new Map<string, Entry<Promise<Value>>>();
    /** The weight of the stored values. */
    #weight = 0;

    /** Store values up to a total weight, each at most a maximum weight. */
    constructor(capacity: number, maximum: number, weigh: (value: Value) => number) {
        this.#capacity = capacity;
        this.#maximum = maximum;
        this.#weigh = weigh;
    }

    /** Read a value at a position or later, from memory or once through a shared read. */
    get(key: string, read: () => Promise<Value>, position: LogPosition): Promise<Value> {
        // answer a value stored at the position or later, marking it used last
        const kept = this.#values.get(key);
        if (kept !== undefined && !LogPosition.isAfter(position, kept.position)) {
            this.#values.delete(key);
            this.#values.set(key, kept);

            return Promise.resolve(kept.value);
        }

        // answer a running read at the position or later
        const running = this.#reading.get(key);
        if (running !== undefined && !LogPosition.isAfter(position, running.position)) {
            return running.value;
        }

        // read anew, storing the value unless a newer one arrives meanwhile
        const reading: Promise<Value> = read().then(
            (value) => {
                // finish the running read, and store its value unless a newer one arrived
                const newest = this.#values.get(key);
                if (this.#reading.get(key)?.value === reading) {
                    this.#reading.delete(key);
                }
                if (newest === undefined || !LogPosition.isAfter(newest.position, position)) {
                    this.#set(key, value, position);
                }

                return value;
            },
            (error: unknown) => {
                if (this.#reading.get(key)?.value === reading) {
                    this.#reading.delete(key);
                }
                throw error;
            },
        );
        this.#reading.set(key, { value: reading, position });

        return reading;
    }

    /** Store a read value, dropping the least recently used ones past the capacity. */
    #set(key: string, value: Value, position: LogPosition): void {
        // replace the key's value, storing none heavier than the maximum
        this.#forget(key);
        const weight = this.#weigh(value);
        if (weight > this.#maximum) {
            return;
        }

        // store the value last, dropping the oldest past the capacity
        this.#values.set(key, { value, position, weight });
        this.#weight += weight;
        for (const oldest of this.#values.keys()) {
            if (this.#weight <= this.#capacity) {
                break;
            }
            this.#forget(oldest);
        }
    }

    /** Drop a stored value and its weight. */
    #forget(key: string): void {
        const kept = this.#values.get(key);
        if (kept !== undefined) {
            this.#values.delete(key);
            this.#weight -= kept.weight;
        }
    }
}

/** A value with the log position it was read at. */
interface Entry<Value> {
    /** The value. */
    readonly value: Value;
    /** The log position it was read at. */
    readonly position: LogPosition;
}
