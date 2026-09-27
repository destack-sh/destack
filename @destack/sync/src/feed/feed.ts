import { Key, TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { Condition, type Match } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { describeLog, type Change, type LogPosition } from "@destack/db/log";
import { SyncError } from "../error/error.ts";
import type { Row } from "@destack/db";
import type { QueryPage } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { EVERYONE, type Audience, type Watch } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import { Stream } from "./stream.ts";
import type { Upstream } from "../dataflow/upstream.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type RunCost } from "../dataflow/run.ts";
import { View, type Cache } from "../dataflow/view.ts";

/**
 * The most recent changes a feed keeps in memory for its subscribers by default.
 *
 * At about 1 KB a change, a busy database's feed holds about 10 MB, and an idle one holds what arrived since it started.
 */
const FEED_CHANGES = 10_000;

/**
 * The most subscribers one feed serves by default.
 *
 * Streams that decide alike share one evaluation, so a row every subscriber follows costs each about 4 µs to publish, about 4 ms in all.
 */
const FEED_SUBSCRIBERS = 1000;

/**
 * The most rows and groups an evaluation's windows, arrangements and tallies know together, by default.
 *
 * Windows hold a limit of rows with their logged columns, about 1 KB each, and arrangements every candidate's sort values, about 100 B each.
 * A hundred thousand, mostly arranged entries and groups, keep an evaluation within about 20 MB.
 */
const CAPACITY = 100_000;

/** The recent log sequences whose row reads subscribers share. */
const SHARED_READS = 8;

/**
 * How long a caught-up stream waits for a commit before repeating its position, by default, in milliseconds.
 *
 * Each repeated position confirms a copy current with its source, and a copy three beats unconfirmed, 30 seconds, is stale.
 * A beat costs one empty page per stream, about a microsecond of work every ten seconds.
 */
const HEARTBEAT_MILLISECONDS = 10_000;

/** One database's log, read once per commit and served to every subscriber of its queries. */
export class Feed implements Cache {
    /** The database whose log the feed reads. */
    readonly database: DatabaseConnection;
    /** The logged tables subscribers may read. */
    readonly tables: readonly Table[];
    /** The most subscribers the feed serves at once. */
    readonly #limit: number;
    /** The most rows and groups one evaluation's windows, arrangements and tallies know together. */
    readonly capacity: number;
    /** The most recent changes the feed keeps in memory, which subscribers at or after the oldest read without the log. */
    readonly #memory: number;
    /** Report each evaluation run's cost, absent when nothing observes the feed. */
    readonly observe: ((cost: RunCost) => void) | undefined;
    /** How long a caught-up stream waits for a commit before repeating its position, in milliseconds. */
    readonly heartbeat: number;
    /** What a copy knows of its source, absent for a source database, whose own rows measure relations. */
    readonly upstream: Upstream | undefined;
    /** The recent changes in commit order. */
    #changes: Change[] = [];
    /** The sequence before the oldest kept change; subscribers at or after it read from memory. */
    #start = Number.POSITIVE_INFINITY;
    /** The sequence the kept changes reach, below zero before reading starts. */
    #sequence = -1;
    /** The subscribers waiting for the feed to pass a sequence. */
    readonly #waiting = new Set<() => void>();
    /** The number of subscribers, which keep the feed reading. */
    #subscribers = 0;
    /** Stop reading once the last subscriber leaves. */
    #reading?: AbortController;
    /** The failure that stopped reading, reported to every subscriber. */
    #failure?: unknown;
    /** The evaluations streams share at the head, by what they evaluate for whom. */
    readonly #evaluations = new Map<string, Evaluation>();
    /** Each shared evaluation's key and the streams sharing it. */
    readonly #shares = new Map<Evaluation, { key: string; streams: number }>();
    /** The reads subscribers share deciding recent sequences, by sequence and what they read. */
    readonly #reads = new Map<number, Map<string, unknown>>();
    /** The conditions of watches over the rows whose changes matter, compiled once per watch. */
    readonly #watchedRows = new WeakMap<Watch, Match>();

    /** Serve the logged ones among a database's tables to at most a limit of subscribers, each evaluation within a capacity of rows and groups, keeping a number of recent changes in memory. */
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
        // serve the logged tables within the limits, with their tree indexes unless the database is a copy
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

    /** Describe the feed for inspection: how far it read, what it keeps, whom it serves, and each shared evaluation's dataflow. */
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

    /** The sequence the feed has read the log up to. */
    get sequence(): number {
        return this.#sequence;
    }

    /**
     * Follow queries from a position for an audience, moving from previous queries when given them.
     *
     * A subscriber without a position of the log's epoch first receives a paged snapshot.
     * So does one whose position the log compacted away.
     * With an interval, pages that follow the head arrive at most once per interval, merged.
     */
    async *subscribe(
        queries: Readonly<Record<string, Query>>,
        after: LogPosition | undefined,
        signal: AbortSignal,
        options: {
            readonly audience?: Audience;
            readonly previous?: Readonly<Record<string, Query>>;
            readonly every?: number;
        } = {},
    ): AsyncGenerator<QueryPage> {
        // resolve the queries before serving them
        const audience = options.audience ?? EVERYONE;
        const stream = new Stream(this, queries, audience, options.every);
        const previous =
            options.previous === undefined
                ? undefined
                : new Evaluation(this, options.previous, audience);

        // refuse subscribers beyond the limit, and keep the feed reading while subscribed
        if (this.#subscribers >= this.#limit) {
            throw new SyncError("OVERLOADED", `feed serves ${this.#limit} subscribers already`);
        }
        this.#join();
        try {
            yield* stream.run(after, signal, previous);
        } finally {
            this.#leave();
        }
    }

    /**
     * Follow one query's result as the log changes it until the signal aborts: its rows with what each includes nested, or its groups.
     *
     * The result holds every row the query selects; a copy's feed measures relations and aggregates through its source.
     * It starts over from the latest position whenever the log no longer holds a position it needs.
     */
    async *watch(
        name: string,
        query: Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly Readonly<Record<string, unknown>>[]> {
        // compile the query, and keep the feed reading while watched
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
                // hold the query as of the latest position, and read its result
                if (position === undefined) {
                    position = await this.database.log.position();
                    await dataflow.fill(new View(this.database, position, this));
                    yield await dataflow.read(name);
                    continue;
                }

                // apply what follows, reading the result again once it changed, or wait for more
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
                    position = (await dataflow.step(run)) ? reached : undefined;
                    if (position !== undefined) {
                        yield await dataflow.read(name);
                    }
                }
            }
        } finally {
            this.#leave();
        }
    }

    /** Read the changes of watched tables and scopes after a sequence, absent once the log no longer holds them. */
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

    /** Read every change of watched tables and scopes after one sequence through another, from memory and then the log, absent once the log no longer holds them. */
    async changesThrough(
        watches: readonly Watch[],
        after: number,
        through: number,
    ): Promise<Change[] | undefined> {
        const changes: Change[] = [];
        for (let sequence = after; sequence < through;) {
            // read from memory while it holds the sequence and reaches beyond it, else a page of the log
            const read =
                sequence >= this.#start && this.#sequence > sequence
                    ? await this.changes(watches, sequence)
                    : await this.#logged(watches, sequence);
            if (read === undefined) {
                return undefined;
            } else if (read.sequence <= sequence) {
                throw new TypeError(`log ends at ${read.sequence} before ${through}`);
            }

            // keep the changes up to the sequence read through
            changes.push(...read.changes.filter((change) => change.sequence <= through));
            sequence = read.sequence;
        }

        return changes;
    }

    /** Read a page of watched tables' and scopes' changes after a sequence from the log, absent once the log no longer holds them. */
    async #logged(
        watches: readonly Watch[],
        sequence: number,
    ): Promise<{ readonly changes: readonly Change[]; readonly sequence: number } | undefined> {
        try {
            const read = await this.database.log.read({
                tables: [...new Set(watches.map((entry) => entry.table))],
                after: sequence,
                scopes: [...new Set(watches.flatMap((entry) => entry.scopes))],
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

    /**
     * Read the images a table's rows had before their first change after a sequence, up to another, by key.
     *
     * A row inserted after the sequence has a null image; overlaying the images on current rows restores the table as it was.
     */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>> {
        // reuse images of the same table after the same sequence that cover as far
        return this.share(after, `images:${table[TABLE].sqlName}:${upto}`, async () => {
            // take them from memory when the feed holds every change of the range, else from the log
            if (upto <= after || after < this.#start || this.#sequence < upto) {
                return this.database.log.images(table, after, upto);
            }
            const images = new Map<string, Row | null>();
            for (let index = this.#after(after); index < this.#changes.length; index += 1) {
                // keep the table's first change in the range of each row
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

    /**
     * Compute once, for every subscriber deciding the same log sequence, what a key names: a read or an encoding.
     *
     * A read that fails is forgotten, so that the next subscriber reads again, and the reads of the lowest sequences go first.
     */
    share<Value>(sequence: number, key: string, compute: () => Value): Value {
        // reuse a read made for the same sequence
        let reads = this.#reads.get(sequence);
        if (reads?.has(key) === true) {
            return reads.get(key) as Value;
        }

        // start the sequence's reads, letting go of the lowest sequence's beyond the kept number
        if (reads === undefined) {
            reads = new Map<string, unknown>();
            this.#reads.set(sequence, reads);
            if (this.#reads.size > SHARED_READS) {
                this.#reads.delete(Math.min(...this.#reads.keys()));
            }
        }

        // read, forgetting a read that fails
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

    /** Decide whether a read for a sequence is shared already. */
    isShared(sequence: number, key: string): boolean {
        return this.#reads.get(sequence)?.has(key) === true;
    }

    /**
     * Share the evaluation of streams deciding alike at a position, returning the one a stream follows from there.
     *
     * A stream shares its own evaluation when none decides alike yet; it keeps its own while the shared one holds another position.
     */
    join(evaluation: Evaluation, position: LogPosition): Evaluation {
        // follow the key of a shared evaluation whose audience's decisions changed
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

        // join the shared evaluation at the same position, or share this one when none decides alike
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

    /** Stop sharing an evaluation with a stream that left it, forgetting it once no stream shares it. */
    leave(evaluation: Evaluation): void {
        // ignore a stream that evaluates alone
        const share = this.#shares.get(evaluation);
        if (share === undefined) {
            return;
        }

        // count the stream out, forgetting the evaluation after the last one
        share.streams -= 1;
        if (share.streams === 0) {
            this.#evaluations.delete(share.key);
            this.#shares.delete(evaluation);
        }
    }

    /** Decide whether streams share an evaluation through the feed. */
    isJoined(evaluation: Evaluation): boolean {
        return this.#shares.has(evaluation);
    }

    /** Wait until the feed passes a sequence, reading fails, or the signal aborts. */
    async next(sequence: number, signal: AbortSignal): Promise<void> {
        await new Promise<void>((resolve) => {
            // wake at once when already past, failed or aborted
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

        // report the failure that stopped reading
        if (this.#failure !== undefined) {
            throw this.#failure;
        }
    }

    /** Find the index of the first kept change after a sequence. */
    #after(sequence: number): number {
        // search the kept changes, which are in sequence order
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
                // report the failure to this reading's subscribers only
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

    /** Read every commit's changes into memory until the signal aborts, keeping the most recent. */
    async #follow(signal: AbortSignal): Promise<void> {
        // start at the latest sequence with nothing kept
        const latest = await this.database.log.latest();
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

    /** Let every waiting subscriber check whether it can continue. */
    #wake(): void {
        for (const wake of this.#waiting) {
            wake();
        }
    }

    /** Decide whether a change is of a watched table, scope and row, as the row was or is. */
    #isWatched(watches: readonly Watch[], change: Change): boolean {
        return watches.some((entry) => {
            // require the table and scope
            if (entry.table !== change.table || !entry.scopes.includes(change.scope)) {
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

/** What a feed holds and serves, as inspection reads it. */
export interface FeedInspection {
    /** The sequence the feed read the log up to, below zero before reading starts. */
    readonly sequence: number;
    /** The recent changes it keeps in memory. */
    readonly changes: number;
    /** The subscribers it serves. */
    readonly subscribers: number;
    /** The most subscribers it serves at once. */
    readonly limit: number;
    /** The most rows and groups one evaluation may know. */
    readonly capacity: number;
    /** The evaluations streams share at the head, with how many streams share each. */
    readonly evaluations: readonly {
        /** The streams sharing the evaluation. */
        readonly streams: number;
        /** The evaluation's dataflow. */
        readonly dataflow: DataflowInspection;
    }[];
}
