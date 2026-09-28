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

/**
 * One subscriber's queries over a feed, from its position to the head and on.
 *
 * Behind the head, a stream evaluates its queries alone, as of its position.
 * At the head, it shares the evaluation of every stream whose queries and audience decide alike.
 */
export class Stream {
    /** The feed the stream reads. */
    readonly #feed: Feed;
    /** The queries the stream follows. */
    readonly #queries: Readonly<Record<string, Query>>;
    /** Who the stream serves. */
    readonly #audience: Audience;
    /** The evaluation the stream follows: its own behind the head, or the one it shares. */
    #evaluation: Evaluation;
    /** The shortest time between two pages that follow the head, absent to publish each at once. */
    readonly #every: number | undefined;

    /** Resolve a subscriber's queries over a feed's tables for an audience. */
    constructor(
        feed: Feed,
        queries: Readonly<Record<string, Query>>,
        audience: Audience,
        every?: number,
    ) {
        // evaluate the queries alone until the stream reaches the head, publishing at most one page per interval
        this.#feed = feed;
        this.#every = every;
        this.#queries = queries;
        this.#audience = audience;
        this.#evaluation = new Evaluation(feed, queries, audience);
    }

    /**
     * Stream the queries from a position until the signal aborts.
     *
     * Without a position of the log's epoch, the stream starts with a snapshot.
     * From previous queries, it moves the subscriber's rows to these.
     * It starts over with a snapshot whenever the log no longer holds a position it needs, or a restore starts a new epoch.
     */
    async *run(
        after: LogPosition | undefined,
        signal: AbortSignal,
        previous?: Evaluation,
    ): AsyncGenerator<QueryPage> {
        let epoch = await this.#feed.database.log.epoch();
        let sequence: number | undefined;
        try {
            while (!signal.aborted) {
                try {
                    // continue within the log's epoch, rebuilding what the position held, or start over with a snapshot
                    if (sequence === undefined) {
                        sequence =
                            after?.epoch !== epoch
                                ? yield* this.#snapshot(epoch, signal)
                                : previous !== undefined
                                  ? yield* this.#reshape(previous, after, signal)
                                  : yield* this.#resume(after, signal);
                    }

                    // publish what follows, completing each page at its position, merged within the interval
                    const following = this.#follow({ epoch, sequence }, signal);
                    yield* this.#every === undefined
                        ? following
                        : coalesce(following, this.#every, this.#feed.tables);
                    return;
                } catch (error) {
                    // start over with a snapshot once the log lost a position or an epoch the stream needed, or access changed too far
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

    /** Publish the pages that follow a position, and the bare position once caught up, until the signal aborts. */
    async *#follow(start: LogPosition, signal: AbortSignal): AsyncGenerator<QueryPage> {
        let position = start;
        let published = start.sequence;
        while (!signal.aborted) {
            // share the evaluation of streams deciding alike at the position
            this.#evaluation = this.#feed.join(this.#evaluation, position);
            const decided = this.#evaluation.after(position);

            // catch up alone once the shared evaluation moved past the position, deciding as of now
            if (decided === undefined) {
                this.#alone();
                await this.#audience.refresh();
                const resumed = yield* this.#resume(position, signal);
                position = { epoch: position.epoch, sequence: resumed };
            }
            // publish the decided pages, letting other work run between streams, then wait or move on
            else {
                const { pages, sequence } = await decided;
                await nextTask();
                for (const page of pages) {
                    published = page.position.sequence;
                    yield page;
                }

                // wait for a commit, repeating the position once a heartbeat passes without one
                if (sequence === position.sequence) {
                    const beat = AbortSignal.timeout(this.#feed.heartbeat);
                    await this.#evaluation.wait(sequence, AbortSignal.any([signal, beat]));
                    if (beat.aborted && !signal.aborted && sequence === published) {
                        yield { reset: false, complete: true, changes: [], position };
                    }
                }
                position = { epoch: position.epoch, sequence };

                // publish the bare position once caught up, so that copies reach it
                if (sequence >= this.#feed.sequence && sequence > published) {
                    published = sequence;
                    yield { reset: false, complete: true, changes: [], position };
                }
            }
        }
    }

    /** Send every row and result the queries held at the log's latest position, then what followed; return the sequence reached. */
    async *#snapshot(epoch: string, signal: AbortSignal): AsyncGenerator<QueryPage, number> {
        // walk the queries as of the latest position, then catch up from it
        const start = { epoch, sequence: await this.#feed.database.log.latest() };
        this.#evaluation.forget(start);
        const last = yield* this.#evaluation.walk(
            new View(this.#feed.database, start, this.#feed),
            "snapshot",
        );

        return yield* this.#catchUp(start, last, signal);
    }

    /** Rebuild what a resuming subscriber holds as of its position, so that it continues exactly; return the sequence reached. */
    async *#resume(after: LogPosition, signal: AbortSignal): AsyncGenerator<QueryPage, number> {
        // rebuild the held rows and windows as they were at the position, and send every aggregate group as it is
        this.#evaluation.forget(after);
        const rebuilt = yield* this.#evaluation.walk(
            new View(this.#feed.database, after, this.#feed),
            "rebuild",
        );

        return yield* this.#catchUp(after, rebuilt, signal);
    }

    /** Move a subscriber from previous queries to these: catch the previous up, then send what enters and leaves. */
    async *#reshape(
        previous: Evaluation,
        after: LogPosition,
        signal: AbortSignal,
    ): AsyncGenerator<QueryPage, number> {
        // rebuild the previous queries' state as of the subscriber's position, discarding their results
        previous.forget(after);
        for await (const _page of previous.walk(
            new View(this.#feed.database, after, this.#feed),
            "rebuild",
        )) {
            // rebuild without sending
        }

        // send the previous queries' changes up to now, so that the subscriber holds them as they are
        const start = await this.#feed.database.log.latest();
        let sequence = after.sequence;
        while (sequence < start && !signal.aborted) {
            const advanced = yield* previous.advance(after.epoch, sequence, false, signal);
            if (advanced === sequence) {
                await this.#feed.next(sequence, signal);
            }
            sequence = advanced;
        }

        // hold what only these queries hold, and let go of what only the previous ones held, at one position
        const position = { epoch: after.epoch, sequence };
        const moved = await this.#evaluation.moveFrom(previous, position);

        return yield* this.#catchUp(position, moved, signal);
    }

    /** Send a held page and the changes after a position up to the log's latest, completing the last page there; return the sequence reached. */
    async *#catchUp(
        position: LogPosition,
        held: QueryPage,
        signal: AbortSignal,
    ): AsyncGenerator<QueryPage, number> {
        // replay the changes committed while the subscriber rebuilt, holding back one page to complete
        const target = await this.#feed.database.log.latest();
        let sequence = position.sequence;
        let last = held;
        while (sequence < target && !signal.aborted) {
            const pages = this.#evaluation.advance(position.epoch, sequence, false, signal);
            let next = await pages.next();
            while (!next.done) {
                // send the held page unless it carries nothing, since the next one follows it
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

/**
 * Publish pages at most once per interval: the first after a quiet interval at once, and those within it merged at its end.
 *
 * A merged page holds each row's net change and each group's last values, completing at the last page's position.
 */
export async function* coalesce(
    pages: AsyncGenerator<QueryPage>,
    every: number,
    tables: readonly Table[],
): AsyncGenerator<QueryPage> {
    // race the next page with the end of the interval while a merged page waits
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

        // publish the merged page once the interval ends
        if (arrived === undefined) {
            yield pending!;
            pending = undefined;
            published = Date.now();
        }
        // publish what remains once the pages end
        else if (arrived.done === true) {
            if (pending !== undefined) {
                yield pending;
            }

            return;
        }
        // merge each page, publishing at once after a quiet interval
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

/** Merge two pages that follow one another into one: each row's net change, each group's last values, both pages' outcomes. */
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

    // keep each group's last values, letting a query's clearing replace its earlier groups
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

/** Net two changes of one row: absent once a row entered and left, an update once it left and entered again. */
function netChange(earlier: RowChange, later: RowChange): RowChange | undefined {
    // forget a row that entered and left within the merge
    if (earlier.operation === "insert" && later.operation === "delete") {
        return undefined;
    }

    // keep the later row, entering while the earlier change entered, and changing once it left and came back
    const operation =
        earlier.operation === "insert" && later.operation === "update"
            ? "insert"
            : earlier.operation === "delete" && later.operation !== "delete"
              ? "update"
              : later.operation;

    return { ...later, operation };
}
