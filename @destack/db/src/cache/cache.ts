import type { TelemetryScope } from "@destack/telemetry";

/** How a cache answered a take: a stored value, a read already running, or a new value. */
const CACHE_RESULTS = ["hit", "shared", "miss"] as const;

/** How a cache answered a take. */
export type CacheResult = (typeof CACHE_RESULTS)[number];

/** Values by key up to a total weight, the least recently used dropped first, counted by name. */
export class Cache<Value> {
    /** The most weight the stored values have together. */
    readonly #capacity: number;
    /** Weigh a value and its key, one by default. */
    readonly #weigh: (key: string, value: Value) => number;
    /** Release a value dropped past the capacity, such as a prepared statement. */
    readonly #evicted: ((value: Value) => void) | undefined;
    /** The values with their weights, least recently used first. */
    readonly #values = new Map<string, { readonly value: Value; readonly weight: number }>();
    /** The weight of the stored values. */
    #weight = 0;
    /** The takes, by how the cache answered them. */
    readonly #requests;
    /** The values dropped to stay within the capacity. */
    readonly #evictions;
    /** The attributes of each answer, built once. */
    readonly #answers: Readonly<Record<CacheResult, Readonly<Record<string, string>>>>;
    /** The cache's name attribute. */
    readonly #named: Readonly<Record<string, string>>;

    /** Store values up to a total weight, counting takes and evictions by the cache's name. */
    constructor(options: {
        /** The cache's name on its signals. */
        readonly name: string;
        /** The most weight the stored values have together. */
        readonly capacity: number;
        /** The telemetry counting its takes and evictions. */
        readonly scope: TelemetryScope;
        /** Weigh a value and its key, one by default. */
        readonly weigh?: (key: string, value: Value) => number;
        /** Release a value dropped past the capacity. */
        readonly evicted?: (value: Value) => void;
    }) {
        // take the bounds, and declare the takes and evictions by the cache's name
        const { name, scope } = options;
        this.#capacity = options.capacity;
        this.#weigh = options.weigh ?? (() => 1);
        this.#evicted = options.evicted;
        this.#named = { "destack.cache.name": name };
        this.#answers = {
            hit: { "destack.cache.name": name, "destack.cache.result": "hit" },
            shared: { "destack.cache.name": name, "destack.cache.result": "shared" },
            miss: { "destack.cache.name": name, "destack.cache.result": "miss" },
        };
        this.#requests = scope.metric.counter("destack.cache.requests", {
            unit: "{request}",
            attributes: { "destack.cache.name": [name], "destack.cache.result": CACHE_RESULTS },
        });
        this.#evictions = scope.metric.counter("destack.cache.evictions", {
            unit: "{entry}",
            attributes: { "destack.cache.name": [name] },
        });
    }

    /** Read a key's value, marking it used last. */
    get(key: string): Value | undefined {
        // move a kept value to the recent end
        const kept = this.#values.get(key);
        if (kept === undefined) {
            return undefined;
        }
        this.#values.delete(key);
        this.#values.set(key, kept);

        return kept.value;
    }

    /** Take a key's value, creating and storing it the first time, counted as a hit or a miss. */
    take(key: string, create: () => Value): Value {
        // reuse the key's value, else create and keep it
        const kept = this.get(key);
        this.count(kept === undefined ? "miss" : "hit");
        if (kept !== undefined) {
            return kept;
        }
        const created = create();
        this.set(key, created);

        return created;
    }

    /** Store a key's value last, dropping the least recently used past the capacity. */
    set(key: string, value: Value): void {
        // replace the key's value
        this.delete(key);
        const weight = this.#weigh(key, value);
        this.#values.set(key, { value, weight });
        this.#weight += weight;

        // drop the oldest past the capacity
        for (const [oldest, kept] of this.#values) {
            if (this.#weight <= this.#capacity) {
                break;
            }
            this.#values.delete(oldest);
            this.#weight -= kept.weight;
            this.#evicted?.(kept.value);
            this.#evictions.add(1, this.#named);
        }
    }

    /** Drop a key's value without releasing it. */
    delete(key: string): void {
        const kept = this.#values.get(key);
        if (kept !== undefined) {
            this.#values.delete(key);
            this.#weight -= kept.weight;
        }
    }

    /** List the stored values, least recently used first. */
    values(): Value[] {
        return [...this.#values.values()].map((kept) => kept.value);
    }

    /** Drop every value without releasing it. */
    clear(): void {
        this.#values.clear();
        this.#weight = 0;
    }

    /** Count a take a wrapping cache answered itself, such as a shared read. */
    count(result: CacheResult): void {
        this.#requests.add(1, this.#answers[result]);
    }
}
