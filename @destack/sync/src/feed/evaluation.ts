import { Telemetry } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import {
    TABLE,
    DatabaseError,
    Change,
    type Condition,
    Key,
    LogPosition,
    type Row,
    type Table,
} from "@destack/db";
import { nth, present } from "@destack/schema";
import type { Query } from "../query/query.ts";
import type { Node } from "../query/node.ts";
import type { Page } from "../query/page.ts";
import { Dataflow, type DataflowInspection } from "../dataflow/dataflow.ts";
import { Run, type Walk } from "../dataflow/run.ts";
import { type Decision, Patch, ViewRecord } from "../dataflow/sink.ts";
import { View } from "../dataflow/view.ts";
import { type Affected, ALL_ROWS, type Audience, NO_ROWS, Watch } from "./audience.ts";
import type { Feed } from "./feed.ts";

/** The recent advances an evaluation stores for the streams that share it. */
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
    /** The evaluation's spans, on the feed's telemetry. */
    readonly #scope: TelemetryScope;
    /** Who the evaluation serves. */
    readonly #audience: Audience;
    /** The images of the rows the subscriber holds, by which pages leave out unchanged rows. */
    readonly #record = new ViewRecord();
    /** The recent advances in the order decided, each a stream takes once. */
    #advances: {
        readonly index: number;
        readonly from: number;
        readonly advance: Promise<Advance>;
    }[];
    /** The last advance that went no further, while it stands: its order, sequence and expiry. */
    #idle: Pick<Advance, "index" | "from" | "until"> | undefined;
    /** The order of the last advance decided. */
    #index = 0;
    /** Each query's dataflow, by name. */
    readonly #dataflows: Map<string, Dataflow>;
    /** The rows any query's dataflow has, which the subscriber has, by key. */
    readonly #present: Set<string>;
    /** The current log position, absent before the first walk. */
    #position: LogPosition | undefined;
    /** The expiry of the decisions the waiting streams share, absent while none waits. */
    #expiry: Expiry | undefined;
    /** Whether the subscriber's rows stand at the position unwalked, until a change needs them. */
    #isDeferred = false;

    /** Compile queries for an audience over a feed. */
    constructor(feed: Feed, queries: Readonly<Record<string, Query>>, audience: Audience) {
        // start with empty state
        this.#advances = [];
        this.#idle = undefined;
        this.#present = new Set<string>();

        // compile each query
        this.#feed = feed;
        this.#scope = TelemetryScope.of(
            import.meta.destack.package,
            feed.database.state.environment.telemetry,
        );
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
            },
        );

        // require the feed's tables
        for (const node of dataflow.nodes) {
            if (!feed.tables.includes(node.table)) {
                throw DatabaseError.of(
                    "QUERY_INVALID",
                    `table '${node.table[TABLE].name}' is invalid: the feed does not read it`,
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

    /** Take the advance after a stream's last one from its position, absent elsewhere. */
    async next(index: number, position: LogPosition): Promise<Advance | undefined> {
        // take the first advance from the position the stream has not taken
        const sequence = position.sequence;
        const known = this.#advances.find((each) => each.index > index && each.from === sequence);
        if (known !== undefined) {
            return await known.advance;
        }

        // decide only at the evaluation's own position
        if (this.position === undefined || !LogPosition.same(this.position, position)) {
            return undefined;
        }

        // answer nothing to the stream holding the last advance while it stands: no commit since, unexpired
        const idle = this.#idle;
        const now = this.#feed.clock.now();
        if (
            idle?.index === index &&
            idle.from === sequence &&
            this.#feed.sequence === sequence &&
            (idle.until === undefined || idle.until > now)
        ) {
            return { index, from: sequence, pages: [], sequence, until: idle.until };
        }

        return await this.#decide(position);
    }

    /** Decide the next advance from a position, keeping a few recent ones for sharing streams. */
    #decide(position: LogPosition): Promise<Advance> {
        // decide the pages in order
        this.#index += 1;
        const decided = {
            index: this.#index,
            from: position.sequence,
            advance: this.#pagesAfter(position, this.#index),
        };
        this.#advances = [...this.#advances, decided].slice(-DECIDED_RECENT);
        this.#idle = undefined;

        // note the last advance once it went no further, and forget a failed one
        decided.advance.then(
            (advance) => {
                if (advance.sequence === advance.from && this.#advances.at(-1) === decided) {
                    this.#idle = advance;
                }
            },
            () => {
                this.#advances = this.#advances.filter((each) => each !== decided);
            },
        );

        return decided.advance;
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

        return patch.page(position, this.#audience, this.#feed, this.#record, {
            reset: false,
            complete: false,
        });
    }

    /** Move to other queries at the current position. */
    async reshape(queries: Readonly<Record<string, Query>>): Promise<Page> {
        // keep the queries that stay alike, and compile the added and changed ones
        const position = this.#requirePosition();
        await this.#restoreRows(position);
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
            const batches = await dataflow.hydrate(run);
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
        this.#advances = [];
        this.#idle = undefined;

        // send what entered and left the union at the position
        const patch = this.#union([released, ...patches], "snapshot");

        return patch.page(position, this.#audience, this.#feed, this.#record, {
            reset: false,
            complete: false,
        });
    }

    /** Decide the subscriber's rows again after some changes, sending the difference. */
    async redecide(
        previous: LogPosition,
        position: LogPosition,
        isComplete: boolean,
        changes: readonly Change[],
    ): Promise<Page> {
        // rebuild the subscriber's rows before the changes, then collect them under the new access
        await this.#restoreRows(previous);
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

        return patch.page(position, this.#audience, this.#feed, this.#record, {
            reset: false,
            complete: isComplete,
        });
    }

    /** Send the pages of the changes after a position, in a span of its own. */
    advance(
        after: LogPosition,
        isComplete: boolean,
        signal: AbortSignal,
    ): Promise<AsyncGenerator<Page, number>> {
        return this.#turn("destack.evaluation.advance", () =>
            this.#advance(after, isComplete, signal),
        );
    }

    /** Send the pages of the changes after a position, returning the sequence read up to. */
    async *#advance(
        after: LogPosition,
        isComplete: boolean,
        signal: AbortSignal,
    ): AsyncGenerator<Page, number> {
        // read from the feed or the log
        const { sequence } = after;
        const read = await this.#feed.changes(this.watches(), sequence);
        if (read === undefined) {
            throw DatabaseError.of(
                "CHANGES_NOT_FOUND",
                `changes after ${sequence} in the log not found`,
            );
        }

        // walk deferred rows before their first changes, unless the queries decide each row by itself
        if (read.changes.length > 0 && !this.isRowwise) {
            await this.#walkDeferred(after);
        }

        // step each run of whole transactions
        let from = sequence;
        for (const changes of Evaluation.#chunks(read.changes, this.#feed.pageSize)) {
            if (signal.aborted) {
                break;
            }
            const position = { ...after, sequence: nth(changes, -1).sequence };
            await this.#feed.scheduler.yield();

            // decide every row again when an access change affects them all
            const previous = { ...after, sequence: from };
            const affected = await this.#affected(changes);
            if (affected.kind === "all" || (affected.kind === "rows" && !this.isRowwise)) {
                yield await this.redecide(previous, position, isComplete, changes);
            }
            // decide the changed and affected rows by their images
            else if (this.isRowwise) {
                const patch = await this.#decideImages(previous, position, changes, affected);
                if (patch.size > 0) {
                    yield await patch.page(position, this.#audience, this.#feed, this.#record, {
                        reset: false,
                        complete: isComplete,
                    });
                }
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
                    yield await patch.page(position, this.#audience, this.#feed, this.#record, {
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

    /** Walk the queries as of a view in its own span, returning the last page. */
    walk(view: View, walk: Walk): Promise<AsyncGenerator<Page, Page>> {
        return this.#turn("destack.evaluation.walk", () => this.#walk(view, walk));
    }

    /** Run one turn of the evaluation in a span of its own, linked to the span asking for it. */
    #turn<Pages extends AsyncGenerator<Page, unknown>>(
        name: string,
        pages: () => Pages,
    ): Promise<Pages> {
        const { telemetry } = this.#feed.database.state.environment;

        return Telemetry.detach(telemetry, (links) =>
            this.#scope.span(name, {}, async () => pages(), { links }),
        );
    }

    /** Walk the queries as of a view, returning the last page. */
    async *#walk(view: View, walk: Walk): AsyncGenerator<Page, Page> {
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
            const batches = await dataflow.hydrate(run);
            while ((await batches.next()).done !== true) {
                if (walk === "snapshot" && run.patch.size >= this.#feed.pageSize) {
                    await this.#feed.scheduler.yield();
                    const patch = this.#union(pending.splice(0), walk);
                    yield await patch.page(
                        view.position,
                        this.#audience,
                        this.#feed,
                        this.#record,
                        {
                            reset: isFirst,
                            complete: false,
                        },
                    );
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

        return await patch.page(view.position, this.#audience, this.#feed, this.#record, {
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
            const batches = await dataflow.hydrate(run);
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
        // empty each dataflow at the position
        for (const dataflow of this.#dataflows.values()) {
            dataflow.forget(position);
        }
        this.#present.clear();
        this.#position = position;
        this.#isDeferred = false;
    }

    /** Take the subscriber's rows as standing at a position, walked once a change needs them. */
    defer(position: LogPosition): void {
        this.forget(position);
        this.#isDeferred = true;
    }

    /** Whether every query decides each row by itself, holding no rows between changes. */
    get isRowwise(): boolean {
        return [...this.#dataflows.values()].every((dataflow) => dataflow.isRowwise);
    }

    /** Rebuild the rows the subscriber has at a position. */
    async #restoreRows(position: LogPosition): Promise<void> {
        if (this.isRowwise) {
            await this.collect(this.#view(position));
        } else {
            await this.#walkDeferred(position);
        }
    }

    /** Decide each changed or affected row by its first and last image. */
    async #decideImages(
        previous: LogPosition,
        position: LogPosition,
        changes: readonly Change[],
        affected: Affected,
    ): Promise<Patch> {
        // take each changed row's first and last image
        const images = new Map<string, Image>();
        for (const change of changes) {
            const key = Key.name(change.table, change.key);
            const known = images.get(key);
            images.set(key, {
                table: change.table,
                before: known === undefined ? Change.before(change) : known.before,
                after: Change.after(change),
            });
        }

        // add the rows an access change affects, as they stand before the changes and after them
        const changed = new Set(images.keys());
        if (affected.kind === "rows") {
            for (const [table, where] of await affected.read([previous, position])) {
                const [before, after] = await Promise.all([
                    this.#view(previous).matching(table, where),
                    this.#view(position).matching(table, where),
                ]);
                for (const [side, rows] of [
                    ["before", before],
                    ["after", after],
                ] as const) {
                    for (const row of rows) {
                        const key = Key.name(table, row);
                        if (!changed.has(key)) {
                            const image = images.get(key) ?? { table, before: null, after: null };
                            images.set(key, { ...image, [side]: row });
                        }
                    }
                }
            }
        }

        // decide the images before the changes, then refresh the access and decide them after
        const was = await this.#selected(previous, images, "before");
        if (affected.kind === "rows") {
            await this.#audience.refresh();
        }
        const is = await this.#selected(position, images, "after");

        // send the rows entering and leaving, and the changed rows staying
        const patch = new Patch();
        for (const [key, { table, after }] of images) {
            if (after !== null && is.has(key) && !was.has(key)) {
                patch.rows.set(key, { table, row: after, operation: "insert" });
            } else if (after !== null && is.has(key) && changed.has(key)) {
                patch.rows.set(key, { table, row: after, operation: "update" });
            } else if (was.has(key) && !is.has(key)) {
                patch.rows.set(key, { table, row: Key.parse(table, key), operation: "delete" });
            }
        }

        return patch;
    }

    /** List the keys of the images on one side the queries have at a position. */
    async #selected(
        position: LogPosition,
        images: ReadonlyMap<string, Image>,
        side: "before" | "after",
    ): Promise<Set<string>> {
        // group the side's images by table
        const tables = new Map<Table, Map<Row, string>>();
        for (const [key, image] of images) {
            const row = image[side];
            if (row !== null) {
                const rows = tables.get(image.table) ?? new Map<Row, string>();
                tables.set(image.table, rows.set(row, key));
            }
        }

        // keep each row a query has, deciding its visibility as of the position
        const run = new Run(this.#view(position), this.#audience, undefined);
        const selected = new Set<string>();
        for (const [table, rows] of tables) {
            for (const dataflow of this.#dataflows.values()) {
                for (const row of await dataflow.select(table, [...rows.keys()], run)) {
                    selected.add(present(rows.get(row), "a selected row's key"));
                }
            }
        }

        return selected;
    }

    /** Walk deferred rows at a position without sending them, which the subscriber has. */
    async #walkDeferred(position: LogPosition): Promise<void> {
        // leave rows walked already
        if (!this.#isDeferred) {
            return;
        }

        // walk them, dropping every page
        this.#isDeferred = false;
        const pages = await this.walk(this.#view(position), "rebuild");
        while ((await pages.next()).done !== true) {
            // drop each page
        }
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
    async #pagesAfter(position: LogPosition, index: number): Promise<Advance> {
        // decide what follows
        const pages: Page[] = [];
        const advancing = await this.advance(position, true, new AbortController().signal);
        let next = await advancing.next();
        while (next.done !== true) {
            pages.push(next.value);
            next = await advancing.next();
        }

        // refresh once the decisions expired
        const sequence = next.value;
        const until = await this.#audience.until();
        if (until !== undefined && until <= this.#feed.clock.now()) {
            pages.push(
                await this.redecide({ ...position, sequence }, { ...position, sequence }, true, []),
            );
        }

        // move the last page to the sequence the pages arrive at, past commits outside the queries
        const last = pages.at(-1);
        if (last !== undefined && last.position.sequence < sequence) {
            pages[pages.length - 1] = { ...last, position: { ...last.position, sequence } };
        }

        return {
            index,
            from: position.sequence,
            pages,
            sequence,
            until: await this.#audience.until(),
        };
    }

    /** Join the rows some changes affect beyond themselves. */
    async #affected(changes: readonly Change[]): Promise<Affected> {
        // take the rows each change affects, every row once one affects all
        const reads: ((
            positions: readonly LogPosition[],
        ) => Promise<ReadonlyMap<Table, Condition>>)[] = [];
        for (const change of changes) {
            const affected = await this.#audience.affects(change);
            if (affected.kind === "all") {
                return ALL_ROWS;
            } else if (affected.kind === "rows") {
                reads.push(affected.read);
            }
        }

        return reads.length === 0
            ? NO_ROWS
            : {
                  kind: "rows",
                  read: async (positions) =>
                      Evaluation.#joined(await Promise.all(reads.map((read) => read(positions)))),
              };
    }

    /** Join the conditions of several affected row sets, table by table. */
    static #joined(sets: readonly ReadonlyMap<Table, Condition>[]): Map<Table, Condition> {
        const wheres = new Map<Table, Condition[]>();
        for (const [table, where] of sets.flatMap((set) => [...set])) {
            wheres.set(table, [...(wheres.get(table) ?? []), where]);
        }

        return new Map([...wheres].map(([table, each]) => [table, { OR: each }]));
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

/** The pages decided after a sequence, in the order an evaluation decides them. */
export interface Advance {
    /** Its order among the evaluation's advances. */
    readonly index: number;
    /** The sequence it follows. */
    readonly from: number;
    /** The pages, complete at their positions. */
    readonly pages: readonly Page[];
    /** The sequence the pages arrive at. */
    readonly sequence: number;
    /** When the audience's decisions expire. */
    readonly until: number | undefined;
}

/** A changed row's first and last image in a run of changes. */
interface Image {
    /** The row's table. */
    readonly table: Table;
    /** The row before the changes, absent for a row they insert. */
    readonly before: Row | null;
    /** The row after the changes, absent for a row they delete. */
    readonly after: Row | null;
}
