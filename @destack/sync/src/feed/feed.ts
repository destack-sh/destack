import { aligned, found } from "@destack/schema";
import {
    and,
    Key,
    TABLE,
    type DatabaseConnection,
    type Table,
    Condition,
    Order,
    DatabaseError,
    describeLog,
    Change,
    type LogPosition,
    type Row,
} from "@destack/db";
import { SyncError } from "../error/error.ts";
import type { Page, RowChange } from "../query/page.ts";
import type { Query, Item, AggregateRow } from "../query/query.ts";
import { Node } from "../query/node.ts";
import { EVERYONE, watchedScopes, type Audience, Watch } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import { Stream } from "./stream.ts";
import type { Upstream } from "../dataflow/upstream.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type RunCost } from "../dataflow/run.ts";
import { Memo, View, type Cache } from "../dataflow/view.ts";

/**
 * The default count of recent changes a feed keeps in memory.
 *
 * At about 1 KB a change, a busy feed keeps about 10 MB.
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

/** The recent log sequences whose reads and encodings the memos keep for subscribers to share. */
const MEMO_SEQUENCES = 8;

/**
 * The default wait before a caught-up stream repeats its position, in milliseconds.
 *
 * A copy three beats unconfirmed, 30 seconds, is stale.
 */
const HEARTBEAT_MILLISECONDS = 10_000;

/**
 * The rows of one page of a capture.
 *
 * At 0.1 to 2 KB a row, a page is up to about 1 MB.
 */
const CAPTURE_ROWS = 500;

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
    #reading: AbortController | undefined;
    /** The failure that stopped reading. */
    #failure: Error | undefined;
    /** The shared evaluations by key. */
    readonly #evaluations = new Map<string, Evaluation>();
    /** Each shared evaluation's key and the streams sharing it. */
    readonly #shares = new Map<Evaluation, { key: string; streams: number }>();
    /** The shared reads of rows by sequence and key. */
    readonly rows = new Memo<Promise<readonly Row[]>>(MEMO_SEQUENCES);
    /** The shared reads of single rows by sequence and key. */
    readonly row = new Memo<Promise<Row | null>>(MEMO_SEQUENCES);
    /** The shared encodings by sequence and key. */
    readonly encodings = new Memo<RowChange>(MEMO_SEQUENCES);
    /** The shared images by sequence and range. */
    readonly #images = new Memo<Promise<ReadonlyMap<string, Row | null>>>(MEMO_SEQUENCES);

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
        options: FeedOptions = {},
    ): AsyncGenerator<Page> {
        // resolve the queries
        const audience = options.audience ?? EVERYONE;
        const stream = new Stream(this, queries, audience, options.every, options.origin);
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

    /**
     * Read every column of the queries' rows as one consistent snapshot run, in one read transaction.
     *
     * The run includes what the log leaves out: unlogged tables and sensitive columns.
     * Sealing turns a row's machine-bound values into values only the target opens.
     */
    async *capture(
        queries: Readonly<Record<string, Query>>,
        signal: AbortSignal,
        options: {
            readonly seal?: (table: Table, row: Row) => Promise<Row>;
        } = {},
    ): AsyncGenerator<Page> {
        // read in one transaction, handing each page over before reading the next
        const handoff = new Handoff<Page>();
        const reading = this.database
            .transaction(
                async (transaction) => {
                    for await (const page of Feed.#pages(transaction, queries, options.seal)) {
                        await handoff.give(page);
                    }
                    handoff.end();
                },
                { isReadOnly: true, isolationLevel: "repeatable read", signal },
            )
            .catch((error: unknown) => handoff.fail(error));
        try {
            yield* handoff.take();
        } finally {
            handoff.end();
            await reading;
        }
    }

    /** Read every column of some queries' rows as pages of one snapshot run at the database's position. */
    static async *#pages(
        database: DatabaseConnection,
        queries: Readonly<Record<string, Query>>,
        seal: ((table: Table, row: Row) => Promise<Row>) | undefined,
    ): AsyncGenerator<Page> {
        // read every query's rows page by page in key order
        const position = await database.log.position();
        const entries = Object.values(queries);
        let isFirst = true;
        for (const [index, query] of entries.entries()) {
            // send each page sealed and complete the run with the last page
            for await (const { rows, isLast } of Feed.#tablePages(database, query)) {
                const sealed =
                    seal === undefined
                        ? rows
                        : await Promise.all(rows.map((row) => seal(query.table, row)));
                yield {
                    reset: isFirst,
                    complete: isLast && index === entries.length - 1,
                    changes: sealed.map((row) => captured(query.table, row)),
                    position,
                };
                isFirst = false;
            }
        }
    }

    /** Read one query's rows in key order a page at a time. */
    static async *#tablePages(
        database: DatabaseConnection,
        query: Query,
    ): AsyncGenerator<{ readonly rows: readonly Row[]; readonly isLast: boolean }> {
        // refuse a query the capture cannot read whole
        if (query.with !== undefined) {
            throw new SyncError(
                "INVALID_SCOPE",
                `capture reads no tables across scopes: ${query.table[TABLE].name}`,
            );
        }
        const table = query.table;
        const order = Order.complete([], table);
        const where = Condition.render(
            { AND: [Node.scoped(query.scopes), query.where ?? {}] },
            table,
        );
        let last: Row | undefined;
        do {
            // read the next page of the table's rows
            const rows: readonly Row[] = await database
                .select()
                .from(table)
                .where(and(where, last === undefined ? undefined : Order.after(order, table, last)))
                .orderBy(...Order.render(order, table))
                .limit(CAPTURE_ROWS);
            last = rows.length === CAPTURE_ROWS ? rows.at(-1) : undefined;
            yield { rows, isLast: last === undefined };
        } while (last !== undefined);
    }

    /** Follow one query's items until the signal aborts. */
    watch(name: string, query: Query, signal: AbortSignal): AsyncGenerator<readonly Item[]> {
        return this.#watched(name, query, signal, (dataflow) => dataflow.read(name));
    }

    /** Follow one aggregate query's groups until the signal aborts. */
    watchResults(
        name: string,
        query: Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly AggregateRow[]> {
        return this.#watched(name, query, signal, (dataflow) => dataflow.results(name));
    }

    /** Follow one query's result, read again after each change of it, until the signal aborts. */
    async *#watched<Result>(
        name: string,
        query: Query,
        signal: AbortSignal,
        resultOf: (dataflow: Dataflow) => Promise<Result>,
    ): AsyncGenerator<Result> {
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
                // run the query and read its result
                if (position === undefined) {
                    position = await this.database.log.position();
                    await dataflow.load(new View(this.database, position, this));
                    yield await resultOf(dataflow);
                    continue;
                }

                // apply the changes and read the result again when they changed it
                const stepped = await this.#stepWatched(dataflow, position, signal);
                position = stepped.position;
                if (stepped.isChanged) {
                    yield await resultOf(dataflow);
                }
            }
        } finally {
            this.#leave();
        }
    }

    /** Apply the changes after a watched query's position. */
    async #stepWatched(
        dataflow: Dataflow,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<{ readonly position: LogPosition | undefined; readonly isChanged: boolean }> {
        // start over once compacted
        const read = await this.changes(dataflow.watches(), position.sequence);
        if (read === undefined) {
            return { position: undefined, isChanged: false };
        }
        // wait past a read without watched changes
        else if (read.changes.length === 0) {
            await this.next(read.sequence, signal);

            return {
                position: { epoch: position.epoch, sequence: read.sequence },
                isChanged: false,
            };
        }
        // step the dataflow through the changes
        else {
            const reached = { epoch: position.epoch, sequence: read.sequence };
            const run = new Run(
                new View(this.database, reached, this),
                EVERYONE,
                undefined,
                read.changes,
            );

            // yield again only when the step changed the selected rows or groups
            const stepped = (await dataflow.step(run)) ? reached : undefined;
            const isChanged = run.patch.rows.size > 0 || run.patch.results.size > 0;

            return { position: stepped, isChanged: stepped !== undefined && isChanged };
        }
    }

    /** Report whether every kept change between two sequences, at least one, replicated an origin's writes, as far as memory keeps them. */
    isOriginated(after: number, through: number, origin: string): boolean {
        // know nothing of a range memory does not keep
        if (after < this.#start || this.#sequence < through) {
            return false;
        }

        // look for a change of another origin, and for one of this origin
        let isFound = false;
        for (let index = this.#after(after); index < this.#changes.length; index += 1) {
            const change = aligned(this.#changes, index);
            if (change.sequence > through) {
                break;
            } else if (change.origin !== origin) {
                return false;
            }
            isFound = true;
        }

        return isFound;
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
                    .filter((change) => Watch.matches(watches, change)),
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
                changes: read.changes.filter((change) => Watch.matches(watches, change)),
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
        return this.#images.share(after, `images:${table[TABLE].sqlName}:${upto}`, async () => {
            // read from the log unless memory has the range
            if (upto <= after || after < this.#start || this.#sequence < upto) {
                return this.database.log.images(table, after, upto);
            }
            const images = new Map<string, Row | null>();
            for (let index = this.#after(after); index < this.#changes.length; index += 1) {
                // keep each row's first change in the range
                const change = aligned(this.#changes, index);
                if (change.sequence > upto) {
                    break;
                }
                const key = change.table === table ? Key.name(table, change.key) : undefined;
                if (key !== undefined && !images.has(key)) {
                    images.set(key, Change.before(change));
                }
            }

            return images;
        });
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
        const current = shared?.position;
        if (
            shared !== undefined &&
            current?.epoch === position.epoch &&
            current.sequence === position.sequence
        ) {
            found(this.#shares, shared).streams += 1;

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
            if (aligned(this.#changes, middle).sequence <= sequence) {
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
                    this.#failure =
                        error instanceof Error
                            ? error
                            : new TypeError("feed reading failed", { cause: error });
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
            this.rows.clear();
            this.row.clear();
            this.encodings.clear();
            this.#images.clear();
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
                this.#start = aligned(dropped, dropped.length - 1).sequence;
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
}

/** Encode a captured row as an inserting change. */
function captured(table: Table, row: Row): RowChange {
    return { table: table[TABLE].sqlName, operation: "insert", row: table[TABLE].encode(row) };
}

/** How a subscriber follows a feed's queries. */
export interface FeedOptions {
    /** The audience the rows are decided for, everyone by default. */
    readonly audience?: Audience;
    /** The queries the subscriber's copy reflects, which the first page is a difference from. */
    readonly previous?: Readonly<Record<string, Query>>;
    /** The interval pages at the head merge within, in milliseconds. */
    readonly every?: number;
    /** The signal ending the stream after its next completed page. */
    readonly drain?: AbortSignal;
    /** The subscriber's own origin, whose replicated writes never advance the bare positions it receives. */
    readonly origin?: string;
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

/** One value at a time from a producer to a consumer, the producer waiting until the consumer takes each. */
class Handoff<Value> {
    /** The value given and not yet taken, with the wake-up of its giver. */
    #given: { readonly value: Value; readonly taken: () => void } | undefined;
    /** The wake-up of a consumer waiting for a value. */
    #waiting: (() => void) | undefined;
    /** Whether the producer ended. */
    #isEnded = false;
    /** The producer's failure. */
    #failure: { readonly error: unknown } | undefined;

    /** Give a value, resolving once the consumer took it, and refusing once the consumer stopped. */
    give(value: Value): Promise<void> {
        // refuse a value once the consumer stopped
        if (this.#isEnded) {
            return Promise.reject(new SyncError("STALE", "the capture's consumer stopped"));
        }

        return new Promise((taken) => {
            this.#given = { value, taken };
            this.#wake();
        });
    }

    /** End the values. */
    end(): void {
        this.#isEnded = true;
        this.#given?.taken();
        this.#wake();
    }

    /** End the values with the producer's failure. */
    fail(error: unknown): void {
        this.#failure = { error };
        this.end();
    }

    /** Take the values in order until the producer ends, rethrowing its failure. */
    async *take(): AsyncGenerator<Value> {
        for (;;) {
            // wait for a value or the end
            if (this.#given === undefined && !this.#isEnded) {
                await new Promise<void>((wake) => {
                    this.#waiting = wake;
                });
            }

            // hand over the given value
            const given = this.#given;
            if (given !== undefined) {
                this.#given = undefined;
                given.taken();
                yield given.value;
            }
            // rethrow the producer's failure
            else if (this.#failure !== undefined) {
                throw this.#failure.error;
            }
            // stop at the end
            else {
                return;
            }
        }
    }

    /** Wake a waiting consumer. */
    #wake(): void {
        this.#waiting?.();
        this.#waiting = undefined;
    }
}
