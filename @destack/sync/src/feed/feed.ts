import { nth, found } from "@destack/schema";
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
import type { Query, Item } from "../query/query.ts";
import type { AggregateRow } from "../query/aggregate.ts";
import { Node } from "../query/node.ts";
import { EVERYONE, type Audience, Watch } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import { Stream } from "./stream.ts";
import type { Upstream } from "../dataflow/upstream.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { PAGE_SIZE } from "../dataflow/selection.ts";
import { Run, type RunCost } from "../dataflow/run.ts";
import { Memo, View, type Cache, type Window } from "../dataflow/view.ts";
import { Clock } from "@destack/environment";

/** The default count of recent changes a feed stores in memory. */
const FEED_CHANGES = 10_000;

/** The default subscriber limit of a feed. */
const FEED_SUBSCRIBERS = 1000;

/** The default count of rows and groups one evaluation knows. */
const CAPACITY = 100_000;

/** The recent log sequences whose reads and encodings the memos store for subscribers to share. */
const MEMO_SEQUENCES = 8;

/** The default wait before a caught-up stream repeats its position, in milliseconds. */
const HEARTBEAT_MILLISECONDS = 10_000;

/** The rows of one capture page across tables: 500 rows of about 500 B make a page near 250 KB. */
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
    /** The most rows one page of a snapshot or a catch-up reads. */
    readonly pageSize: number;
    /** The count of recent changes stored in memory. */
    readonly #memory: number;
    /** Report each run's cost. */
    readonly observe: ((cost: RunCost) => void) | undefined;
    /** The wait before a caught-up stream repeats its position, in milliseconds. */
    readonly heartbeat: number;
    /** What a replica knows of its source. */
    readonly upstream: Upstream | undefined;
    /** The clock the feed's streams and decisions run at. */
    readonly clock: Clock;
    /** The signal ending every caught-up stream's wait for its next heartbeat at once. */
    #beating = new AbortController();
    /** The beats so far, which streams compare to repeat a beat they missed. */
    #beats = 0;
    /** The recent changes in commit order. */
    #changes: Change[];
    /** The sequence before the oldest stored change. */
    #start: number;
    /** The sequence the stored changes arrive at, below zero before reading. */
    #sequence: number;
    /** The subscribers waiting for the feed to pass a sequence. */
    readonly #waiting: Set<() => void>;
    /** The subscriber count. */
    #subscribers: number;
    /** Stop reading after the last subscriber. */
    #reading: AbortController | undefined;
    /** The failure that stopped reading. */
    #failure: Error | undefined;
    /** The shared evaluations by key. */
    readonly #evaluations: Map<string, Evaluation>;
    /** Each shared evaluation's key and the streams sharing it. */
    readonly #shares: Map<Evaluation, { key: string; streams: number }>;
    /** The shared reads of rows by sequence and key. */
    readonly rows: Memo<Promise<readonly Row[]>>;
    /** The shared ordered windows by sequence and key. */
    readonly windows: Memo<Promise<Window>>;
    /** The shared reads of single rows by sequence and key. */
    readonly row: Memo<Promise<Row | null>>;
    /** The shared encodings by sequence and key. */
    readonly encodings: Memo<RowChange>;
    /** The shared images by sequence and range. */
    readonly #images: Memo<Promise<ReadonlyMap<string, Row | null>>>;

    /** Serve a database's logged tables within the limits. */
    constructor(
        database: DatabaseConnection,
        tables: readonly Table[],
        options: {
            readonly subscribers?: number;
            readonly capacity?: number;
            readonly pageSize?: number;
            readonly changes?: number;
            readonly upstream?: Upstream;
            readonly observe?: (cost: RunCost) => void;
            readonly heartbeat?: number;
            readonly clock: Clock;
        },
    ) {
        // start with empty state
        this.#changes = [];
        this.#start = Number.POSITIVE_INFINITY;
        this.#sequence = -1;
        this.#waiting = new Set<() => void>();
        this.#subscribers = 0;
        this.#evaluations = new Map<string, Evaluation>();

        // start the rest of the state empty
        this.#shares = new Map<Evaluation, { key: string; streams: number }>();
        this.rows = new Memo<Promise<readonly Row[]>>(MEMO_SEQUENCES);
        this.windows = new Memo<Promise<Window>>(MEMO_SEQUENCES);
        this.row = new Memo<Promise<Row | null>>(MEMO_SEQUENCES);
        this.encodings = new Memo<RowChange>(MEMO_SEQUENCES);
        this.#images = new Memo<Promise<ReadonlyMap<string, Row | null>>>(MEMO_SEQUENCES);

        // store the logged tables and the limits
        this.database = database;
        this.upstream = options.upstream;
        this.clock = options.clock;
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
        this.pageSize = options.pageSize ?? PAGE_SIZE;
        this.#memory = options.changes ?? FEED_CHANGES;
        this.observe = options.observe;
        this.heartbeat = options.heartbeat ?? HEARTBEAT_MILLISECONDS;
    }

    /** The signal ending caught-up streams' current wait for their heartbeat. */
    get beating(): AbortSignal {
        return this.#beating.signal;
    }

    /** The beats so far. */
    get beats(): number {
        return this.#beats;
    }

    /** Beat at each heartbeat of the feed's clock until the reading stops. */
    async #beat(signal: AbortSignal): Promise<void> {
        for (;;) {
            // wait a heartbeat, ending with the reading
            const isBeating = await this.clock.sleep(this.heartbeat, signal).then(
                () => true,
                (error: unknown) => {
                    if (!signal.aborted) {
                        throw error;
                    }

                    return false;
                },
            );
            if (!isBeating) {
                return;
            }

            // count the beat, and end the caught-up streams' wait with a fresh signal for the next
            const beating = this.#beating;
            this.#beats++;
            this.#beating = new AbortController();
            beating.abort();
        }
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

    /** Follow queries from a position for an audience. */
    async *subscribe(
        queries: Readonly<Record<string, Query>>,
        after: LogPosition | undefined,
        signal: AbortSignal,
        options: FeedOptions = {},
    ): AsyncGenerator<Page> {
        // resolve the queries
        const audience = options.audience ?? EVERYONE;
        const stream = new Stream(this, queries, audience, options.every);
        const previous =
            options.previous === undefined
                ? undefined
                : new Evaluation(this, options.previous, audience);

        // refuse subscribers beyond the limit, then hand the caller its stream
        if (this.#subscribers >= this.#limit) {
            throw SyncError.of("OVERLOADED", `feed already has ${subscribersOf(this.#limit)}`);
        }
        options.opened?.(stream);
        this.#join();
        try {
            yield* stream.run(after, signal, previous, options.drain);
        } finally {
            this.#leave();
            options.closed?.(stream);
        }
    }

    /** Read every column of the queries' rows as one snapshot run, in one read transaction. */
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

    /** Read every column of some queries' rows as one snapshot run of pages across tables. */
    static async *#pages(
        database: DatabaseConnection,
        queries: Readonly<Record<string, Query>>,
        seal: ((table: Table, row: Row) => Promise<Row>) | undefined,
    ): AsyncGenerator<Page> {
        // read the queries in order into pages of rows across tables
        const position = await database.log.position();
        let changes: RowChange[] = [];
        let isFirst = true;
        for (const query of Object.values(queries)) {
            for await (const rows of Feed.#rows(database, query)) {
                // seal the rows
                const sealed =
                    seal === undefined
                        ? rows
                        : await Promise.all(rows.map((row) => seal(query.table, row)));
                let rest = sealed.map((row) => Feed.#captured(query.table, row));

                // fill the page, sending a full one once more rows follow it
                while (rest.length > 0) {
                    // send the full page
                    if (changes.length === CAPTURE_ROWS) {
                        yield { reset: isFirst, complete: false, changes, position };
                        isFirst = false;
                        changes = [];
                    }

                    // take the rows the page has room for
                    const room = CAPTURE_ROWS - changes.length;
                    changes.push(...rest.slice(0, room));
                    rest = rest.slice(room);
                }
            }
        }

        // complete the run with the last page
        yield { reset: isFirst, complete: true, changes, position };
    }

    /** Read one query's rows in key order a page at a time. */
    static async *#rows(
        database: DatabaseConnection,
        query: Query,
    ): AsyncGenerator<readonly Row[]> {
        // refuse a query the capture cannot read whole
        if (query.with !== undefined) {
            throw SyncError.of(
                "INVALID_SCOPE",
                `capture cannot read tables across scopes: ${query.table[TABLE].name}`,
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
            yield rows;
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
                changesThrough: (watches, after, through, reader) =>
                    this.changesThrough(watches, after, through, reader),
                isMaterialized: true,
                pageSize: this.pageSize,
            },
        );
        if (this.#subscribers >= this.#limit) {
            throw SyncError.of("OVERLOADED", `feed already has ${subscribersOf(this.#limit)}`);
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
                position: { ...position, sequence: read.sequence },
                isChanged: false,
            };
        }
        // step the dataflow through the changes
        else {
            const current = { ...position, sequence: read.sequence };
            const run = new Run(
                new View(this.database, current, this),
                EVERYONE,
                undefined,
                read.changes,
            );

            // yield again only when the step changed the selected rows or groups
            await dataflow.step(run);
            const isChanged = run.patch.rows.size > 0 || run.patch.results.size > 0;

            return { position: current, isChanged };
        }
    }

    /** Read the watched changes after a sequence, absent once compacted. */
    async changes(
        watches: readonly Watch[],
        sequence: number,
    ): Promise<{ readonly changes: readonly Change[]; readonly sequence: number } | undefined> {
        // read stored changes from memory
        if (sequence >= this.#start) {
            return {
                changes: this.#changes
                    .slice(this.#after(sequence))
                    .filter((change) => Watch.matches(watches, change)),
                sequence: Math.max(sequence, this.#sequence),
            };
        }

        return this.#logged(watches, sequence, this.database);
    }

    /** Read the watched changes between two sequences, absent once compacted. */
    async changesThrough(
        watches: readonly Watch[],
        after: number,
        through: number,
        reader: DatabaseConnection,
    ): Promise<Change[] | undefined> {
        const changes: Change[] = [];
        for (let sequence = after; sequence < through;) {
            // read from memory or a page of the log
            const read =
                sequence >= this.#start && this.#sequence > sequence
                    ? await this.changes(watches, sequence)
                    : await this.#logged(watches, sequence, reader);
            if (read === undefined) {
                return undefined;
            } else if (read.sequence <= sequence) {
                throw new TypeError(`log ends at ${read.sequence}, before ${through}`);
            }

            // take the changes up to the end sequence
            changes.push(...read.changes.filter((change) => change.sequence <= through));
            sequence = read.sequence;
        }

        return changes;
    }

    /** Read a page of watched changes from the log, absent once compacted. */
    async #logged(
        watches: readonly Watch[],
        sequence: number,
        reader: DatabaseConnection,
    ): Promise<{ readonly changes: readonly Change[]; readonly sequence: number } | undefined> {
        const scopes = Watch.scopes(watches);
        try {
            const read = await reader.log.read({
                tables: [...new Set(watches.map((entry) => entry.table))],
                after: sequence,
                ...(scopes === undefined ? {} : { scopes }),
            });

            return {
                changes: read.changes.filter((change) => Watch.matches(watches, change)),
                sequence: read.sequence,
            };
        } catch (error) {
            if (DatabaseError.is(error, "CHANGES_COMPACTED")) {
                return undefined;
            }
            throw error;
        }
    }

    /** Read a table's row images before their first change between two sequences, by key. */
    images(
        table: Table,
        after: number,
        upto: number,
        reader: DatabaseConnection,
    ): Promise<ReadonlyMap<string, Row | null>> {
        // share the images per range
        return this.#images.share(after, `images:${table[TABLE].sqlName}:${upto}`, async () => {
            // read from the log unless memory has the range
            if (upto <= after || after < this.#start || this.#sequence < upto) {
                return reader.log.images(table, after, upto);
            }
            const images = new Map<string, Row | null>();
            for (let index = this.#after(after); index < this.#changes.length; index += 1) {
                // take each row's first change in the range
                const change = nth(this.#changes, index);
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

    /** Share the evaluation of streams deciding alike at a position, returning the one to use. */
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
            current?.log === position.log &&
            current.epoch === position.epoch &&
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

    /** Find the index of the first stored change after a sequence. */
    #after(sequence: number): number {
        // search the stored changes in sequence order
        let low = 0;
        let high = this.#changes.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (nth(this.#changes, middle).sequence <= sequence) {
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
            const fail = (error: unknown) => {
                // report the failure to this reading's subscribers
                if (this.#reading === reading) {
                    this.#failure =
                        error instanceof Error
                            ? error
                            : new TypeError("feed reading failed", { cause: error });
                    this.#wake();
                }
            };
            this.#follow(reading.signal).catch(fail);
            this.#beat(reading.signal).catch(fail);
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
            this.windows.clear();
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

        // store each page and drop the oldest changes beyond the limit
        for await (const page of this.database.log.follow(
            { tables: this.tables, after: latest },
            signal,
            { isAdvancing: true },
        )) {
            if (signal.aborted) {
                return;
            }
            this.#changes.push(...page.changes);
            this.#sequence = page.sequence;
            if (this.#changes.length > this.#memory) {
                const dropped = this.#changes.splice(0, this.#changes.length - this.#memory);
                this.#start = nth(dropped, -1).sequence;
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

    /** Encode a captured row as an inserting change. */
    static #captured(table: Table, row: Row): RowChange {
        return { table: table[TABLE].sqlName, operation: "insert", row: table[TABLE].encode(row) };
    }
}

/** How a subscriber follows a feed's queries. */
export interface FeedOptions {
    /** The audience the rows are decided for, everyone by default. */
    readonly audience?: Audience;
    /** The queries the subscriber's replica reflects, which the first page is a difference from. */
    readonly previous?: Readonly<Record<string, Query>>;
    /** The interval pages at the head merge within, in milliseconds. */
    readonly every?: number;
    /** The signal ending the stream after its next completed page. */
    readonly drain?: AbortSignal;
    /** Receive the stream once it starts, to ask it to confirm positions. */
    readonly opened?: (stream: Stream) => void;
    /** Receive the stream once it ends, however it ends. */
    readonly closed?: (stream: Stream) => void;
}

/** The inspection of a feed. */
export interface FeedInspection {
    /** The sequence the feed read the log up to, below zero before reading. */
    readonly sequence: number;
    /** The stored recent changes. */
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

/** One value at a time from a producer to a consumer, the producer waiting for each take. */
class Handoff<Value> {
    /** The value given and not yet taken, with the wake-up of its giver. */
    #given: { readonly value: Value; readonly taken: () => void } | undefined;
    /** The wake-up of a consumer waiting for a value. */
    #waiting: (() => void) | undefined;
    /** Whether the producer ended. */
    #isEnded: boolean;
    /** The producer's failure. */
    #failure: { readonly error: unknown } | undefined;

    /** Start empty. */
    constructor() {
        // start with empty state
        this.#isEnded = false;
    }

    /** Give a value, resolving once the consumer took it, refusing once the consumer stopped. */
    give(value: Value): Promise<void> {
        // refuse a value once the consumer stopped
        if (this.#isEnded) {
            return Promise.reject(SyncError.of("STALE", "the capture's consumer stopped"));
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

/** Count subscribers in words, as one subscriber or several subscribers. */
function subscribersOf(count: number): string {
    return count === 1 ? "1 subscriber" : `${count} subscribers`;
}
