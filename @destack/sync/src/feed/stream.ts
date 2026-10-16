import { DatabaseError, type LogPosition, TABLE, type Table } from "@destack/db";
import type { Page, ResultChange, RowChange } from "../query/page.ts";
import { canonicalize } from "@destack/schema";
import type { Query } from "../query/query.ts";
import type { Audience } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import type { Feed } from "./feed.ts";
import { View } from "../dataflow/view.ts";
import type { Clock } from "@destack/environment";

/** One subscriber's queries over a feed, on their own behind the head and shared at the head. */
export class Stream {
    /** The feed the stream reads. */
    readonly #feed: Feed;
    /** The queries the stream follows. */
    readonly #queries: Readonly<Record<string, Query>>;
    /** Who the stream serves. */
    readonly #audience: Audience;
    /** The followed evaluation: its own, or a shared one. */
    #evaluation: Evaluation;
    /** The shortest time between two pages at the head. */
    readonly #every: number | undefined;
    /** The subscriber's origin, whose replicated writes never advance its bare positions. */
    readonly #origin: string | undefined;
    /** The feed's beats when the stream last published, a later one repeating its position. */
    #heard: number;

    /** Create the stream of a subscriber. */
    constructor(
        feed: Feed,
        queries: Readonly<Record<string, Query>>,
        audience: Audience,
        every?: number,
        origin?: string,
    ) {
        // evaluate the queries on their own
        this.#feed = feed;
        this.#every = every;
        this.#origin = origin;
        this.#queries = queries;
        this.#audience = audience;
        this.#evaluation = new Evaluation(feed, queries, audience);
        this.#heard = feed.beats;
    }

    /** Stream the queries from a position until aborted, or to a completed page once drained. */
    async *run(
        after: LogPosition | undefined,
        signal: AbortSignal,
        previous?: Evaluation,
        drain?: AbortSignal,
    ): AsyncGenerator<Page> {
        let epoch = await this.#feed.database.log.epoch();
        let sequence: number | undefined;
        try {
            while (!signal.aborted) {
                try {
                    // continue within the epoch or start over with a snapshot
                    if (sequence === undefined) {
                        // start over with a snapshot outside the epoch
                        if (after?.epoch !== epoch) {
                            sequence = yield* this.#snapshot(epoch, signal);
                        }
                        // reshape the replica of previous queries
                        else if (previous !== undefined) {
                            sequence = yield* this.#reshape(previous, after, signal);
                        }
                        // resume within the epoch
                        else {
                            sequence = yield* this.#resume(after, signal);
                        }
                    }

                    // publish what follows
                    const following = this.#follow({ epoch, sequence }, signal, drain);
                    yield* this.#every === undefined
                        ? following
                        : Stream.coalesce(
                              following,
                              this.#every,
                              this.#feed.tables,
                              this.#feed.clock,
                          );
                    return;
                } catch (error) {
                    // start over after a lost position or epoch
                    if (
                        !(
                            error instanceof DatabaseError &&
                            (error.code === "CHANGES_COMPACTED" || error.code === "STALE_EPOCH")
                        )
                    ) {
                        throw error;
                    }
                    this.#alone();
                    epoch = await this.#feed.database.log.epoch();
                    after = undefined;
                    previous = undefined;
                    sequence = undefined;
                }
            }
        } finally {
            this.#feed.leave(this.#evaluation);
        }
    }

    /** Publish pages after a position, then the bare position once caught up, until done. */
    async *#follow(
        start: LogPosition,
        signal: AbortSignal,
        drain: AbortSignal | undefined,
    ): AsyncGenerator<Page> {
        // publish from the start until aborted, or drained between completed pages
        let position = start;
        let published = start.sequence;
        const waking = drain === undefined ? signal : AbortSignal.any([signal, drain]);
        while (!waking.aborted) {
            // join the shared evaluation
            this.#evaluation = this.#feed.join(this.#evaluation, position);
            const decided = this.#evaluation.after(position);

            // catch up on its own once the shared evaluation moved on
            if (decided === undefined) {
                this.#alone();
                await this.#audience.refresh();
                const resumed = yield* this.#resume(position, signal);
                position = { epoch: position.epoch, sequence: resumed };
            }
            // publish the decided pages before waiting or moving on
            else {
                const { pages, sequence } = await decided;
                await Evaluation.nextTask();
                for (const page of pages) {
                    published = page.position.sequence;
                    this.#heard = this.#feed.beats;
                    yield page;
                }

                // wait for a commit, or repeat the published position at the feed's next beat
                if (sequence === position.sequence) {
                    const beat = this.#feed.beating;
                    if (this.#feed.beats === this.#heard) {
                        await this.#evaluation.wait(sequence, AbortSignal.any([waking, beat]));
                    }

                    // repeat the position once the feed beat since it published
                    const isBeat = beat.aborted || this.#feed.beats !== this.#heard;
                    if (isBeat && !waking.aborted) {
                        const repeated = { epoch: position.epoch, sequence: published };
                        this.#heard = this.#feed.beats;
                        yield { reset: false, complete: true, changes: [], position: repeated };
                    }
                }
                position = { epoch: position.epoch, sequence };

                // publish the bare position once caught up past more than the subscriber's own replicated writes
                const isOwn =
                    this.#origin !== undefined &&
                    this.#feed.isOriginated(published, sequence, this.#origin);
                if (sequence >= this.#feed.sequence && sequence > published && !isOwn) {
                    published = sequence;
                    this.#heard = this.#feed.beats;
                    yield { reset: false, complete: true, changes: [], position };
                }
            }
        }
    }

    /** Send a snapshot at the latest position and what followed, returning the sequence reached. */
    async *#snapshot(epoch: string, signal: AbortSignal): AsyncGenerator<Page, number> {
        // walk the queries before catching up
        const start = { epoch, sequence: (await this.#feed.database.log.position()).sequence };
        this.#evaluation.forget(start);
        const last = yield* this.#evaluation.walk(
            new View(this.#feed.database, start, this.#feed),
            "snapshot",
        );

        return yield* this.#catchUp(start, last, signal);
    }

    /** Rebuild a resuming subscriber's state at its position, returning the sequence reached. */
    async *#resume(after: LogPosition, signal: AbortSignal): AsyncGenerator<Page, number> {
        // rebuild the selected rows and windows and send every group
        this.#evaluation.forget(after);
        const rebuilt = yield* this.#evaluation.walk(
            new View(this.#feed.database, after, this.#feed),
            "rebuild",
        );

        return yield* this.#catchUp(after, rebuilt, signal);
    }

    /** Move a subscriber from previous queries to these. */
    async *#reshape(
        previous: Evaluation,
        after: LogPosition,
        signal: AbortSignal,
    ): AsyncGenerator<Page, number> {
        // rebuild the previous queries' state
        previous.forget(after);
        const pages = previous.walk(new View(this.#feed.database, after, this.#feed), "rebuild");
        while ((await pages.next()).done !== true) {
            // rebuild without sending
        }

        // send the previous queries' changes up to now
        const start = (await this.#feed.database.log.position()).sequence;
        let sequence = after.sequence;
        while (sequence < start && !signal.aborted) {
            const advanced = yield* previous.advance(after.epoch, sequence, false, signal);
            if (advanced === sequence) {
                await this.#feed.next(sequence, signal);
            }
            sequence = advanced;
        }

        // send the difference at one position
        const position = { epoch: after.epoch, sequence };
        const moved = await this.#evaluation.moveFrom(previous, position);

        return yield* this.#catchUp(position, moved, signal);
    }

    /** Send a pending page and changes up to the latest position, returning the sequence. */
    async *#catchUp(
        position: LogPosition,
        pending: Page,
        signal: AbortSignal,
    ): AsyncGenerator<Page, number> {
        // replay the changes except the last page
        const target = (await this.#feed.database.log.position()).sequence;
        let sequence = position.sequence;
        let last = pending;
        while (sequence < target && !signal.aborted) {
            const pages = this.#evaluation.advance(position.epoch, sequence, false, signal);
            let next = await pages.next();
            while (next.done !== true) {
                // send the pending page unless empty
                if (last.reset || last.changes.length > 0 || (last.results?.length ?? 0) > 0) {
                    yield last;
                }
                last = next.value;
                next = await pages.next();
            }
            if (next.value === sequence) {
                await this.#feed.next(sequence, signal);
            }
            sequence = next.value;
        }

        // stop short of completing a page the changes never reached
        if (signal.aborted) {
            return sequence;
        }

        // complete the last page at the target
        const reached = Math.max(sequence, target);
        yield { ...last, complete: true, position: { epoch: position.epoch, sequence: reached } };

        return reached;
    }

    /** Leave a shared evaluation for one of the stream's own. */
    #alone(): void {
        if (this.#feed.isJoined(this.#evaluation)) {
            this.#feed.leave(this.#evaluation);
            this.#evaluation = new Evaluation(this.#feed, this.#queries, this.#audience);
        }
    }

    /** Publish pages at most once per interval, merging those within it. */
    static async *coalesce(
        pages: AsyncGenerator<Page>,
        every: number,
        tables: readonly Table[],
        clock: Clock,
    ): AsyncGenerator<Page> {
        // read the next page ahead, keeping its failure for the next take
        let pending: Page | undefined;
        let published = Number.NEGATIVE_INFINITY;
        let next = Stream.#settle(pages.next());
        try {
            for (;;) {
                // race the next page with the interval's end on the clock
                const due = published + every;
                const sleeping = new AbortController();
                const arrived =
                    pending === undefined
                        ? await next
                        : await Promise.race([
                              next,
                              clock.sleep(due - clock.now(), sleeping.signal).then(
                                  () => undefined,
                                  () => undefined,
                              ),
                          ]);
                sleeping.abort();

                // publish the merged page at the interval's end
                if (arrived === undefined) {
                    if (pending === undefined) {
                        throw new TypeError("an interval ended without a pending page");
                    }
                    yield pending;
                    pending = undefined;
                    published = clock.now();
                }
                // rethrow the failure of the pages
                else if (arrived.status === "rejected") {
                    throw arrived.reason;
                }
                // publish what remains at the end
                else if (arrived.value.done === true) {
                    if (pending !== undefined) {
                        yield pending;
                    }

                    return;
                }
                // merge each page, or publish at once after a quiet interval
                else {
                    next = Stream.#settle(pages.next());
                    pending =
                        pending === undefined
                            ? arrived.value.value
                            : Stream.#merge(pending, arrived.value.value, tables);
                    if (clock.now() >= due) {
                        yield pending;
                        pending = undefined;
                        published = clock.now();
                    }
                }
            }
        } finally {
            // stop the pages once the consumer stops, after the read ahead
            await pages.return(undefined);
        }
    }

    /** Merge two consecutive pages. */
    static #merge(earlier: Page, later: Page, tables: readonly Table[]): Page {
        // net each row's changes, by table and key
        const changes = new Map<string, RowChange>();
        for (const change of [...earlier.changes, ...later.changes]) {
            const table = tables.find((entry) => entry[TABLE].sqlName === change.table);
            if (table === undefined) {
                throw new TypeError(`a merged page has an unknown table: ${change.table}`);
            }
            const key = JSON.stringify([
                change.table,
                ...table[TABLE].key.map((name) => change.row[name]),
            ]);
            const known = changes.get(key);
            const net = known === undefined ? change : Stream.#netChange(known, change);
            if (net === undefined) {
                changes.delete(key);
            } else {
                changes.set(key, net);
            }
        }

        // keep each group's last values
        const results = new Map<string, ResultChange>();
        for (const result of [...(earlier.results ?? []), ...(later.results ?? [])]) {
            if (result.group === null) {
                for (const key of [...results.keys()].filter((entry) =>
                    entry.startsWith(`${result.query}\n`),
                )) {
                    results.delete(key);
                }
            }
            results.set(`${result.query}\n${canonicalize(result.group)}`, result);
        }

        return {
            reset: earlier.reset,
            complete: later.complete,
            changes: [...changes.values()],
            ...(results.size === 0 ? {} : { results: [...results.values()] }),
            position: later.position,
        };
    }

    /** Net two changes of one row. */
    static #netChange(earlier: RowChange, later: RowChange): RowChange | undefined {
        // forget a row that entered and left
        if (earlier.operation === "insert" && later.operation === "delete") {
            return undefined;
        }

        // net the two operations
        let operation = later.operation;
        if (earlier.operation === "insert" && later.operation === "update") {
            operation = "insert";
        } else if (earlier.operation === "delete" && later.operation !== "delete") {
            operation = "update";
        }

        return { ...later, operation };
    }

    /** Settle a read into its outcome, so a failure waits for its take. */
    static #settle<Value>(read: Promise<Value>): Promise<PromiseSettledResult<Value>> {
        return read.then(
            (value) => ({ status: "fulfilled", value }),
            (reason: unknown) => ({ status: "rejected", reason }),
        );
    }
}
