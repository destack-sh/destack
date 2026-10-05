import {
    Change,
    Key,
    TABLE,
    type DatabaseConnection,
    type Row,
    type Table,
    type Scalar,
    type ColumnValue,
    DatabaseError,
    type LogPosition,
} from "@destack/db";
import { canonicalize, aligned, found, zip } from "@destack/schema";
import { watchedScopes, type Audience, type Watch } from "../feed/audience.ts";
import { Node } from "../query/node.ts";
import type { Query, Item, AggregateRow } from "../query/query.ts";
import { Aggregation, Mirror, Relation } from "./aggregation.ts";
import { SyncError } from "../error/error.ts";
import { Materialization } from "./materialization.ts";
import type { Arrangement } from "./filter.ts";
import type { Context, Pipeline } from "./pipeline.ts";
import { Run, type RunCost, type Transition } from "./run.ts";
import type { View } from "./view.ts";
import { PAGE_ROWS, Selection } from "./selection.ts";
import { Sink } from "./sink.ts";
import { Trace, type Upstream } from "./upstream.ts";

/** The rows of an include naming no partition, one list shared so equal reads keep their identity. */
const NO_ROWS: readonly Item[] = Object.freeze([]);

/** Queries compiled to one pipeline per node, kept current as of a log position. */
export class Dataflow implements Arrangement {
    /** The root nodes of the queries. */
    readonly roots: readonly Node[];
    /** Every node of the queries, roots first. */
    readonly nodes: readonly Node[];
    /** What the subscriber has. */
    readonly sink = new Sink();
    /** The current log position, absent before the first hydration. */
    #position: LogPosition | undefined;
    /** What the pipelines share. */
    readonly #context: Context;
    /** Each node's pipeline, absent for stateless nodes. */
    readonly #pipelines = new Map<Node, Pipeline>();
    /** The entries each selection's last read built. */
    readonly #materialized = new Map<Selection, Materialization>();
    /** The row and aggregate pipelines, shallowest first. */
    readonly #steps: readonly Pipeline[];
    /** The relation pipelines, deepest first. */
    readonly #relations: readonly Relation[];
    /** The pipelines deciding rows one by one, by table. */
    readonly #deciding = new Map<Table, Pipeline[]>();
    /** The aggregations counting by images. */
    readonly #linear: readonly Aggregation[];
    /** The aggregations a copy reads from its source. */
    readonly #mirrors: readonly Mirror[];
    /** The arrangement of a copy's relations. */
    readonly #trace: Trace | undefined;
    /** The relations a copy follows through its source, deepest first. */
    readonly #traced: readonly Node[];
    /** The tables and scopes the queries read. */
    readonly #reads: readonly Watch[];
    /** The canonical form of the queries. */
    readonly #queries: string;
    /** The most rows and groups the pipelines may know. */
    readonly #capacity: number | undefined;
    /** Report each run's cost. */
    readonly #observe: ((cost: RunCost) => void) | undefined;
    /** The cost of the last run. */
    #last: RunCost | undefined;
    /** The summed cost of every run, and the run count. */
    #total: RunCost & { readonly runs: number } = {
        runs: 0,
        changes: 0,
        operations: 0,
        decided: 0,
        sent: 0,
        milliseconds: 0,
    };

    /** Compile queries for an audience over a database or a copy. */
    constructor(
        queries: Readonly<Record<string, Query>>,
        options: {
            readonly audience: Audience;
            readonly database: DatabaseConnection;
            readonly upstream?: Upstream;
            readonly changesThrough: Context["changesThrough"];
            readonly isMaterialized?: boolean;
            readonly capacity?: number;
            readonly observe?: (cost: RunCost) => void;
        },
    ) {
        // resolve the queries into trees
        const upstream = options.upstream;
        this.roots = Object.entries(queries).map(
            ([name, query]) => new Node(name, query, query.scopes),
        );
        this.#queries = canonicalize(this.roots.map((root) => [root.name, describe(root)]));
        this.#capacity = options.capacity;
        this.#observe = options.observe;
        this.nodes = this.roots.flatMap((root) => root.nodes());
        this.#trace = upstream === undefined ? undefined : new Trace(upstream);
        this.#context = {
            audience: options.audience,
            sink: this.sink,
            arrangement: this,
            upstream,
            database: options.database,
            isMaterialized: options.isMaterialized ?? false,
            changesThrough: options.changesThrough,
        };
        this.#reads = this.nodes.flatMap(readsOf);

        // refuse unlogged watched tables and concealable columns
        this.#requireLogged();
        this.#requireUnconcealable(options.audience);

        // build each node's pipeline and link them
        this.#build(upstream);
        this.#link();

        // order the pipelines by kind and depth
        const pipelines = [...this.#pipelines.values()];
        this.#steps = pipelines
            .filter((pipeline) => !(pipeline instanceof Relation))
            .toSorted((left, right) => left.node.depth - right.node.depth);
        this.#relations = pipelines
            .filter((pipeline): pipeline is Relation => pipeline instanceof Relation)
            .toSorted((left, right) => right.node.depth - left.node.depth);
        this.#linear = pipelines.filter(isLinear);
        this.#mirrors = pipelines.filter(
            (pipeline): pipeline is Mirror => pipeline instanceof Mirror,
        );
        this.#traced = this.nodes
            .filter((node) => node.kind === "relation" && !this.#pipelines.has(node))
            .toSorted((left, right) => right.depth - left.depth);
    }

    /** Refuse watched tables the log leaves out. */
    #requireLogged(): void {
        for (const watch of this.watches()) {
            if (watch.table[TABLE].retention === "none") {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `watched table is not logged: ${watch.table[TABLE].name}`,
                );
            }
        }
    }

    /** Refuse nodes reading columns the audience may conceal. */
    #requireUnconcealable(audience: Audience): void {
        for (const node of this.nodes) {
            const concealable = audience.concealable(node.table);
            const read = node.reads().filter((name) => concealable.includes(name));
            if (read.length > 0) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `query ${node.name} reads concealable columns of ${node.table[TABLE].name}: ${read.join(", ")}`,
                );
            }
        }
    }

    /** Build the pipeline of each stateful node. */
    #build(upstream: Upstream | undefined): void {
        for (const node of this.nodes) {
            const pipeline = this.#pipelineOf(node, upstream);
            if (pipeline !== undefined) {
                this.#pipelines.set(node, pipeline);
            }
        }
    }

    /** Link each pipeline to its parent and children and index the deciding ones by table. */
    #link(): void {
        for (const [node, pipeline] of this.#pipelines) {
            // link the parent and children
            pipeline.parent =
                node.parent === undefined ? undefined : this.#pipelines.get(node.parent);
            pipeline.children = node.children.map((child) => this.#pipelines.get(child));

            // index a pipeline deciding rows one by one
            const isDeciding =
                pipeline instanceof Selection ||
                (pipeline instanceof Aggregation && pipeline.isTracked);
            if (isDeciding) {
                this.#deciding.set(node.table, [
                    ...(this.#deciding.get(node.table) ?? []),
                    pipeline,
                ]);
            }
        }
    }

    /** The current log position, absent before the first hydration. */
    get position(): LogPosition | undefined {
        return this.#position;
    }

    /** Move to a later position of the same epoch without watched changes. */
    advance(position: LogPosition): void {
        this.#position = position;
    }

    /** The name of the queries and the audience. */
    get key(): string {
        return `${this.#queries}\n${this.#context.audience.key}`;
    }

    /** List the tables and scopes the dataflow reads. */
    watches(): Watch[] {
        return [
            ...this.#reads,
            ...this.#context.audience.watches,
            ...(this.#context.upstream?.watches ?? []),
        ];
    }

    /** The rows and groups the pipelines know. */
    get size(): number {
        let size = 0;
        for (const pipeline of this.#pipelines.values()) {
            size += pipeline.size;
        }

        return size;
    }

    /** Forget everything and select nothing as of a position. */
    forget(position: LogPosition): void {
        // select nothing as of the position
        this.#position = position;
        for (const pipeline of this.#pipelines.values()) {
            pipeline.forget();
        }
        this.sink.forget();
        this.#trace?.forget();
    }

    /** Select every root's rows and results as of a run's position, yielding after each page. */
    async *hydrate(run: Run, start?: Row): AsyncGenerator<void> {
        // measure every relation first
        const cost = this.#measure(run);
        for (const relation of this.#relations) {
            await relation.hydrate(run);
        }
        for (const root of this.roots) {
            // open the root's one partition
            const pipeline = found(this.#pipelines, root);
            pipeline.openRoot(start);

            // count an aggregate or fill a window
            if (root.aggregate !== undefined || root.limit !== undefined) {
                run.workOf(pipeline).opened.set("", null);
                await this.#settle(run);
                await this.sink.emit(run);
                yield;
                continue;
            }

            // select the root's rows a page at a time
            if (!(pipeline instanceof Selection)) {
                throw new TypeError(`root ${root.name} selects rows without a selection`);
            }
            let after = start;
            for (let isDone = false; !isDone;) {
                const rows = await pipeline.page(after, run);
                isDone = rows.length < PAGE_ROWS;
                after = rows.at(-1);
                await this.#settle(run);
                await this.sink.emit(run);
                yield;
            }
        }
        this.#position = run.view.position;
        cost();
        this.#requireCapacity();
    }

    /** Select every root's rows and results as of a view. */
    async load(view: View, start?: Row): Promise<void> {
        // forget everything and select every batch as of the view
        this.forget(view.position);
        const run = new Run(view, this.#context.audience, "collect");
        const batches = this.hydrate(run, start);
        while ((await batches.next()).done !== true) {
            // select every batch
        }
    }

    /** Apply a run of committed changes, returning false when access changed too far to follow. */
    async step(run: Run): Promise<boolean> {
        // take each changed row's first and last image
        const cost = this.#measure(run);
        const transitions = new Map<Table, Map<string, Transition>>();
        for (const change of run.changes) {
            const rows = transitions.get(change.table) ?? new Map<string, Transition>();
            transitions.set(change.table, rows);
            const key = Key.name(change.table, change.key);
            const known = rows.get(key);
            rows.set(key, {
                before: known === undefined ? Change.before(change) : known.before,
                after: Change.after(change),
            });
        }

        // decide each changed row and the rows it moves again
        for (const [table, rows] of transitions) {
            for (const [key, { before, after }] of rows) {
                run.changed.add(key);
                this.#mark(run, table, key, after);
                await this.#moved(run, table, before, after, false);
            }
        }

        // decide again the rows the audience decides again
        if (!(await this.#redecide(run))) {
            return false;
        }

        // measure the relations the source measured and those the copy measures itself
        await this.#trace?.relate(
            this.#traced,
            (relation) => this.#pipelines.get(relation.link().parent),
            run,
        );
        for (const relation of this.#relations) {
            await relation.step(run.workOf(relation), run);
        }
        await this.#settle(run);
        for (const aggregation of this.#linear) {
            await aggregation.tally(run);
        }

        // read a copy's changed aggregates again
        await this.#refreshMirrors(run);
        await this.sink.emit(run);
        this.#position = run.view.position;
        cost();
        this.#requireCapacity();

        return true;
    }

    /** Decide again the rows the audience's changed access rows name, returning false when it names everything. */
    async #redecide(run: Run): Promise<boolean> {
        // collect the rows each change names
        const affected = new Map<Table, Row[]>();
        for (const change of run.changes) {
            const entries = await this.#context.audience.dependents(change);
            if (entries === "everything") {
                return false;
            }
            for (const entry of entries) {
                affected.set(entry.table, [...(affected.get(entry.table) ?? []), entry.key]);
            }
        }

        // read each named row and decide it again where it is
        for (const [table, keys] of affected) {
            const byKey = run.affected.get(table) ?? new Map<string, Row>();
            run.affected.set(table, byKey);
            for (const [key, row] of zip(keys, await run.view.keyed(table, keys))) {
                const name = Key.name(table, key);
                if (row !== null) {
                    byKey.set(name, row);
                }
                run.changed.add(name);
                this.#mark(run, table, name, row);
                await this.#moved(run, table, row, row, true);
            }
        }

        return true;
    }

    /** Read again the groups of each mirror whose table or source groups a run changed. */
    async #refreshMirrors(run: Run): Promise<void> {
        for (const mirror of this.#mirrors) {
            // require the upstream the mirror reads
            const upstream = this.#context.upstream;
            if (upstream === undefined) {
                throw new TypeError(
                    `mirror ${mirror.node.name} reads its groups without an upstream`,
                );
            }

            // refresh a mirror whose rows or groups changed
            const isChanged = run.changes.some(
                (change) =>
                    change.table === mirror.node.table ||
                    upstream.groupOf(change)?.query === mirror.node.name,
            );
            if (isChanged) {
                await mirror.refresh(run);
            }
        }
    }

    /** Read a root's current items: its rows with what their includes select. */
    async read(name: string): Promise<readonly Item[]> {
        const pipeline = this.#root(name);
        if (!(pipeline instanceof Selection)) {
            throw new TypeError(`dataflow query ${name} measures groups instead of rows`);
        }

        return found(await this.#nest(pipeline, [""]), "");
    }

    /** Read an aggregate root's current groups. */
    async results(name: string): Promise<readonly AggregateRow[]> {
        const pipeline = this.#root(name);
        if (pipeline instanceof Selection) {
            throw new TypeError(`dataflow query ${name} selects rows instead of groups`);
        }

        return groupsOf(pipeline, "").map(({ group, values }) => ({ group, values }));
    }

    /** Find a root's pipeline, requiring a materialized dataflow. */
    #root(name: string): Pipeline {
        // require a materialized dataflow
        if (!this.#context.isMaterialized) {
            throw new TypeError("a dataflow reads its results only when it materializes its rows");
        }
        const root = this.roots.find((entry) => entry.name === name);
        if (root === undefined) {
            throw new TypeError(`dataflow has no query ${name}`);
        }

        return found(this.#pipelines, root);
    }

    /** Look up a relation's measures of the rows naming a parent value. */
    measured(
        relation: Node,
        value: ColumnValue | undefined,
        run: Run,
    ): Readonly<Record<string, Scalar>> {
        // read the source's measures or the relation's tally of the parent value
        const pipeline = this.#pipelines.get(relation);
        if (this.#trace !== undefined && pipeline === undefined) {
            return this.#trace.measured(relation, value, run);
        } else if (!(pipeline instanceof Relation)) {
            throw new TypeError(`relation ${relation.name} has no relation pipeline`);
        }
        const tally = pipeline.tallies.get(JSON.stringify(relation.groupFor(value)));

        return tally === undefined || tally.isEmpty ? {} : tally.values();
    }

    /** Read the source's measures of a node's relations the source measured. */
    async prepare(node: Node, _rows: readonly Row[], run: Run): Promise<void> {
        await this.#trace?.read(
            node.relations.filter((relation) => !this.#pipelines.has(relation)),
            run,
        );
    }

    /** Build a node's pipeline: a relation's unless traced upstream, an aggregate's, or a selection's, none for a node measured elsewhere. */
    #pipelineOf(node: Node, upstream: Upstream | undefined): Pipeline | undefined {
        const hasTreeIndex = upstream === undefined;
        const isMeasured = upstream !== undefined && upstream.isMeasured?.(node) !== false;

        // follow a relation here unless its upstream traces it
        if (node.kind === "relation") {
            return isMeasured ? undefined : new Relation(node, this.#context, hasTreeIndex);
        }
        // measure an aggregate here or mirror the upstream's measures
        else if (node.aggregate !== undefined) {
            return isMeasured
                ? new Mirror(node, this.#context, hasTreeIndex)
                : new Aggregation(node, this.#context, hasTreeIndex);
        }
        // select rows unless the node sits below an aggregate
        else {
            return node.hasRows ? new Selection(node, this.#context, hasTreeIndex) : undefined;
        }
    }

    /** Describe the pipelines and run costs. */
    inspect(): DataflowInspection {
        return {
            ...(this.#position === undefined ? {} : { position: this.#position }),
            size: this.size,
            ...(this.#capacity === undefined ? {} : { capacity: this.#capacity }),
            ...(this.#last === undefined ? {} : { last: this.#last }),
            total: this.#total,
            pipelines: [...this.#pipelines.values()].map((pipeline) => ({
                node: pipeline.node.name,
                kind: pipelineKind(pipeline),
                path: pipeline.node.path?.kind ?? "root",
                partitions: pipeline.partitions.size,
                members: pipeline.members.size,
                size: pipeline.size,
            })),
        };
    }

    /** Start measuring a run, returning the function that records its cost. */
    #measure(run: Run): () => void {
        // note the operations and time
        const database = this.#context.database;
        const operations = database.driver.state.operations;
        const started = performance.now();

        return () => {
            // add up and report the cost
            const cost = {
                changes: run.changes.length,
                operations: database.driver.state.operations - operations,
                decided: run.decided,
                sent: run.patch.size,
                milliseconds: performance.now() - started,
            };
            const total = this.#total;
            this.#last = cost;
            this.#total = {
                runs: total.runs + 1,
                changes: total.changes + cost.changes,
                operations: total.operations + cost.operations,
                decided: total.decided + cost.decided,
                sent: total.sent + cost.sent,
                milliseconds: total.milliseconds + cost.milliseconds,
            };
            this.#observe?.(cost);
        };
    }

    /** Require the pipelines to stay within the capacity. */
    #requireCapacity(): void {
        const size = this.size;
        if (this.#capacity !== undefined && size > this.#capacity) {
            throw new SyncError(
                "OVER_CAPACITY",
                `queries hold ${size} rows and groups, more than ${this.#capacity}`,
            );
        }
    }

    /** Step every pipeline with work and show the relation groups. */
    async #settle(run: Run): Promise<void> {
        for (const pipeline of this.#steps) {
            const work = run.work.get(pipeline);
            if (work !== undefined) {
                run.work.delete(pipeline);
                await pipeline.step(work, run);
            }
        }
        for (const relation of this.#relations) {
            const work = run.work.get(relation);
            if (work !== undefined) {
                run.work.delete(relation);
                relation.show(work, run);
            }
        }
    }

    /** Decide a row again at every pipeline of its table. */
    #mark(run: Run, table: Table, key: string, row: Row | null): void {
        for (const pipeline of this.#deciding.get(table) ?? []) {
            run.workOf(pipeline).dirty.set(key, row);
        }
    }

    /** Decide again the rows a changed row moves through each pipeline's path. */
    async #moved(
        run: Run,
        table: Table,
        before: Row | null,
        after: Row | null,
        isAccess: boolean,
    ): Promise<void> {
        for (const pipeline of [...this.#steps, ...this.#relations]) {
            const dirty = new Map<string, Row | null>();
            await pipeline.input.moves(table, before, after, isAccess, run, dirty);
            for (const [key, row] of dirty) {
                run.workOf(pipeline).dirty.set(key, row);
            }
        }
    }

    /** Read a selection's rows in order, with concealed columns left out and includes nested, rebuilding only what changed. */
    async #nest(
        pipeline: Selection,
        names: readonly string[],
    ): Promise<Map<string, readonly Item[]>> {
        // order every partition's keys by placing the rows changed since the last read
        const node = pipeline.node;
        const materialization = this.#materialization(pipeline);
        const rowOf = (key: string) => {
            const row = found(pipeline.members, key).row;
            if (row === undefined) {
                throw new TypeError(`member ${key} of ${node.name} keeps no row`);
            }

            return row;
        };
        const compare = (left: Row, right: Row) => node.compare(left, right);
        const orders = names.map((name) =>
            materialization.order(name, pipeline.keys(name), rowOf, compare),
        );

        // read the concealed columns of every selected row
        const current = this.#position;
        if (current === undefined) {
            throw new TypeError("a dataflow reads its rows before its first hydration");
        }
        const rows = orders.map((keys) => keys.map(rowOf));
        const concealed = await this.#context.audience.conceals(node.table, rows.flat(), current);

        // read each include's items or groups by the partition each row names
        const { included, measured } = await this.#includes(
            pipeline,
            orders,
            rows,
            materialization,
        );

        // take each unchanged entry and list again, building only the changed ones
        let offset = 0;
        const lists = zip(names, zip(orders, rows)).map(([name, [keys, listed]], position) => {
            const entries = zip(keys, listed).map(([key, row], index) =>
                materialization.entry(
                    key,
                    row,
                    aligned(concealed, offset++),
                    included.map(
                        ({ name: include, items }) =>
                            [include, aligned(aligned(items, position), index)] as const,
                    ),
                    measured.map(
                        ({ name: include, groups }) =>
                            [include, aligned(aligned(groups, position), index)] as const,
                    ),
                ),
            );

            return materialization.list(name, entries);
        });
        materialization.prune(pipeline.members, pipeline.partitions);

        return new Map(zip(names, lists));
    }

    /** Read the materialization of a selection's last read, starting one on the first. */
    #materialization(pipeline: Selection): Materialization {
        // reuse the last read's
        const known = this.#materialized.get(pipeline);
        if (known !== undefined) {
            return known;
        }
        const started = new Materialization();
        this.#materialized.set(pipeline, started);

        return started;
    }

    /** Read each include's nested items, or its groups kept equal to the last read's, by the partition each selected row names. */
    async #includes(
        pipeline: Selection,
        orders: readonly (readonly string[])[],
        rows: readonly (readonly Row[])[],
        materialization: Materialization,
    ): Promise<{
        readonly included: { readonly name: string; readonly items: (readonly Item[])[][] }[];
        readonly measured: {
            readonly name: string;
            readonly groups: (readonly AggregateRow[])[][];
        }[];
    }> {
        // collect the includes in child order
        const node = pipeline.node;
        const included: { readonly name: string; readonly items: (readonly Item[])[][] }[] = [];
        const measured: {
            readonly name: string;
            readonly groups: (readonly AggregateRow[])[][];
        }[] = [];
        for (const [child, pipelined] of zip(node.children, pipeline.children)) {
            // name each selected row's partition of the include
            if (child.kind !== "include" || pipelined === undefined) {
                continue;
            }
            const name = child.name.slice(node.name.length + 1);
            const partitions = partitionsOf(child, rows);

            // take the nested items of each row's partition
            if (pipelined instanceof Selection) {
                included.push({ name, items: await this.#nestedItems(pipelined, partitions) });
            }
            // keep equal groups as the last read had them
            else {
                const groups = reuseEqualGroups(
                    child,
                    pipelined,
                    name,
                    partitions,
                    orders,
                    materialization,
                );
                measured.push({ name, groups });
            }
        }

        return { included, measured };
    }

    /** Read the nested items of each selected row's partition of an include. */
    async #nestedItems(
        pipeline: Selection,
        partitions: readonly (readonly (string | undefined)[])[],
    ): Promise<(readonly Item[])[][]> {
        // nest every named partition once
        const named = [...new Set(partitions.flat().filter((entry) => entry !== undefined))];
        const nested = await this.#nest(pipeline, named);

        return partitions.map((list) =>
            list.map((partition) =>
                partition === undefined ? NO_ROWS : (nested.get(partition) ?? NO_ROWS),
            ),
        );
    }
}

/** Decide whether a pipeline is an aggregation counting by images. */
function isLinear(pipeline: Pipeline): pipeline is Aggregation {
    return (
        pipeline instanceof Aggregation && !(pipeline instanceof Relation) && !pipeline.isTracked
    );
}

/** List the tables and scopes a node reads. */
function readsOf(node: Node): Watch[] {
    return [
        { table: node.table, scopes: node.scopes },
        ...(node.closure === undefined ? [] : [{ table: node.closure, scopes: node.scopes }]),
        ...(node.path?.kind === "junction"
            ? [{ table: node.path.table, scopes: node.scopes }]
            : []),
    ];
}

/** Read each selected row's partition of an include. */
function partitionsOf(child: Node, rows: readonly (readonly Row[])[]): (string | undefined)[][] {
    return rows.map((list) =>
        list.map((row) => {
            const value = child.valueOf(row);

            return isPresent(value) ? child.partition(value) : undefined;
        }),
    );
}

/** Read an aggregate include's groups per selected row and reuse equal ones. */
function reuseEqualGroups(
    child: Node,
    pipeline: Pipeline,
    name: string,
    partitions: readonly (readonly (string | undefined)[])[],
    orders: readonly (readonly string[])[],
    materialization: Materialization,
): (readonly AggregateRow[])[][] {
    return zip(partitions, orders).map(([list, keys]) =>
        zip(list, keys).map(([partition, key]) => {
            // reuse the previous groups when equal
            const groups = measuresOf(child, pipeline, partition);
            const previous = materialization.previous(key, name);
            const isUnchanged = previous !== undefined && Materialization.isSame(previous, groups);

            return isUnchanged ? previous : groups;
        }),
    );
}

/** Read an aggregate include's groups for one selected row's partition, a whole include's measures as one group. */
function measuresOf(
    child: Node,
    pipeline: Pipeline,
    name: string | undefined,
): readonly AggregateRow[] {
    // list the partition's groups
    const aggregate = child.aggregate;
    if (aggregate === undefined) {
        throw new TypeError(`include ${child.name} measures nothing`);
    }
    const partitionName = child.link().partitionName;
    const listed = (name === undefined ? [] : groupsOf(pipeline, name)).map(
        ({ group: { [partitionName]: _joined, ...group }, values }) => ({ group, values }),
    );

    // take the one group's measures
    return aggregate.groupBy === undefined || aggregate.groupBy.length === 0
        ? [{ group: {}, values: listed[0]?.values ?? child.emptyValues() }]
        : listed;
}

/** Read an aggregate pipeline's groups in one partition. */
function groupsOf(pipeline: Pipeline, name: string): AggregateRow[] {
    // read the mirrored or counted groups
    const node = pipeline.node;
    let groups: AggregateRow[];
    if (pipeline instanceof Mirror) {
        groups = pipeline.groups();
    } else if (pipeline instanceof Aggregation) {
        groups = [...pipeline.tallies.values()]
            .filter((tally) => !tally.isEmpty)
            .map((tally) => ({ group: tally.group, values: tally.values() }));
    } else {
        throw new TypeError(`node ${node.name} has groups without an aggregate pipeline`);
    }

    // keep the partition's groups in order
    const partitionName = node.path === undefined ? undefined : node.link().partitionName;

    return groups
        .filter(
            ({ group }) =>
                partitionName === undefined || JSON.stringify(group[partitionName]) === name,
        )
        .toSorted((left, right) => (canonicalize(left.group) < canonicalize(right.group) ? -1 : 1));
}

/** Whether a value is present. */
function isPresent(value: unknown): boolean {
    return value !== null && value !== undefined;
}

/** Read the watched changes between two sequences from a database's log, absent once compacted. */
export function changesThroughLog(database: DatabaseConnection): Context["changesThrough"] {
    return async (watches, after, through) => {
        // read the scopes the watches read, every scope when one reads them all
        const scopes = watchedScopes(watches);
        const tables = [...new Set(watches.map((entry) => entry.table))];
        try {
            return await database.log.range(
                { tables, after, ...(scopes === undefined ? {} : { scopes }) },
                through,
            );
        } catch (error) {
            if (error instanceof DatabaseError && error.code === "CHANGES_COMPACTED") {
                return undefined;
            }
            throw error;
        }
    };
}

/** Describe a resolved query tree in JSON, as a dataflow's identity. */
function describe(node: Node): unknown {
    return {
        selection: node.selection,
        limit: node.limit ?? null,
        aggregate: node.aggregate ?? null,
        children: node.children.map((child) => [child.kind, describe(child)]),
    };
}

/** The inspection of a dataflow. */
export interface DataflowInspection {
    /** The current log position. */
    readonly position?: LogPosition;
    /** The rows and groups the pipelines know. */
    readonly size: number;
    /** The most rows and groups they may know. */
    readonly capacity?: number;
    /** The cost of the last run. */
    readonly last?: RunCost;
    /** The summed cost of every run, and the run count. */
    readonly total: RunCost & { readonly runs: number };
    /** Each node's pipeline. */
    readonly pipelines: readonly {
        /** The node's path of names. */
        readonly node: string;
        /** What the pipeline keeps. */
        readonly kind: "selection" | "aggregation" | "relation" | "mirror";
        /** How the node's rows reach their partitions. */
        readonly path: string;
        /** The open partitions. */
        readonly partitions: number;
        /** The selected rows. */
        readonly members: number;
        /** The window rows or groups the pipeline knows. */
        readonly size: number;
    }[];
}

/** Name a pipeline's kind as an inspection reports it. */
function pipelineKind(pipeline: Pipeline): DataflowInspection["pipelines"][number]["kind"] {
    // name each pipeline class
    if (pipeline instanceof Selection) {
        return "selection";
    } else if (pipeline instanceof Relation) {
        return "relation";
    } else if (pipeline instanceof Aggregation) {
        return "aggregation";
    } else {
        return "mirror";
    }
}
