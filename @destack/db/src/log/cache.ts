import { nth } from "@destack/schema";
import type { TelemetryScope } from "@destack/telemetry";
import { Cache } from "../cache/cache.ts";

/** Reads of one kind, each kept with its tables' commit versions, least recently used dropped. */
export class ReadCache<Value> {
    /** The most weight one stored value has. */
    readonly #maximum: number;
    /** Weigh a value and its key, such as by its rows. */
    readonly #weigh: (key: string, value: Value) => number;
    /** The values read with their versions, by key. */
    readonly #values: Cache<Entry<Value>>;
    /** The reads running, by key. */
    readonly #reading = new Map<string, Entry<Promise<Value>>>();

    /** Store values up to a total weight, each at most a maximum weight, counting reads by name. */
    constructor(
        capacity: number,
        maximum: number,
        weigh: (key: string, value: Value) => number,
        name: string,
        scope: TelemetryScope,
    ) {
        this.#maximum = maximum;
        this.#values = new Cache({
            name,
            capacity,
            scope,
            weigh: (key, entry) => weigh(key, entry.value),
        });
        this.#weigh = weigh;
    }

    /** Read a value at its tables' versions, from memory or once through a shared read. */
    get(key: string, versions: readonly number[], read: () => Promise<Value>): Promise<Value> {
        // answer a value stored at the same versions, marking it used last
        const kept = this.#values.get(key);
        if (kept !== undefined && isSame(kept.versions, versions)) {
            this.#values.count("hit");

            return Promise.resolve(kept.value);
        }

        // answer a read running at the same versions
        const running = this.#reading.get(key);
        if (running !== undefined && isSame(running.versions, versions)) {
            this.#values.count("shared");

            return running.value;
        }

        // read anew, storing the value at the versions read before it unless a newer one arrived
        this.#values.count("miss");
        const reading: Promise<Value> = read().then(
            (value) => {
                // finish the running read
                if (this.#reading.get(key)?.value === reading) {
                    this.#reading.delete(key);
                }

                // store the value unless a newer one arrived meanwhile
                const newest = this.#values.get(key);
                if (newest === undefined || !isNewer(newest.versions, versions)) {
                    this.#set(key, value, versions);
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
        this.#reading.set(key, { value: reading, versions });

        return reading;
    }

    /** Store a read value, unless heavier than the maximum. */
    #set(key: string, value: Value, versions: readonly number[]): void {
        if (this.#weigh(key, value) > this.#maximum) {
            this.#values.delete(key);

            return;
        }
        this.#values.set(key, { value, versions });
    }
}

/** A value with its tables' commit versions when it was read. */
interface Entry<Value> {
    /** The value. */
    readonly value: Value;
    /** The commit version of each table it read, in the read's table order. */
    readonly versions: readonly number[];
}

/** Decide whether a version list moved past another: no table older, one newer. */
function isNewer(left: readonly number[], right: readonly number[]): boolean {
    return (
        left.every((version, index) => version >= nth(right, index)) &&
        left.some((version, index) => version > nth(right, index))
    );
}

/** Decide whether two version lists are equal. */
function isSame(left: readonly number[], right: readonly number[]): boolean {
    return left.length === right.length && left.every((version, index) => version === right[index]);
}
