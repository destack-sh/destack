import { TABLE, DatabaseError, Change, Key, LogPosition, type Table } from "@destack/db";
import { nth } from "@destack/schema";
import type { Query } from "../query/query.ts";
import type { Node } from "../query/node.ts";
import type { Page } from "../query/page.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type Walk } from "../dataflow/run.ts";
import { type Decision, Patch } from "../dataflow/sink.ts";
import { View } from "../dataflow/view.ts";
import { type Audience, Watch } from "./audience.ts";
import type { Feed } from "./feed.ts";

/** The recent sequences whose decided pages an evaluation stores. */
const DECIDED_RECENT = 4;

/** The timer of a deadline the waiting streams of an evaluation share. */
interface Expiry {
    /** The time the decisions expire at. */
    readonly until: number;
    /** Aborts once the decisions expire, or another deadline replaces this one. */
    readonly expired: AbortController;
    /** Aborts the timer once no stream waits. */
    readonly disarm: AbortController;
    /** The streams waiting now. */
    waiters: number;
}

/** One set of queries evaluated for one audience over a feed, each its own dataflow. */
export class Evaluation {
    /** The feed the evaluation reads. */
    readonly #feed: Feed;
    /** Who the evaluation serves. */
    readonly #audience: Audience;
    /** The decided pages after each recent sequence. */
    readonly #decided: Map<number, Promise<Advance>>;
    /** The sequences with final decisions and their expiry. */
    readonly #idle: Map<number, number | undefined>;
    /** Each query's dataflow, by name. */
    readonly #dataflows: Map<string, Dataflow>;
    /** The rows any query's dataflow has, which the subscriber has, by key. */
    readonly #present: Set<string>;
    /** The current log position, absent before the first walk. */
    #position: LogPosition | undefined;
    /** The expiry of the decisions the waiting streams share, absent while none waits. */
    #expiry: Expiry | undefined;

    /** Compile queries for an audience over a feed. */
    constructor(feed: Feed, queries: Readonly<Record<string, Query>>, audience: Audience) {
        // start with empty state
        this.#decided = new Map<number, Promise<Advance>>();
        this.#idle = new Map<number, number | undefined>();
        this.#present = new Set<string>();

        // compile each query
        this.#feed = feed;
        this.#audience = audience;
        this.#dataflows = new Map(
            Object.entries(queries).map(([name, query]) => [name, this.#compile(name, query)]),
        );
    }

    /** Compile one query over the feed's tables, refusing a table the feed does not read. */
    #compile(name: string, query: Query): Dataflow {
        // compile the query
        const feed = this.#feed;
        const dataflow = new Dataflow(
            { [name]: query },
            {
                audience: this.#audience,
                database: feed.database,
                ...(feed.upstream === undefined ? {} : { upstream: feed.upstream }),
                changesThrough: (watches, after, through, reader) =>
                    feed.changesThrough(watches, after, through, reader),
                capacity: feed.capacity,
                pageSize: feed.pageSize,
                ...(feed.observe === undefined ? {} : { observe: feed.observe }),
            },
        );

        // require the feed's tables
        for (const node of dataflow.nodes) {
            if (!feed.tables.includes(node.table)) {
                throw DatabaseError.of(
                    "QUERY_INVALID",
                    `feed does not read ${node.table[TABLE].name}`,
                );
            }
        }

        return dataflow;
    }

    /** The name of the queries and the audience. */
    get key(): string {
        return [...this.#dataflows.values()].map((dataflow) => dataflow.key).join("\n");
    }

    /** Describe each query's dataflow. */
    inspect(): DataflowInspection[] {
        return [...this.#dataflows.values()].map((dataflow) => dataflow.inspect());
    }

    /** The current log position, absent before the first walk. */
    get position(): LogPosition | undefined {
        return this.#position;
    }

    /** Every node of the queries. */
    get nodes(): Node[] {
        return [...this.#dataflows.values()].flatMap((dataflow) => [...dataflow.nodes]);
    }

    /** List the tables and scopes the queries and the audience read. */
    watches(): Watch[] {
        return [...this.#dataflows.values()].flatMap((dataflow) => dataflow.watches());
    }

    /** List the rows the subscriber has, each key with its table. */
    rows(): Map<string, Table> {
        return new Map(
            [...this.#dataflows.values()].flatMap((dataflow) => [...dataflow.sink.rows()]),
        );
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
        } else if (this.position === undefined || !LogPosition.same(this.position, position)) {
            return undefined;
        }

        // decide the pages and store a few recent ones
        const decided = this.#pagesAfter(position);
        this.#decided.set(sequence, decided);
        this.#idle.delete(sequence);
        for (const old of [...this.#decided.keys()].slice(0, -DECIDED_RECENT)) {
            this.#decided.delete(old);
            this.#idle.delete(old);
        }
        decided.then(
            (result) => {
                // note decisions that went no further
                if (result.sequence === sequence && this.#decided.get(sequence) === decided) {
                    this.#idle.set(sequence, result.until);
                }
            },
            () => this.#decided.delete(sequence),
        );

        return decided;
    }

    /** Wait until the feed passes a change in some scopes, the decisions expire, or aborts. */
    async wait(sequence: number, signal: AbortSignal, scopes: readonly string[]): Promise<void> {
        // wait only for the feed, without expiry
        const routes = Watch.routes(this.watches(), scopes);
        const until = await this.#audience.until();
        if (until === undefined) {
            return this.#feed.next(sequence, signal, routes);
        }

        // wake the waiters of an earlier deadline, and arm one timer for this one
        if (this.#expiry?.until !== until) {
            this.#expiry?.expired.abort();
            this.#expiry?.disarm.abort();
            const { clock } = this.#feed;
            const expired = new AbortController();
            const disarm = new AbortController();
            void clock.sleep(until - clock.now(), disarm.signal).then(
                () => expired.abort(),
                () => undefined,
            );
            this.#expiry = { until, expired, disarm, waiters: 0 };
        }

        // wait beside the other streams, disarming the timer once the last one stops
        const expiry = this.#expiry;
        expiry.waiters += 1;
        try {
            await this.#feed.next(
                sequence,
                AbortSignal.any([signal, expiry.expired.signal]),
                routes,
            );
        } finally {
            expiry.waiters -= 1;
            if (expiry.waiters === 0 && this.#expiry === expiry) {
                expiry.disarm.abort();
                this.#expiry = undefined;
            }
        }
    }

    /** Move from another evaluation's state to this one's at one position. */
    async moveFrom(previous: Evaluation, position: LogPosition): Promise<Page> {
        // collect both and send the difference
        const view = this.#view(position);
        await previous.collect(view);
        const before = previous.rows();
        const after = await this.collect(view);
        const patch = Patch.difference(before, after, previous.nodes, () => false);

        return patch.page(position, this.#audience, this.#feed, { reset: false, complete: false });
    }

    /** Move to other queries at the current position. */
    async reshape(queries: Readonly<Record<string, Query>>): Promise<Page> {
        // keep the queries that stay alike, and compile the added and changed ones
        const position = this.#requirePosition();
        const kept = new Map<string, Dataflow>();
        const added = new Map<string, Dataflow>();
        for (const [name, query] of Object.entries(queries)) {
            const known = this.#dataflows.get(name);
            const compiled = this.#compile(name, query);
            if (known !== undefined && known.key === compiled.key) {
                kept.set(name, known);
            } else {
                added.set(name, compiled);
            }
        }
        const removed = [...this.#dataflows].filter(([name]) => !kept.has(name));

        // select the added queries' rows at the position
        const view = this.#view(position);
        const patches: Patch[] = [];
        for (const dataflow of added.values()) {
            dataflow.forget(position);
            const run = new Run(view, this.#audience, "snapshot");
            const batches = dataflow.hydrate(run);
            while ((await batches.next()).done !== true) {
                // select every batch
            }
            patches.push(run.patch);
        }

        // let go of the removed queries' rows and groups, then swap in the new queries
        const released = new Patch();
        for (const [, dataflow] of removed) {
            for (const [key, table] of dataflow.sink.rows()) {
                released.rows.set(key, { table, row: Key.parse(table, key), operation: "delete" });
            }
            for (const node of dataflow.nodes) {
                if (node.aggregate !== undefined) {
                    released.clear(node);
                }
            }
        }
        this.#dataflows.clear();
        for (const [name, dataflow] of [...kept, ...added]) {
            this.#dataflows.set(name, dataflow);
        }
        this.#decided.clear();
        this.#idle.clear();

        // send what entered and left the union at the position
        const patch = this.#union([released, ...patches], "snapshot");

        return patch.page(position, this.#audience, this.#feed, { reset: false, complete: false });
    }

    /** Decide the subscriber's rows again, sending the difference and the changed rows. */
    async redecide(
        position: LogPosition,
        isComplete: boolean,
        changes: readonly Change[],
    ): Promise<Page> {
        // read what the subscriber has, decide again, and collect what the new decisions allow
        const before = this.rows();
        await this.#audience.refresh();
        const after = await this.collect(this.#view(position));

        // send the difference
        const changed = new Set(
            changes.map((change) => Key.name(change.table, Change.image(change))),
        );
        const patch = Patch.difference(
            before,
            after,
            this.nodes,
            (table, key) => changed.has(key) || this.#audience.concealable(table).length > 0,
        );

        return patch.page(position, this.#audience, this.#feed, {
            reset: false,
            complete: isComplete,
        });
    }

    /** Send the pages of the changes after a position, returning the sequence read up to. */
    async *advance(
        after: LogPosition,
        isComplete: boolean,
        signal: AbortSignal,
    ): AsyncGenerator<Page, number> {
        // read from the feed or the log
        const { sequence } = after;
        const read = await this.#feed.changes(this.watches(), sequence);
        if (read === undefined) {
            throw DatabaseError.of(
                "CHANGES_COMPACTED",
                `log no longer has changes after ${sequence}`,
            );
        }

        // step each run of whole transactions
        let from = sequence;
        for (const changes of Evaluation.#chunks(read.changes, this.#feed.pageSize)) {
            if (signal.aborted) {
                break;
            }
            const position = { ...after, sequence: nth(changes, -1).sequence };
            await this.#feed.scheduler.yield();

            // decide every row again across a change of access, sending the difference
            if (await this.#decides(changes)) {
                yield await this.redecide(position, isComplete, changes);
            }
            // step each query's changed rows, sending what entered, changed and left the union
            else {
                const view = this.#view(position);
                const patches: Patch[] = [];
                for (const dataflow of this.#dataflows.values()) {
                    const run = new Run(view, this.#audience, undefined, changes);
                    await dataflow.step(run);
                    patches.push(run.patch);
                }
                const patch = this.#union(patches, undefined);
                if (patch.size > 0) {
                    yield await patch.page(position, this.#audience, this.#feed, {
                        reset: false,
                        complete: isComplete,
                    });
                }
            }
            from = position.sequence;
        }

        // take the later position of the walk and the read
        const latest = Math.max(sequence, read.sequence);
        const advanced = signal.aborted ? from : latest;
        this.#move({ ...after, sequence: advanced });

        return advanced;
    }

    /** Walk the queries as of a view, returning the last page. */
    async *walk(view: View, walk: Walk): AsyncGenerator<Page, Page> {
        // walk each query in turn, sending a snapshot's full pages as they fill
        let isFirst = walk === "snapshot";
        const pending: Patch[] = [];
        for (const dataflow of this.#dataflows.values()) {
            // clear the aggregates of a rebuilt subscriber
            const run = new Run(view, this.#audience, walk);
            pending.push(run.patch);
            if (walk === "rebuild") {
                for (const node of dataflow.nodes) {
                    if (node.aggregate !== undefined) {
                        run.patch.clear(node);
                    }
                }
            }

            // select the query's rows and results
            const batches = dataflow.hydrate(run);
            while ((await batches.next()).done !== true) {
                if (walk === "snapshot" && run.patch.size >= this.#feed.pageSize) {
                    await this.#feed.scheduler.yield();
                    const patch = this.#union(pending.splice(0), walk);
                    yield await patch.page(view.position, this.#audience, this.#feed, {
                        reset: isFirst,
                        complete: false,
                    });
                    isFirst = false;
                    run.patch.drain();
                    pending.push(run.patch);
                }
            }
        }
        this.#position = view.position;

        // send what remains as the last page, then take the rows the queries have as the subscriber's
        const patch = this.#union(pending, walk);
        this.#rememberRows();

        return await patch.page(view.position, this.#audience, this.#feed, {
            reset: isFirst,
            complete: false,
        });
    }

    /** Collect every row and result as of a view. */
    async collect(view: View): Promise<Patch> {
        // select every query's rows as of the view
        this.forget(view.position);
        const patches: Patch[] = [];
        for (const dataflow of this.#dataflows.values()) {
            const run = new Run(view, this.#audience, "collect");
            const batches = dataflow.hydrate(run);
            while ((await batches.next()).done !== true) {
                // collect every batch
            }
            patches.push(run.patch);
        }
        const patch = this.#union(patches, "collect");
        this.#rememberRows();

        return patch;
    }

    /** Take every row a query's dataflow has as one the subscriber has. */
    #rememberRows(): void {
        this.#present.clear();
        for (const key of this.rows().keys()) {
            this.#present.add(key);
        }
    }

    /** Forget everything and select nothing as of a position. */
    forget(position: LogPosition): void {
        for (const dataflow of this.#dataflows.values()) {
            dataflow.forget(position);
        }
        this.#present.clear();
        this.#position = position;
    }

    /** Join the queries' decisions into the subscriber's rows. */
    #union(patches: readonly Patch[], walk: Walk | undefined): Patch {
        // take every group as decided, as each belongs to one query
        const union = new Patch();
        for (const patch of patches) {
            for (const [key, result] of patch.results) {
                union.results.set(key, result);
            }
        }

        // take each row's latest present image, or its key once a query let it go
        const decided = new Map<string, Decision>();
        for (const patch of patches) {
            for (const [key, decision] of patch.rows) {
                const known = decided.get(key);
                if (known === undefined || decision.operation !== "delete") {
                    decided.set(key, decision);
                }
            }
        }

        // send a row entering the union, changing in it or leaving it
        for (const [key, decision] of decided) {
            const isBefore = this.#present.has(key);
            const isAfter = [...this.#dataflows.values()].some((dataflow) =>
                dataflow.sink.has(key),
            );
            if (isAfter && !isBefore) {
                this.#present.add(key);
                union.rows.set(key, { ...decision, operation: "insert" });
            } else if (isAfter && decision.operation !== "delete" && walk === undefined) {
                union.rows.set(key, { ...decision, operation: "update" });
            } else if (!isAfter && isBefore) {
                this.#present.delete(key);
                union.rows.set(key, {
                    table: decision.table,
                    row: Key.parse(decision.table, key),
                    operation: "delete",
                });
            }
        }

        return union;
    }

    /** Move every query's dataflow to a later position of the same epoch. */
    #move(position: LogPosition): void {
        for (const dataflow of this.#dataflows.values()) {
            dataflow.advance(position);
        }
        this.#position = position;
    }

    /** Read the current position, refusing an evaluation that never walked. */
    #requirePosition(): LogPosition {
        const position = this.#position;
        if (position === undefined) {
            throw new TypeError("an evaluation moves only once it walked");
        }

        return position;
    }

    /** Decide the pages after the current position, and a refresh once decisions expire. */
    async #pagesAfter(position: LogPosition): Promise<Advance> {
        // decide what follows
        const pages: Page[] = [];
        const advancing = this.advance(position, true, new AbortController().signal);
        let next = await advancing.next();
        while (next.done !== true) {
            pages.push(next.value);
            next = await advancing.next();
        }

        // refresh once the decisions expired
        const sequence = next.value;
        const until = await this.#audience.until();
        if (until !== undefined && until <= this.#feed.clock.now()) {
            pages.push(await this.redecide({ ...position, sequence }, true, []));
        }

        // move the last page to the sequence the pages arrive at, past commits outside the queries
        const last = pages.at(-1);
        if (last !== undefined && last.position.sequence < sequence) {
            pages[pages.length - 1] = { ...last, position: { ...last.position, sequence } };
        }

        return { pages, sequence, until: await this.#audience.until() };
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

    /** Split changes into runs of about a page that end with whole transactions. */
    static #chunks(changes: readonly Change[], pageSize: number): Change[][] {
        // close a run when it has a page and a transaction ends
        const runs: Change[][] = [];
        let run: Change[] = [];
        for (const [index, change] of changes.entries()) {
            run.push(change);
            const next = changes[index + 1];
            if (
                next === undefined ||
                (run.length >= pageSize && !Evaluation.#isSameTransaction(change, next))
            ) {
                runs.push(run);
                run = [];
            }
        }

        return runs;
    }

    /** Whether two changes belong to one transaction. */
    static #isSameTransaction(change: Change, next: Change): boolean {
        return change.transaction !== null && change.transaction === next.transaction;
    }
}

/** The pages decided after a sequence. */
export interface Advance {
    /** The pages, complete at their positions. */
    readonly pages: readonly Page[];
    /** The sequence the pages arrive at. */
    readonly sequence: number;
    /** When the audience's decisions expire. */
    readonly until: number | undefined;
}
