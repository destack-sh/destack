import { DatabaseError, type LogPosition, TABLE, type Table } from "@destack/db";
import type { Page, ResultChange, RowChange } from "../query/page.ts";
import { canonicalize, type JsonObject } from "@destack/schema";
import type { Query } from "../query/query.ts";
import { Node } from "../query/node.ts";
import type { Audience } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import type { Feed } from "./feed.ts";
import { View } from "../dataflow/view.ts";
import type { Clock } from "@destack/environment";
import { SyncError } from "../error/error.ts";
import type { RequestKey, SubscriptionRequest } from "../replica/publication.ts";

/** One subscriber's queries over a feed, alone behind the head and shared at the head. */
export class Stream {
    /** The feed the stream reads. */
    readonly #feed: Feed;
    /** The queries the stream follows. */
    #queries: Readonly<Record<string, Query>>;
    /** The queries a subscriber asked the stream to move to, absent once it moved. */
    #requested: Reshape | undefined;
    /** The subscriber's request the stream follows, absent without one. */
    readonly #request: StreamRequest | undefined;
    /** The parameters the stream reflects, absent for a stream without a request. */
    #parameters: JsonObject | undefined;
    /** Aborts once the stream must end: its caller's session lapsed or its request failed. */
    readonly #ended = new AbortController();
    /** Disarms the session's lapse once the stream ends. */
    readonly #disarm = new AbortController();
    /** Who the stream serves. */
    readonly #audience: Audience;
    /** The followed evaluation: a separate one, or a shared one. */
    #evaluation: Evaluation;
    /** The shortest time between two pages at the head. */
    readonly #every: number | undefined;
    /** The highest position the subscriber asked the stream to confirm, absent before any ask. */
    #confirming: LogPosition | undefined;
    /** The log and epoch the stream publishes, absent before it starts. */
    #published: Omit<LogPosition, "sequence"> | undefined;
    /** The controller that wakes the stream's wait once a subscriber asks for a position. */
    #asked = new AbortController();
    /** The feed's beats when the stream last published, a later one making it publish again. */
    #heard: number;

    /** Create the stream of a subscriber. */
    constructor(
        feed: Feed,
        queries: Readonly<Record<string, Query>>,
        audience: Audience,
        every?: number,
        request?: StreamRequest,
    ) {
        // evaluate the queries alone
        this.#feed = feed;
        this.#request = request;
        this.#parameters = request?.parameters;
        this.#every = every;
        this.#queries = queries;
        this.#audience = audience;
        this.#evaluation = new Evaluation(feed, queries, audience);
        this.#heard = feed.beats;
    }

    /** Move the stream to other queries at its next position, waking its wait. */
    reshape(queries: Readonly<Record<string, Query>>, parameters?: JsonObject): void {
        // record the queries, then wake the wait
        this.#requested = { queries, ...(parameters === undefined ? {} : { parameters }) };
        const asked = this.#asked;
        this.#asked = new AbortController();
        asked.abort();
    }

    /** Ask the stream to publish a position at or past a position. */
    confirm(position: LogPosition): void {
        // take a higher ask of the published log and epoch, waking the stream's wait
        const confirming = this.#confirming;
        const published = this.#published;
        const isPublished =
            published === undefined ||
            (position.log === published.log && position.epoch === published.epoch);
        const isHigher =
            confirming === undefined ||
            confirming.log !== position.log ||
            confirming.epoch !== position.epoch ||
            position.sequence > confirming.sequence;
        if (isPublished && isHigher) {
            this.#confirming = position;
            const asked = this.#asked;
            this.#asked = new AbortController();
            asked.abort();
        }
    }

    /** Key the tables some queries read, in order. */
    static #tables(queries: Readonly<Record<string, Query>>): string {
        const tables = Object.values(queries).flatMap((query) =>
            Node.tables(query).map((table) => table[TABLE].sqlName),
        );

        return [...new Set(tables)].toSorted().join("\n");
    }

    /** Report whether the stream arrived at an asked position its follower lacks. */
    #isAsked(recorded: number, position: LogPosition): boolean {
        const confirming = this.#confirming;

        return (
            confirming !== undefined &&
            confirming.log === position.log &&
            confirming.epoch === position.epoch &&
            confirming.sequence > recorded &&
            position.sequence >= confirming.sequence
        );
    }

    /** Read the sequence asked past what the follower recorded, absent for none. */
    #ahead(recorded: number): number | undefined {
        const confirming = this.#confirming;
        const current = this.#published;

        return confirming !== undefined &&
            current !== undefined &&
            confirming.log === current.log &&
            confirming.epoch === current.epoch &&
            confirming.sequence > recorded
            ? confirming.sequence
            : undefined;
    }

    /** Stream the queries from a position until aborted, or to a completed page once drained. */
    async *run(
        after: LogPosition | undefined,
        signal: AbortSignal,
        previous?: Evaluation,
        drain?: AbortSignal,
    ): AsyncGenerator<Page> {
        // follow the subscriber's request, ending at a completed page once its session lapses
        drain = AbortSignal.any([...(drain === undefined ? [] : [drain]), this.#ended.signal]);
        const following = new AbortController();
        const requesting = this.#followRequest(AbortSignal.any([signal, following.signal]));
        const lapsesAt = this.#request?.lapsesAt;
        if (lapsesAt !== undefined) {
            const { clock } = this.#feed;
            clock.sleep(lapsesAt - clock.now(), this.#disarm.signal).then(
                () =>
                    this.#ended.abort(
                        SyncError.of("CREDENTIAL_LAPSED", "the caller's session lapsed"),
                    ),
                () => {},
            );
        }

        // publish the log's current epoch
        const { log, epoch: started } = await this.#feed.database.log.position();
        let epoch = started;
        this.#published = { log, epoch };
        let sequence: number | undefined;
        try {
            while (!signal.aborted) {
                try {
                    // continue within the epoch or start over with a snapshot
                    if (sequence === undefined) {
                        // start over with a snapshot outside the epoch
                        if (after?.log !== log || after.epoch !== epoch) {
                            sequence = yield* this.#snapshot(signal);
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
                    const published = this.#follow({ log, epoch, sequence }, signal, drain);
                    yield* this.#every === undefined
                        ? published
                        : Stream.coalesce(
                              published,
                              this.#every,
                              this.#feed.tables,
                              this.#feed.clock,
                          );

                    // end the stream once it must, after the completed page it drained to
                    const ended = this.#ended.signal;
                    if (ended.aborted && !signal.aborted) {
                        throw ended.reason;
                    }

                    return;
                } catch (error) {
                    // start over after a lost position or epoch
                    if (
                        !(
                            DatabaseError.is(error) &&
                            (error.reason === "CHANGES_COMPACTED" || error.reason === "EPOCH_STALE")
                        )
                    ) {
                        throw error;
                    }
                    this.#alone();
                    epoch = (await this.#feed.database.log.position()).epoch;
                    this.#published = { log, epoch };
                    after = undefined;
                    previous = undefined;
                    sequence = undefined;
                }
            }
        } finally {
            this.#feed.leave(this.#evaluation);
            following.abort();
            this.#disarm.abort();
            await requesting;
        }
    }

    /** Follow the subscriber's request: its asks, its credential's lapse and its parameters. */
    async #followRequest(signal: AbortSignal): Promise<void> {
        // follow nothing without a request
        const request = this.#request;
        if (request === undefined) {
            return;
        }

        // apply the request's row now and after each change, in order
        let applying = Promise.resolve();
        await request.feed
            .followRequest(
                request.key,
                (row) => {
                    applying = applying
                        .then(() => this.#applyRequest(request, row))
                        .catch((error: unknown) => {
                            this.#ended.abort(error);
                        });
                },
                signal,
            )
            .then(() => applying)
            .catch((error: unknown) => {
                // end the stream with a failure to follow the request
                if (!signal.aborted) {
                    this.#ended.abort(error);
                }
            });
    }

    /** Apply a request's ask, its credential's lapse, and its changed parameters. */
    async #applyRequest(
        request: StreamRequest,
        row: SubscriptionRequest | undefined,
    ): Promise<void> {
        if (row === undefined) {
            return;
        }

        // ask for the confirmation the subscriber waits for
        if (row.confirming !== null) {
            this.confirm(row.confirming);
        }

        // move in place to the queries of changed parameters over the same tables
        if (canonicalize(row.parameters) !== canonicalize(this.#parameters ?? null)) {
            this.#parameters = row.parameters;
            const queries = await request.queries(row.parameters);
            if (
                Stream.#tables(queries) ===
                Stream.#tables(this.#requested?.queries ?? this.#queries)
            ) {
                this.reshape(queries, row.parameters);
            }
        }
    }

    /** Publish pages after a position, then positions at each heartbeat and ask, until done. */
    async *#follow(
        start: LogPosition,
        signal: AbortSignal,
        drain: AbortSignal | undefined,
    ): AsyncGenerator<Page> {
        // publish from the start until aborted, or drained between completed pages
        const { log, epoch } = start;
        let position = start;
        let published = start.sequence;
        // track the last position the follower records, unset to answer a standing ask once
        let recorded = Number.NEGATIVE_INFINITY;
        const waking = drain === undefined ? signal : AbortSignal.any([signal, drain]);
        while (!waking.aborted) {
            // move to the queries a subscriber asked for, alone, at the current position
            const requested = this.#requested;
            if (requested !== undefined) {
                this.#requested = undefined;
                yield* this.#reshapeAt(requested, position, signal);
                this.#queries = requested.queries;
                published = position.sequence;
                recorded = position.sequence;
                this.#heard = this.#feed.beats;
            }

            // join the shared evaluation
            this.#evaluation = this.#feed.join(this.#evaluation, position);
            const decided = this.#evaluation.after(position);

            // catch up alone once the shared evaluation moved on
            if (decided === undefined) {
                this.#alone();
                await this.#audience.refresh();
                const resumed = yield* this.#resume(position, signal);
                position = { log, epoch, sequence: resumed };
            }
            // publish the decided pages before waiting or moving on
            else {
                const { pages, sequence } = await decided;
                await Evaluation.nextTask(this.#feed.clock);
                for (const page of pages) {
                    published = page.position.sequence;
                    if (page.reset || page.changes.length > 0) {
                        recorded = published;
                    }
                    this.#heard = this.#feed.beats;
                    yield page;
                }

                // wait for a commit, an ask or a reshape, else publish the position at the next beat
                const asked = { log, epoch, sequence };
                const ahead = this.#ahead(recorded);
                const isAskPassed = ahead !== undefined && this.#feed.sequence >= ahead;
                if (
                    sequence === position.sequence &&
                    this.#requested === undefined &&
                    !this.#isAsked(recorded, asked) &&
                    !isAskPassed
                ) {
                    const beat = this.#feed.beating;
                    if (this.#feed.beats === this.#heard) {
                        const woken = AbortSignal.any([waking, beat, this.#asked.signal]);
                        await (ahead === undefined
                            ? this.#evaluation.wait(sequence, woken)
                            : this.#feed.next(ahead - 1, woken));
                    }

                    // publish the current position once the feed beat since the stream published
                    const isBeat = beat.aborted || this.#feed.beats !== this.#heard;
                    if (isBeat && !waking.aborted) {
                        const current = { log, epoch, sequence };
                        const isConfirmation = this.#isAsked(recorded, current);
                        published = sequence;
                        if (isConfirmation) {
                            recorded = sequence;
                        }
                        this.#heard = this.#feed.beats;
                        yield {
                            reset: false,
                            complete: true,
                            changes: [],
                            position: current,
                            ...(isConfirmation ? { isConfirmation } : {}),
                        };
                    }
                }
                position = { log, epoch, sequence };

                // confirm the position once past the sequence the subscriber asked to confirm
                if (this.#isAsked(recorded, position)) {
                    published = sequence;
                    recorded = sequence;
                    this.#heard = this.#feed.beats;
                    yield {
                        reset: false,
                        complete: true,
                        changes: [],
                        position,
                        isConfirmation: true,
                    };
                }
            }
        }

        // end a drained stream at a completed page of its position, unless it just sent that page
        if (!signal.aborted && published < position.sequence) {
            yield { reset: false, complete: true, changes: [], position };
        }
    }

    /** Move the evaluation to other queries at a position, sending the difference. */
    async *#reshapeAt(
        requested: Reshape,
        position: LogPosition,
        signal: AbortSignal,
    ): AsyncGenerator<Page> {
        // take a separate evaluation, rebuilt at the position when it was shared
        if (this.#feed.isJoined(this.#evaluation)) {
            this.#alone();
            const rebuilt = this.#evaluation.walk(this.#view(position), "rebuild");
            while ((await rebuilt.next()).done !== true && !signal.aborted) {
                // rebuild without sending
            }
        }

        // send what entered and left at the position
        const page = await this.#evaluation.reshape(requested.queries);
        const { parameters } = requested;
        yield { ...page, complete: true, ...(parameters === undefined ? {} : { parameters }) };
    }

    /** Show the feed's database as of a position. */
    #view(position: LogPosition): View {
        return new View(this.#feed.database, position, this.#feed);
    }

    /** Send a snapshot at the latest position and what followed, returning the last sequence. */
    async *#snapshot(signal: AbortSignal): AsyncGenerator<Page, number> {
        // walk the queries before catching up
        const start = await this.#feed.database.log.position();
        this.#evaluation.forget(start);
        const last = yield* this.#evaluation.walk(
            new View(this.#feed.database, start, this.#feed),
            "snapshot",
        );

        return yield* this.#catchUp(start, last, signal);
    }

    /** Rebuild a resuming subscriber's state at its position, returning the last sequence. */
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
            const advanced = yield* previous.advance({ ...after, sequence }, false, signal);
            if (advanced === sequence) {
                await this.#feed.next(sequence, signal);
            }
            sequence = advanced;
        }

        // send the difference at one position
        const position = { ...after, sequence };
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
            const pages = this.#evaluation.advance({ ...position, sequence }, false, signal);
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

        // stop short of completing a page the changes never arrived at
        if (signal.aborted) {
            return sequence;
        }

        // complete the last page at the target
        const final = Math.max(sequence, target);
        yield { ...last, complete: true, position: { ...position, sequence: final } };

        return final;
    }

    /** Leave a shared evaluation for a separate one of the stream. */
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
        // read the next page ahead, storing its failure for the next take
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

        // record each group's last values
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

/** A subscriber's request a stream follows at its source. */
export interface StreamRequest {
    /** The feed of the database recording the request. */
    readonly feed: Feed;
    /** The request's key at the source. */
    readonly key: RequestKey;
    /** When the caller's session ends the stream, absent in process, in UTC epoch milliseconds. */
    readonly lapsesAt?: number;
    /** The parameters the stream starts with. */
    readonly parameters: JsonObject;
    /** Compile the queries of a request's parameters. */
    readonly queries: (parameters: JsonObject) => Promise<Readonly<Record<string, Query>>>;
}

/** The queries a stream moves to, with the parameters they reflect. */
interface Reshape {
    /** The queries. */
    readonly queries: Readonly<Record<string, Query>>;
    /** The parameters, absent for a move of the source's own. */
    readonly parameters?: JsonObject;
}
