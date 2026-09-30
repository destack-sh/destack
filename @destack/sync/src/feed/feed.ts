import { Key, TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { Condition, type Match } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { describeLog, type Change, type LogPosition } from "@destack/db/log";
import { SyncError } from "../error/error.ts";
import type { Row } from "@destack/db";
import type { QueryPage } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { EVERYONE, watchedScopes, type Audience, type Watch } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import { Stream } from "./stream.ts";
import type { Upstream } from "../dataflow/upstream.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type RunCost } from "../dataflow/run.ts";
import { View, type Cache } from "../dataflow/view.ts";

/**
 * The default count of recent changes a feed keeps in memory.
 *
 * At about 1 KB a change, a busy feed holds about 10 MB.
 */
const FEED_CHANGES = 10_000;

/**
 * The default subscriber limit of a feed.
 *
 * A row every subscriber follows costs each about 4 µs to publish, about 4 ms in all.
 */
const FEED_SUBSCRIBERS = 1000;

/**
 * The default count of rows and groups one evaluation knows.
 *
 * Window rows take about 1 KB and arranged entries about 100 B: 100k fit in about 20 MB.
 */
const CAPACITY = 100_000;

/** The recent log sequences whose row reads subscribers share. */
const SHARED_READS = 8;

/**
 * The default wait before a caught-up stream repeats its position, in milliseconds.
 *
 * A copy three beats unconfirmed, 30 seconds, is stale.
 */
const HEARTBEAT_MILLISECONDS = 10_000;

/** One database's log, read once per commit and served to every subscriber. */
export class Feed implements Cache {
    /** The database whose log the feed reads. */
    readonly database: DatabaseConnection;
    /** The logged tables subscribers may read. */
    readonly tables: readonly Table[];
    /** The subscriber limit. */
    readonly #limit: number;
    /** The most rows and groups one evaluation knows. */
    readonly capacity: number;
    /** The count of recent changes kept in memory. */
    readonly #memory: number;
    /** Report each run's cost. */
    readonly observe: ((cost: RunCost) => void) | undefined;
    /** The wait before a caught-up stream repeats its position, in milliseconds. */
    readonly heartbeat: number;
    /** What a copy knows of its source. */
    readonly upstream: Upstream | undefined;
    /** The recent changes in commit order. */
    #changes: Change[] = [];
    /** The sequence before the oldest kept change. */
    #start = Number.POSITIVE_INFINITY;
    /** The sequence the kept changes reach, below zero before reading. */
    #sequence = -1;
    /** The subscribers waiting for the feed to pass a sequence. */
    readonly #waiting = new Set<() => void>();
    /** The subscriber count. */
    #subscribers = 0;
    /** Stop reading after the last subscriber. */
    #reading?: AbortController;
    /** The failure that stopped reading. */
    #failure?: unknown;
    /** The shared evaluations by key. */
    readonly #evaluations = new Map<string, Evaluation>();
    /** Each shared evaluation's key and the streams sharing it. */
    readonly #shares = new Map<Evaluation, { key: string; streams: number }>();
    /** The shared reads by sequence and key. */
    readonly #reads = new Map<number, Map<string, unknown>>();
    /** The compiled row conditions of watches. */
    readonly #watchedRows = new WeakMap<Watch, Match>();

    /** Serve a database's logged tables within the limits. */
    constructor(
        database: DatabaseConnection,
        tables: readonly Table[],
        options: {
            readonly subscribers?: number;
            readonly capacity?: number;
            readonly changes?: number;
            readonly upstream?: Upstream;
            readonly observe?: (cost: RunCost) => void;
            readonly heartbeat?: number;
        } = {},
    ) {
        // keep the logged tables and the limits
        this.database = database;
        this.upstream = options.upstream;
        this.tables = tables
            .flatMap((table) => [
                table,
                ...(table[TABLE].tree === undefined || this.upstream !== undefined
                    ? []
                    : [table[TABLE].tree.ancestors]),
            ])
            .filter((table) => describeLog(table) !== undefined);
        this.#limit = options.subscribers ?? FEED_SUBSCRIBERS;
        this.capacity = options.capacity ?? CAPACITY;
        this.#memory = options.changes ?? FEED_CHANGES;
        this.observe = options.observe;
        this.heartbeat = options.heartbeat ?? HEARTBEAT_MILLISECONDS;
    }

    /** Describe the feed. */
    inspect(): FeedInspection {
        return {
            sequence: this.#sequence,
            changes: this.#changes.length,
            subscribers: this.#subscribers,
            limit: this.#limit,
            capacity: this.capacity,
            evaluations: [...this.#shares].map(([evaluation, share]) => ({
                streams: share.streams,
                dataflow: evaluation.inspect(),
            })),
        };
    }

    /** The sequence the feed read the log up to. */
    get sequence(): number {
        return this.#sequence;
    }

    /**
     * Follow queries from a position for an audience.
     *
     * A subscriber without a current position first receives a paged snapshot.
     * With an interval, pages at the head merge within the interval.
     * Once the drain aborts, the stream ends after its next completed page.
     */
    async *subscribe(
        queries: Readonly<Record<string, Query>>,
        after: LogPosition | undefined,
        signal: AbortSignal,
        options: {
            readonly audience?: Audience;
            readonly previous?: Readonly<Record<string, Query>>;
            readonly every?: number;
            readonly drain?: AbortSignal;
        } = {},
    ): AsyncGenerator<QueryPage> {
        // resolve the queries
        const audience = options.audience ?? EVERYONE;
        const stream = new Stream(this, queries, audience, options.every);
        const previous =
            options.previous === undefined
                ? undefined
                : new Evaluation(this, options.previous, audience);

        // refuse subscribers beyond the limit
        if (this.#subscribers >= this.#limit) {
            throw new SyncError("OVERLOADED", `feed serves ${this.#limit} subscribers already`);
        }
        this.#join();
        try {
            yield* stream.run(after, signal, previous, options.drain);
        } finally {
            this.#leave();
        }
    }

    /** Follow one query's result until the signal aborts. */
    async *watch(
        name: string,
        query: Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly Readonly<Record<string, unknown>>[]> {
        // compile the query
        const dataflow = new Dataflow(
            { [name]: query },
            {
                audience: EVERYONE,
                database: this.database,
                ...(this.upstream === undefined ? {} : { upstream: this.upstream }),
                changesThrough: (watches, after, through) =>
                    this.changesThrough(watches, after, through),
                isMaterialized: true,
            },
        );
        if (this.#subscribers >= this.#limit) {
            throw new SyncError("OVERLOADED", `feed serves ${this.#limit} subscribers already`);
        }
        this.#join();
        try {
            let position: LogPosition | undefined;
            while (!signal.aborted) {
                // hold the query and read its result
                if (position === undefined) {
                    position = await this.database.log.position();
                    await dataflow.fill(new View(this.database, position, this));
                    yield await dataflow.read(name);
                    continue;
                }

                // apply the changes and read the result again
                const read = await this.changes(dataflow.watches(), position.sequence);
                if (read === undefined) {
                    position = undefined;
                } else if (read.changes.length === 0) {
                    position = { epoch: position.epoch, sequence: read.sequence };
                    await this.next(read.sequence, signal);
                } else {
                    const reached = { epoch: position.epoch, sequence: read.sequence };
                    const run = new Run(
                        new View(this.database, reached, this),
                        EVERYONE,
                        undefined,
                        read.changes,
                    );
                    // yield again only when the step changed the held rows or groups
                    position = (await dataflow.step(run)) ? reached : undefined;
                    const isChanged = run.patch.rows.size > 0 || run.patch.results.size > 0;
                    if (position !== undefined && isChanged) {
                        yield await dataflow.read(name);
                    }
                }
            }
        } finally {
            this.#leave();
        }
    }

    /** Read the watched changes after a sequence, absent once compacted. */
    async changes(
        watches: readonly Watch[],
        sequence: number,
    ): Promise<{ readonly changes: readonly Change[]; readonly sequence: number } | undefined> {
        // read kept changes from memory
        if (sequence >= this.#start) {
            return {
                changes: this.#changes
                    .slice(this.#after(sequence))
                    .filter((change) => this.#isWatched(watches, change)),
                sequence: Math.max(sequence, this.#sequence),
            };
        }

        return this.#logged(watches, sequence);
    }

    /** Read the watched changes between two sequences, absent once compacted. */
    async changesThrough(
        watches: readonly Watch[],
        after: number,
        through: number,
    ): Promise<Change[] | undefined> {
        const changes: Change[] = [];
        for (let sequence = after; sequence < through;) {
            // read from memory or a page of the log
            const read =
                sequence >= this.#start && this.#sequence > sequence
                    ? await this.changes(watches, sequence)
                    : await this.#logged(watches, sequence);
            if (read === undefined) {
                return undefined;
            } else if (read.sequence <= sequence) {
                throw new TypeError(`log ends at ${read.sequence} before ${through}`);
            }

            // keep the changes up to the end sequence
            changes.push(...read.changes.filter((change) => change.sequence <= through));
            sequence = read.sequence;
        }

        return changes;
    }

    /** Read a page of watched changes from the log, absent once compacted. */
    async #logged(
        watches: readonly Watch[],
        sequence: number,
    ): Promise<{ readonly changes: readonly Change[]; readonly sequence: number } | undefined> {
        const scopes = watchedScopes(watches);
        try {
            const read = await this.database.log.read({
                tables: [...new Set(watches.map((entry) => entry.table))],
                after: sequence,
                ...(scopes === undefined ? {} : { scopes }),
            });

            return {
                changes: read.changes.filter((change) => this.#isWatched(watches, change)),
                sequence: read.sequence,
            };
        } catch (error) {
            if (error instanceof DatabaseError && error.code === "CHANGES_COMPACTED") {
                return undefined;
            }
            throw error;
        }
    }

    /** Read the images a table's rows had before their first change between two sequences, by key. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>> {
        // share the images per range
        return this.share(after, `images:${table[TABLE].sqlName}:${upto}`, async () => {
            // read from the log unless memory holds the range
            if (upto <= after || after < this.#start || this.#sequence < upto) {
                return this.database.log.images(table, after, upto);
            }
            const images = new Map<string, Row | null>();
            for (let index = this.#after(after); index < this.#changes.length; index += 1) {
                // keep each row's first change in the range
                const change = this.#changes[index]!;
                if (change.sequence > upto) {
                    break;
                }
                const key = change.table === table ? Key.name(table, change.key) : undefined;
                if (key !== undefined && !images.has(key)) {
                    images.set(key, (change.before as Row | undefined) ?? null);
                }
            }

            return images;
        });
    }

    /** Compute a keyed read or encoding once per log sequence, forgetting failed reads. */
    share<Value>(sequence: number, key: string, compute: () => Value): Value {
        // reuse a read of the same sequence
        let reads = this.#reads.get(sequence);
        if (reads?.has(key) === true) {
            return reads.get(key) as Value;
        }

        // start the sequence's reads, dropping the oldest beyond the kept number
        if (reads === undefined) {
            reads = new Map<string, unknown>();
            this.#reads.set(sequence, reads);
            if (this.#reads.size > SHARED_READS) {
                this.#reads.delete(Math.min(...this.#reads.keys()));
            }
        }

        // read, forgetting a failed read
        const value = compute();
        reads.set(key, value);
        if (value instanceof Promise) {
            const shared = reads;
            value.catch(() => {
                if (shared.get(key) === value) {
                    shared.delete(key);
                }
            });
        }

        return value;
    }

    /** Decide whether a read for a sequence is shared. */
    isShared(sequence: number, key: string): boolean {
        return this.#reads.get(sequence)?.has(key) === true;
    }

    /** Share the evaluation of streams deciding alike at a position, returning the one to follow. */
    join(evaluation: Evaluation, position: LogPosition): Evaluation {
        // rekey a shared evaluation whose audience changed
        const key = evaluation.key;
        const share = this.#shares.get(evaluation);
        if (share !== undefined) {
            if (share.key !== key && !this.#evaluations.has(key)) {
                this.#evaluations.delete(share.key);
                this.#evaluations.set(key, evaluation);
                share.key = key;
            }

            return evaluation;
        }

        // join a shared evaluation at the same position, or share this one
        const shared = this.#evaluations.get(key);
        const held = shared?.position;
        if (
            shared !== undefined &&
            held?.epoch === position.epoch &&
            held.sequence === position.sequence
        ) {
            this.#shares.get(shared)!.streams += 1;

            return shared;
        } else if (shared === undefined) {
            this.#evaluations.set(key, evaluation);
            this.#shares.set(evaluation, { key, streams: 1 });
        }

        return evaluation;
    }

    /** Count a stream out of a shared evaluation, forgetting it after the last. */
    leave(evaluation: Evaluation): void {
        // skip an unshared evaluation
        const share = this.#shares.get(evaluation);
        if (share === undefined) {
            return;
        }

        // count the stream out
        share.streams -= 1;
        if (share.streams === 0) {
            this.#evaluations.delete(share.key);
            this.#shares.delete(evaluation);
        }
    }

    /** Decide whether an evaluation is shared. */
    isJoined(evaluation: Evaluation): boolean {
        return this.#shares.has(evaluation);
    }

    /** Wait until the feed passes a sequence, a read fails, or the signal aborts. */
    async next(sequence: number, signal: AbortSignal): Promise<void> {
        await new Promise<void>((resolve) => {
            // wake once past, failed or aborted
            const wake = () => {
                if (this.#sequence > sequence || this.#failure !== undefined || signal.aborted) {
                    this.#waiting.delete(wake);
                    signal.removeEventListener("abort", wake);
                    resolve();
                }
            };
            this.#waiting.add(wake);
            signal.addEventListener("abort", wake, { once: true });
            wake();
        });

        // report the reading failure
        if (this.#failure !== undefined) {
            throw this.#failure;
        }
    }

    /** Find the index of the first kept change after a sequence. */
    #after(sequence: number): number {
        // search the kept changes in sequence order
        let low = 0;
        let high = this.#changes.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (this.#changes[middle]!.sequence <= sequence) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        return low;
    }

    /** Count a subscriber, starting to read with the first. */
    #join(): void {
        this.#subscribers += 1;
        if (this.#reading === undefined) {
            const reading = new AbortController();
            this.#reading = reading;
            this.#failure = undefined;
            this.#follow(reading.signal).catch((error: unknown) => {
                // report the failure to this reading's subscribers
                if (this.#reading === reading) {
                    this.#failure = error;
                    this.#wake();
                }
            });
        }
    }

    /** Count a subscriber out, stopping to read after the last. */
    #leave(): void {
        this.#subscribers -= 1;
        if (this.#subscribers === 0) {
            this.#reading?.abort();
            this.#reading = undefined;
            this.#changes = [];
            this.#start = Number.POSITIVE_INFINITY;
            this.#sequence = -1;
            this.#reads.clear();
            this.#evaluations.clear();
        }
    }

    /** Read each commit's changes into memory until the signal aborts. */
    async #follow(signal: AbortSignal): Promise<void> {
        // start at the latest sequence
        const latest = (await this.database.log.position()).sequence;
        if (signal.aborted) {
            return;
        }
        this.#start = latest;
        this.#sequence = latest;
        this.#wake();

        // keep each page and drop the oldest changes beyond the limit
        for await (const page of this.database.log.follow(
            { tables: this.tables, after: latest },
            signal,
        )) {
            if (signal.aborted) {
                return;
            }
            this.#changes.push(...page.changes);
            this.#sequence = page.sequence;
            if (this.#changes.length > this.#memory) {
                const dropped = this.#changes.splice(0, this.#changes.length - this.#memory);
                this.#start = dropped.at(-1)!.sequence;
            }
            this.#wake();
        }
    }

    /** Wake every waiting subscriber. */
    #wake(): void {
        for (const wake of this.#waiting) {
            wake();
        }
    }

    /** Decide whether a change is of a watched table, scope and row. */
    #isWatched(watches: readonly Watch[], change: Change): boolean {
        return watches.some((entry) => {
            // require the table and scope
            if (
                entry.table !== change.table ||
                (entry.scopes !== "every" && !entry.scopes.includes(change.scope))
            ) {
                return false;
            } else if (entry.where === undefined) {
                return true;
            }

            // require the row to match before or after the change
            let match = this.#watchedRows.get(entry);
            if (match === undefined) {
                match = Condition.compile(entry.where, entry.table);
                this.#watchedRows.set(entry, match);
            }

            return [change.before, change.after].some(
                (image) => image !== undefined && Condition.matches(match, image),
            );
        });
    }
}

/** The inspection of a feed. */
export interface FeedInspection {
    /** The sequence the feed read the log up to, below zero before reading. */
    readonly sequence: number;
    /** The kept recent changes. */
    readonly changes: number;
    /** The subscribers it serves. */
    readonly subscribers: number;
    /** The subscriber limit. */
    readonly limit: number;
    /** The most rows and groups one evaluation may know. */
    readonly capacity: number;
    /** The shared evaluations with their stream counts. */
    readonly evaluations: readonly {
        /** The streams sharing the evaluation. */
        readonly streams: number;
        /** The evaluation's dataflow. */
        readonly dataflow: DataflowInspection;
    }[];
}
