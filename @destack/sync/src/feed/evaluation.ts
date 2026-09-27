import { TABLE } from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import type { Change, LogPosition } from "@destack/db/log";
import type { Query } from "../query/query.ts";
import type { QueryPage } from "../query/page.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type Walk } from "../dataflow/run.ts";
import { PAGE_ROWS } from "../dataflow/selection.ts";
import { Patch } from "../dataflow/sink.ts";
import { View } from "../dataflow/view.ts";
import type { Audience } from "./audience.ts";
import type { Feed } from "./feed.ts";

/** The recent sequences whose decided pages an evaluation keeps for streams a step behind. */
const DECIDED_RECENT = 4;

/** The longest delay a timer holds, 2^31 - 1 milliseconds; a later moment waits again. */
const LONGEST_DELAY = 2_147_483_647;

/**
 * One set of queries evaluated for one audience over a feed, published page by page.
 *
 * Its dataflow keeps what each node holds as of the evaluation's position, and each run of committed changes applies to it as a delta.
 * Streams whose queries and audience decide alike share one evaluation at the head of the feed, and each of its pages.
 */
export class Evaluation {
    /** The feed the evaluation reads. */
    readonly #feed: Feed;
    /** Who the evaluation serves, deciding alike for every stream sharing it. */
    readonly #audience: Audience;
    /** The pages after each recent sequence, decided once for every stream sharing the evaluation. */
    readonly #decided = new Map<number, Promise<Advance>>();
    /** The sequences whose decisions reached no further, which hold until the feed passes them or the moment their decisions held until passes. */
    readonly #idle = new Map<number, number | undefined>();
    /** The queries' dataflow. */
    readonly #dataflow: Dataflow;

    /** Compile queries over a feed's tables for an audience. */
    constructor(feed: Feed, queries: Readonly<Record<string, Query>>, audience: Audience) {
        // compile the queries, naming them canonically
        this.#feed = feed;
        this.#audience = audience;
        this.#dataflow = new Dataflow(queries, {
            audience,
            database: feed.database,
            ...(feed.upstream === undefined ? {} : { upstream: feed.upstream }),
            changesThrough: (watches, after, through) =>
                feed.changesThrough(watches, after, through),
            capacity: feed.capacity,
            ...(feed.observe === undefined ? {} : { observe: feed.observe }),
        });

        // require the feed's tables
        for (const node of this.#dataflow.nodes) {
            if (!feed.tables.includes(node.table)) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `feed does not read ${node.table[TABLE].name}`,
                );
            }
        }
    }

    /** The name of the queries and of everything the audience decides, equal for evaluations that decide alike. */
    get key(): string {
        return this.#dataflow.key;
    }

    /** Describe the evaluation's dataflow for inspection. */
    inspect(): DataflowInspection {
        return this.#dataflow.inspect();
    }

    /** The log position the dataflow holds, absent before the first walk. */
    get position(): LogPosition | undefined {
        return this.#dataflow.position;
    }

    /**
     * Decide the pages after the position the evaluation holds, once for every stream sharing it; absent for another position.
     *
     * The pages complete at their positions, followed by a refresh once the audience's decisions expired.
     */
    after(position: LogPosition): Promise<Advance> | undefined {
        // reuse the pages decided after the position, unless they reached no further and the feed moved on since
        const sequence = position.sequence;
        const known = this.#decided.get(sequence);
        const until = this.#idle.get(sequence);
        const isStale =
            this.#idle.has(sequence) &&
            (this.#feed.sequence > sequence || (until !== undefined && until <= Date.now()));
        if (known !== undefined && !isStale) {
            return known;
        } else if (!isSame(this.position, position)) {
            return undefined;
        }

        // decide the pages after it, keeping a few recent decisions for streams a step behind
        const decided = this.#pagesAfter(position);
        this.#decided.set(sequence, decided);
        this.#idle.delete(sequence);
        for (const old of [...this.#decided.keys()].slice(0, -DECIDED_RECENT)) {
            this.#decided.delete(old);
            this.#idle.delete(old);
        }
        decided.then(
            (result) => {
                // note decisions that reached no further, which a later write replaces
                if (result.sequence === sequence && this.#decided.get(sequence) === decided) {
                    this.#idle.set(sequence, result.until);
                }
            },
            () => this.#decided.delete(sequence),
        );

        return decided;
    }

    /** Wait until the feed passes a sequence, or the audience's decisions expire, or the signal aborts. */
    async wait(sequence: number, signal: AbortSignal): Promise<void> {
        // wait for the feed alone while decisions never expire
        const until = await this.#audience.until();
        if (until === undefined) {
            return this.#feed.next(sequence, signal);
        }

        // wake at that moment too
        const expired = new AbortController();
        const timer = setTimeout(
            () => expired.abort(),
            Math.min(until - Date.now(), LONGEST_DELAY),
        );
        try {
            await this.#feed.next(sequence, AbortSignal.any([signal, expired.signal]));
        } finally {
            clearTimeout(timer);
        }
    }

    /** Move what another evaluation holds to what this one holds at one position, as a page that does not complete. */
    async moveFrom(previous: Evaluation, position: LogPosition): Promise<QueryPage> {
        // collect both at the position and send the difference
        const view = this.#view(position);
        const held = await previous.collect(view);
        const holding = await this.collect(view);
        const patch = Patch.difference(held, holding, previous.#dataflow.nodes, () => false);

        return patch.page(position, this.#audience, this.#feed, { reset: false, complete: false });
    }

    /** Decide what the subscriber holds again as of now at a position it holds, sending what the audience's refreshed decisions change. */
    async refresh(position: LogPosition): Promise<QueryPage> {
        // collect what the subscriber holds, then what it holds once the audience decides as of now
        const view = this.#view(position);
        const held = await this.collect(view);
        await this.#audience.refresh();
        const holding = await this.collect(view);

        // send the difference, and every held row whose readable columns may have changed
        const patch = Patch.difference(
            held,
            holding,
            this.#dataflow.nodes,
            (table) => this.#audience.concealable(table).length > 0,
        );

        return patch.page(position, this.#audience, this.#feed, { reset: false, complete: true });
    }

    /** Send the pages the changes after a sequence make, completing each when live; return the sequence read up to. */
    async *advance(
        epoch: string,
        sequence: number,
        isComplete: boolean,
        signal: AbortSignal,
    ): AsyncGenerator<QueryPage, number> {
        // read what follows from the feed, or from the log when it is older
        const read = await this.#feed.changes(this.#dataflow.watches(), sequence);
        if (read === undefined) {
            throw new DatabaseError(
                "CHANGES_COMPACTED",
                `log no longer holds changes after ${sequence}`,
            );
        }

        // step each run of whole transactions as of its last one, publishing what it changes
        let from = sequence;
        for (const changes of chunks(read.changes)) {
            if (signal.aborted) {
                break;
            }
            const position = { epoch, sequence: changes.at(-1)!.sequence };
            await nextTask();
            const run = new Run(this.#view(position), this.#audience, undefined, changes);
            const isFollowed = await this.#dataflow.step(run);
            from = position.sequence;
            if (!isFollowed) {
                throw new Restart("access changed beyond what a page follows");
            } else if (run.patch.size > 0) {
                yield await run.patch.page(position, this.#audience, this.#feed, {
                    reset: false,
                    complete: isComplete,
                });
            }
        }

        // hold the position decided up to, all of the read unless aborted
        const reached = Math.max(sequence, read.sequence);
        const held = signal.aborted ? from : reached;
        this.#dataflow.advance({ epoch, sequence: held });

        return held;
    }

    /** Walk the queries as of a view, holding what they hold there and sending what the walk sends; return the last patch's page. */
    async *walk(view: View, walk: Walk): AsyncGenerator<QueryPage, QueryPage> {
        // clear what a rebuilt subscriber held of every aggregate, whose groups follow
        const run = new Run(view, this.#audience, walk);
        let isFirst = walk === "snapshot";
        if (walk === "rebuild") {
            for (const node of this.#dataflow.nodes) {
                if (node.aggregate !== undefined) {
                    run.patch.clear(node);
                }
            }
        }

        // hold each root's rows and results, sending full pages of a snapshot as they fill
        for await (const _batch of this.#dataflow.hydrate(run)) {
            if (walk === "snapshot" && run.patch.size >= PAGE_ROWS) {
                await nextTask();
                yield await run.patch.page(view.position, this.#audience, this.#feed, {
                    reset: isFirst,
                    complete: false,
                });
                isFirst = false;
                run.patch.drain();
            }
        }

        // hold the walked position, within the capacity

        return await run.patch.page(view.position, this.#audience, this.#feed, {
            reset: isFirst,
            complete: false,
        });
    }

    /** Collect every row and result the queries hold as of a view, holding them there. */
    async collect(view: View): Promise<Patch> {
        // hold nothing, then every root's rows as of the view
        this.forget(view.position);
        const run = new Run(view, this.#audience, "collect");
        for await (const _batch of this.#dataflow.hydrate(run)) {
            // collect every batch
        }

        return run.patch;
    }

    /** Forget everything the evaluation holds, holding nothing as of a position until a walk holds it again. */
    forget(position: LogPosition): void {
        this.#dataflow.forget(position);
    }

    /** Decide the pages after the position the evaluation holds, and a refresh once the audience's decisions expired. */
    async #pagesAfter(position: LogPosition): Promise<Advance> {
        // decide what follows, completing each page
        const pages: QueryPage[] = [];
        const advancing = this.advance(
            position.epoch,
            position.sequence,
            true,
            new AbortController().signal,
        );
        let next = await advancing.next();
        while (!next.done) {
            pages.push(next.value);
            next = await advancing.next();
        }

        // decide again as of now once the audience's decisions expired
        const until = await this.#audience.until();
        if (until !== undefined && until <= Date.now()) {
            pages.push(await this.refresh({ epoch: position.epoch, sequence: next.value }));
        }

        return { pages, sequence: next.value, until: await this.#audience.until() };
    }

    /** Show the feed's database as of a position. */
    #view(position: LogPosition): View {
        return new View(this.#feed.database, position, this.#feed);
    }
}

/** The pages decided after a sequence, the sequence they reach, and when the decisions expire. */
export interface Advance {
    /** The pages, each complete at its position. */
    readonly pages: readonly QueryPage[];
    /** The sequence the pages reach. */
    readonly sequence: number;
    /** When the audience's decisions expire, absent when they never do. */
    readonly until: number | undefined;
}

/** A reason a stream starts over with a snapshot: access changed beyond what its pages follow. */
export class Restart extends Error {}

/** Let other work run before deciding more, since decisions on shared reads continue without waiting on input or output. */
export function nextTask(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

/** Split changes into runs of about a page, each ending with a whole transaction. */
function chunks(changes: readonly Change[]): Change[][] {
    // close a run once it holds a page and the next change starts another transaction
    const runs: Change[][] = [];
    let run: Change[] = [];
    for (const [index, change] of changes.entries()) {
        run.push(change);
        const next = changes[index + 1];
        if (next === undefined || (run.length >= PAGE_ROWS && !isSameTransaction(change, next))) {
            runs.push(run);
            run = [];
        }
    }

    return runs;
}

/** Whether two changes belong to one transaction, which a page never splits. */
function isSameTransaction(change: Change, next: Change): boolean {
    return change.transaction !== null && change.transaction === next.transaction;
}

/** Whether two positions are the same, absent ones never. */
function isSame(left: LogPosition | undefined, right: LogPosition): boolean {
    return left !== undefined && left.epoch === right.epoch && left.sequence === right.sequence;
}
