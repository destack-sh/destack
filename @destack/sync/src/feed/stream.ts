import { DatabaseError } from "@destack/db/error";
import type { LogPosition } from "@destack/db/log";
import type { QueryPage, ResultChange, RowChange } from "../query/page.ts";
import { TABLE, type Table } from "@destack/db";
import { canonicalize } from "@destack/schema/json";
import type { Query } from "../query/query.ts";
import type { Audience } from "./audience.ts";
import { Evaluation, nextTask, Restart } from "./evaluation.ts";
import type { Feed } from "./feed.ts";
import { View } from "../dataflow/view.ts";

/** One subscriber's queries over a feed, alone behind the head and shared at the head. */
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

    /** Create the stream of a subscriber. */
    constructor(
        feed: Feed,
        queries: Readonly<Record<string, Query>>,
        audience: Audience,
        every?: number,
    ) {
        // evaluate the queries alone
        this.#feed = feed;
        this.#every = every;
        this.#queries = queries;
        this.#audience = audience;
        this.#evaluation = new Evaluation(feed, queries, audience);
    }

    /**
     * Stream the queries from a position until aborted, or to a completed page once drained.
     *
     * A stream without a current position starts with a snapshot.
     * From previous queries, it moves the subscriber's rows to these.
     */
    async *run(
        after: LogPosition | undefined,
        signal: AbortSignal,
        previous?: Evaluation,
        drain?: AbortSignal,
    ): AsyncGenerator<QueryPage> {
        let epoch = await this.#feed.database.log.epoch();
        let sequence: number | undefined;
        try {
            while (!signal.aborted) {
                try {
                    // continue within the epoch or start over with a snapshot
                    if (sequence === undefined) {
                        sequence =
                            after?.epoch !== epoch
                                ? yield* this.#snapshot(epoch, signal)
                                : previous !== undefined
                                  ? yield* this.#reshape(previous, after, signal)
                                  : yield* this.#resume(after, signal);
                    }

                    // publish what follows
                    const following = this.#follow({ epoch, sequence }, signal, drain);
                    yield* this.#every === undefined
                        ? following
                        : coalesce(following, this.#every, this.#feed.tables);
                    return;
                } catch (error) {
                    // start over after a lost position, epoch or access change
                    if (
                        !(error instanceof Restart) &&
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

    /** Publish the pages after a position and the bare position once caught up, until aborted or drained. */
    async *#follow(
        start: LogPosition,
        signal: AbortSignal,
        drain: AbortSignal | undefined,
    ): AsyncGenerator<QueryPage> {
        // publish from the start until aborted, or drained between completed pages
        let position = start;
        let published = start.sequence;
        const waking = drain === undefined ? signal : AbortSignal.any([signal, drain]);
        while (!waking.aborted) {
            // join the shared evaluation
            this.#evaluation = this.#feed.join(this.#evaluation, position);
            const decided = this.#evaluation.after(position);

            // catch up alone once the shared evaluation moved on
            if (decided === undefined) {
                this.#alone();
                await this.#audience.refresh();
                const resumed = yield* this.#resume(position, signal);
                position = { epoch: position.epoch, sequence: resumed };
            }
            // publish the decided pages, then wait or move on
            else {
                const { pages, sequence } = await decided;
                await nextTask();
                for (const page of pages) {
                    published = page.position.sequence;
                    yield page;
                }

                // wait for a commit or a heartbeat
                if (sequence === position.sequence) {
                    const beat = AbortSignal.timeout(this.#feed.heartbeat);
                    await this.#evaluation.wait(sequence, AbortSignal.any([waking, beat]));
                    if (beat.aborted && !waking.aborted && sequence === published) {
                        yield { reset: false, complete: true, changes: [], position };
                    }
                }
                position = { epoch: position.epoch, sequence };

                // publish the bare position once caught up
                if (sequence >= this.#feed.sequence && sequence > published) {
                    published = sequence;
                    yield { reset: false, complete: true, changes: [], position };
                }
            }
        }
    }

    /** Send a snapshot at the latest position and what followed, returning the sequence reached. */
    async *#snapshot(epoch: string, signal: AbortSignal): AsyncGenerator<QueryPage, number> {
        // walk the queries, then catch up
        const start = { epoch, sequence: (await this.#feed.database.log.position()).sequence };
        this.#evaluation.forget(start);
        const last = yield* this.#evaluation.walk(
            new View(this.#feed.database, start, this.#feed),
            "snapshot",
        );

        return yield* this.#catchUp(start, last, signal);
    }

    /** Rebuild a resuming subscriber's state at its position, returning the sequence reached. */
    async *#resume(after: LogPosition, signal: AbortSignal): AsyncGenerator<QueryPage, number> {
        // rebuild the held rows and windows and send every group
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
    ): AsyncGenerator<QueryPage, number> {
        // rebuild the previous queries' state
        previous.forget(after);
        for await (const _page of previous.walk(
            new View(this.#feed.database, after, this.#feed),
            "rebuild",
        )) {
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

    /** Send a held page and the changes up to the latest position, returning the sequence reached. */
    async *#catchUp(
        position: LogPosition,
        held: QueryPage,
        signal: AbortSignal,
    ): AsyncGenerator<QueryPage, number> {
        // replay the changes except the last page
        const target = (await this.#feed.database.log.position()).sequence;
        let sequence = position.sequence;
        let last = held;
        while (sequence < target && !signal.aborted) {
            const pages = this.#evaluation.advance(position.epoch, sequence, false, signal);
            let next = await pages.next();
            while (!next.done) {
                // send the held page unless empty
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
}

/** Publish pages at most once per interval, merging those within it. */
export async function* coalesce(
    pages: AsyncGenerator<QueryPage>,
    every: number,
    tables: readonly Table[],
): AsyncGenerator<QueryPage> {
    // race the next page with the interval's end
    let pending: QueryPage | undefined;
    let published = Number.NEGATIVE_INFINITY;
    let next = pages.next();
    for (;;) {
        const due = published + every;
        let timer: ReturnType<typeof setTimeout> | undefined;
        const arrived =
            pending === undefined
                ? await next
                : await Promise.race([
                      next,
                      new Promise<undefined>((resolve) => {
                          timer = setTimeout(
                              () => resolve(undefined),
                              Math.max(0, due - Date.now()),
                          );
                      }),
                  ]);
        clearTimeout(timer);

        // publish the merged page at the interval's end
        if (arrived === undefined) {
            yield pending!;
            pending = undefined;
            published = Date.now();
        }
        // publish what remains at the end
        else if (arrived.done === true) {
            if (pending !== undefined) {
                yield pending;
            }

            return;
        }
        // merge each page, or publish at once after a quiet interval
        else {
            next = pages.next();
            pending = pending === undefined ? arrived.value : merge(pending, arrived.value, tables);
            if (Date.now() >= due) {
                yield pending;
                pending = undefined;
                published = Date.now();
            }
        }
    }
}

/** Merge two consecutive pages. */
function merge(earlier: QueryPage, later: QueryPage, tables: readonly Table[]): QueryPage {
    // net each row's changes, by table and key
    const changes = new Map<string, RowChange>();
    for (const change of [...earlier.changes, ...later.changes]) {
        const table = tables.find((entry) => entry[TABLE].sqlName === change.table)!;
        const key = JSON.stringify([
            change.table,
            ...table[TABLE].key.map((name) => change.row[name]),
        ]);
        const known = changes.get(key);
        const net = known === undefined ? change : netChange(known, change);
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
    const outcomes = [...(earlier.outcomes ?? []), ...(later.outcomes ?? [])];

    return {
        reset: earlier.reset,
        complete: later.complete,
        changes: [...changes.values()],
        ...(results.size === 0 ? {} : { results: [...results.values()] }),
        position: later.position,
        ...(outcomes.length === 0 ? {} : { outcomes }),
    };
}

/** Net two changes of one row. */
function netChange(earlier: RowChange, later: RowChange): RowChange | undefined {
    // forget a row that entered and left
    if (earlier.operation === "insert" && later.operation === "delete") {
        return undefined;
    }

    // keep the later row with the net operation
    const operation =
        earlier.operation === "insert" && later.operation === "update"
            ? "insert"
            : earlier.operation === "delete" && later.operation !== "delete"
              ? "update"
              : later.operation;

    return { ...later, operation };
}
