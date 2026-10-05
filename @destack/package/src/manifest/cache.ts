import type { Digest } from "@destack/schema";
import type { BuildReader } from "./reader.ts";

/** The builds a cache keeps values of by default: 256 catalogs of some kilobytes each take a few megabytes. */
const CAPACITY = 256;

/** Values read once from each build's declarations, kept by the instance serving them. */
export class BuildCache<Value> {
    /** Read the value of one build. */
    readonly #read: (reader: BuildReader) => Promise<Value>;
    /** The most builds kept, the least recently read dropped first. */
    readonly #capacity: number;
    /** The values read so far, by the digest of each build's graph, least recently read first. */
    readonly #values = new Map<Digest, Promise<Value>>();

    /** Keep the values a read takes from builds, up to a capacity of builds. */
    constructor(read: (reader: BuildReader) => Promise<Value>, capacity: number = CAPACITY) {
        this.#read = read;
        this.#capacity = capacity;
    }

    /** Read a build's value once per graph, reading it again after a failed read. */
    read(reader: BuildReader): Promise<Value> {
        // keep a value read before as the most recent
        const key = reader.manifest.lists.graph.digest;
        const kept = this.#values.get(key);
        if (kept !== undefined) {
            this.#values.delete(key);
            this.#values.set(key, kept);

            return kept;
        }

        // read the value and drop the least recently read beyond the capacity
        const value = this.#read(reader);
        this.#values.set(key, value);
        for (const [oldest] of this.#values) {
            if (this.#values.size <= this.#capacity) {
                break;
            }
            this.#values.delete(oldest);
        }

        // read again after a failed read
        value.catch(() => {
            if (this.#values.get(key) === value) {
                this.#values.delete(key);
            }
        });

        return value;
    }
}
