import { TABLE, DatabaseError, type Change, type LogPosition } from "@destack/db";
import { aligned } from "@destack/schema";
import type { Query } from "../query/query.ts";
import type { Page } from "../query/page.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type Walk } from "../dataflow/run.ts";
import { PAGE_ROWS } from "../dataflow/selection.ts";
import { Patch } from "../dataflow/sink.ts";
import { View } from "../dataflow/view.ts";
import type { Audience } from "./audience.ts";
import type { Feed } from "./feed.ts";

/** The recent sequences whose decided pages an evaluation keeps. */
const DECIDED_RECENT = 4;

/** The longest timer delay, 2^31 - 1 milliseconds. */
const LONGEST_DELAY = 2_147_483_647;

/** One set of queries evaluated for one audience over a feed. */
export class Evaluation {
    /** The feed the evaluation reads. */
    readonly #feed: Feed;
    /** Who the evaluation serves. */
    readonly #audience: Audience;
    /** The decided pages after each recent sequence. */
    readonly #decided = new Map<number, Promise<Advance>>();
    /** The sequences with final decisions and their expiry. */
    readonly #idle = new Map<number, number | undefined>();
    /** The queries' dataflow. */
    readonly #dataflow: Dataflow;

    /** Compile queries for an audience over a feed. */
    constructor(feed: Feed, queries: Readonly<Record<string, Query>>, audience: Audience) {
        // compile the queries
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

    /** The name of the queries and the audience. */
    get key(): string {
        return this.#dataflow.key;
    }

    /** Describe the dataflow. */
    inspect(): DataflowInspection {
        return this.#dataflow.inspect();
    }

    /** The current log position, absent before the first walk. */
    get position(): LogPosition | undefined {
        return this.#dataflow.position;
    }

    /** Decide the pages after the current position, absent for another position. */
    after(position: LogPosition): Promise<Advance> | undefined {
        // reuse the decided pages unless stale
        const sequence = position.sequence;
        const known = this.#decided.get(sequence);
        const until = this.#idle.get(sequence);
        const isStale =
            this.#idle.has(sequence) &&
            (this.#feed.sequence > sequence ||
                (until !== undefined && until <= this.#feed.clock.now()));
        if (known !== undefined && !isStale) {
            return known;
        } else if (!isSame(this.position, position)) {
            return undefined;
        }

        // decide the pages and keep a few recent ones
        const decided = this.#pagesAfter(position);
        this.#decided.set(sequence, decided);
        this.#idle.delete(sequence);
        for (const old of [...this.#decided.keys()].slice(0, -DECIDED_RECENT)) {
            this.#decided.delete(old);
            this.#idle.delete(old);
        }
        decided.then(
            (result) => {
                // note decisions that reached no further
                if (result.sequence === sequence && this.#decided.get(sequence) === decided) {
                    this.#idle.set(sequence, result.until);
                }
            },
            () => this.#decided.delete(sequence),
        );

        return decided;
    }

    /** Wait until the feed passes a sequence, the decisions expire, or the signal aborts. */
    async wait(sequence: number, signal: AbortSignal): Promise<void> {
        // wait for the feed alone without expiry
        const until = await this.#audience.until();
        if (until === undefined) {
            return this.#feed.next(sequence, signal);
        }

        // wake at the expiry too
        const expired = new AbortController();
        const timer = setTimeout(
            () => expired.abort(),
            Math.min(until - this.#feed.clock.now(), LONGEST_DELAY),
        );
        try {
            await this.#feed.next(sequence, AbortSignal.any([signal, expired.signal]));
        } finally {
            clearTimeout(timer);
        }
    }

    /** Move from another evaluation's state to this one's at one position. */
    async moveFrom(previous: Evaluation, position: LogPosition): Promise<Page> {
        // collect both and send the difference
        const view = this.#view(position);
        await previous.collect(view);
        const before = previous.#dataflow.sink.rows();
        const after = await this.collect(view);
        const patch = Patch.difference(before, after, previous.#dataflow.nodes, () => false);

        return patch.page(position, this.#audience, this.#feed, { reset: false, complete: false });
    }

    /** Decide the subscriber's rows again at a position, sending the difference to what it has. */
    async redecide(position: LogPosition, isComplete: boolean): Promise<Page> {
        // keep what the subscriber has, decide again, and collect what the new decisions admit
        const before = this.#dataflow.sink.rows();
        await this.#audience.refresh();
        const after = await this.collect(this.#view(position));

        // send the difference and rows with possibly changed readable columns
        const patch = Patch.difference(
            before,
            after,
            this.#dataflow.nodes,
            (table) => this.#audience.concealable(table).length > 0,
        );

        return patch.page(position, this.#audience, this.#feed, {
            reset: false,
            complete: isComplete,
        });
    }

    /** Send the pages of the changes after a sequence, returning the sequence read up to. */
    async *advance(
        epoch: string,
        sequence: number,
        isComplete: boolean,
        signal: AbortSignal,
    ): AsyncGenerator<Page, number> {
        // read from the feed or the log
        const read = await this.#feed.changes(this.#dataflow.watches(), sequence);
        if (read === undefined) {
            throw new DatabaseError(
                "CHANGES_COMPACTED",
                `log no longer has changes after ${sequence}`,
            );
        }

        // step each run of whole transactions
        let from = sequence;
        for (const changes of chunks(read.changes)) {
            if (signal.aborted) {
                break;
            }
            const position = { epoch, sequence: aligned(changes, changes.length - 1).sequence };
            await nextTask();

            // decide every row again across a change of access, sending the difference
            if (await this.#decides(changes)) {
                yield await this.redecide(position, isComplete);
            }
            // step the changed rows
            else {
                const run = new Run(this.#view(position), this.#audience, undefined, changes);
                await this.#dataflow.step(run);
                if (run.patch.size > 0) {
                    yield await run.patch.page(position, this.#audience, this.#feed, {
                        reset: false,
                        complete: isComplete,
                    });
                }
            }
            from = position.sequence;
        }

        // keep the reached position
        const reached = Math.max(sequence, read.sequence);
        const advanced = signal.aborted ? from : reached;
        this.#dataflow.advance({ epoch, sequence: advanced });

        return advanced;
    }

    /** Walk the queries as of a view, returning the last page. */
    async *walk(view: View, walk: Walk): AsyncGenerator<Page, Page> {
        // clear the aggregates of a rebuilt subscriber
        const run = new Run(view, this.#audience, walk);
        let isFirst = walk === "snapshot";
        if (walk === "rebuild") {
            for (const node of this.#dataflow.nodes) {
                if (node.aggregate !== undefined) {
                    run.patch.clear(node);
                }
            }
        }

        // select each root's rows and results
        const batches = this.#dataflow.hydrate(run);
        while ((await batches.next()).done !== true) {
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

        // keep the walked position within the capacity

        return await run.patch.page(view.position, this.#audience, this.#feed, {
            reset: isFirst,
            complete: false,
        });
    }

    /** Collect every row and result as of a view. */
    async collect(view: View): Promise<Patch> {
        // select every root's rows as of the view
        this.forget(view.position);
        const run = new Run(view, this.#audience, "collect");
        const batches = this.#dataflow.hydrate(run);
        while ((await batches.next()).done !== true) {
            // collect every batch
        }

        return run.patch;
    }

    /** Forget everything and select nothing as of a position. */
    forget(position: LogPosition): void {
        this.#dataflow.forget(position);
    }

    /** Decide the pages after the current position, and a refresh once decisions expire. */
    async #pagesAfter(position: LogPosition): Promise<Advance> {
        // decide what follows
        const pages: Page[] = [];
        const advancing = this.advance(
            position.epoch,
            position.sequence,
            true,
            new AbortController().signal,
        );
        let next = await advancing.next();
        while (next.done !== true) {
            pages.push(next.value);
            next = await advancing.next();
        }

        // refresh once the decisions expired
        const until = await this.#audience.until();
        if (until !== undefined && until <= this.#feed.clock.now()) {
            const reached = { epoch: position.epoch, sequence: next.value };
            pages.push(await this.redecide(reached, true));
        }

        return { pages, sequence: next.value, until: await this.#audience.until() };
    }

    /** Report whether any of some changes may decide the visibility of other rows. */
    async #decides(changes: readonly Change[]): Promise<boolean> {
        for (const change of changes) {
            if (await this.#audience.decides(change)) {
                return true;
            }
        }

        return false;
    }

    /** Show the feed's database as of a position. */
    #view(position: LogPosition): View {
        return new View(this.#feed.database, position, this.#feed);
    }
}

/** The pages decided after a sequence. */
export interface Advance {
    /** The pages, complete at their positions. */
    readonly pages: readonly Page[];
    /** The sequence the pages reach. */
    readonly sequence: number;
    /** When the audience's decisions expire. */
    readonly until: number | undefined;
}

/** Let other work run before deciding more. */
export function nextTask(): Promise<void> {
    return new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

/** Split changes into runs of about a page that end with whole transactions. */
function chunks(changes: readonly Change[]): Change[][] {
    // close a run when it has a page and a transaction ends
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

/** Whether two changes belong to one transaction. */
function isSameTransaction(change: Change, next: Change): boolean {
    return change.transaction !== null && change.transaction === next.transaction;
}

/** Whether two positions are the same. */
function isSame(left: LogPosition | undefined, right: LogPosition): boolean {
    return left !== undefined && left.epoch === right.epoch && left.sequence === right.sequence;
}
