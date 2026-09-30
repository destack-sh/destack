import { Key, TABLE, type DatabaseConnection, type Row, type Table } from "@destack/db";
import type { Scalar } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import type { Change, LogPosition } from "@destack/db/log";
import { canonicalize } from "@destack/schema/json";
import { watchedScopes, type Audience, type Watch } from "../feed/audience.ts";
import { Node } from "../query/node.ts";
import type { Query } from "../query/query.ts";
import { Aggregation, Mirror, Relation } from "./aggregation.ts";
import { SyncError } from "../error/error.ts";
import type { Include } from "../query/query.ts";
import type { Arrangement } from "./filter.ts";
import type { Context, Pipeline } from "./pipeline.ts";
import { Run, type RunCost, type Transition } from "./run.ts";
import type { View } from "./view.ts";
import { PAGE_ROWS, Selection } from "./selection.ts";
import { Sink } from "./sink.ts";
import { Trace, type Upstream } from "./upstream.ts";

/** Queries compiled to one pipeline per node, kept current as of a log position. */
export class Dataflow implements Arrangement {
    /** The root nodes of the queries. */
    readonly roots: readonly Node[];
    /** Every node of the queries, roots first. */
    readonly nodes: readonly Node[];
    /** What the subscriber holds. */
    readonly sink = new Sink();
    /** The held log position, absent before the first hydration. */
    #position: LogPosition | undefined;
    /** What the pipelines share. */
    readonly #context: Context;
    /** Each node's pipeline, absent for stateless nodes. */
    readonly #pipelines = new Map<Node, Pipeline>();
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
        this.#queries = canonicalize(
            Object.entries(queries).map(([name, query]) => [name, describe(query)]),
        );
        this.#capacity = options.capacity;
        this.#observe = options.observe;
        this.roots = Object.entries(queries).map(
            ([name, query]) => new Node(name, query, query.scopes),
        );
        this.nodes = this.roots.flatMap((root) => root.nodes());
        this.#trace = upstream === undefined ? undefined : new Trace(upstream);
        this.#context = {
            audience: options.audience,
            sink: this.sink,
            arrangement: this.#trace ?? this,
            upstream,
            database: options.database,
            isMaterialized: options.isMaterialized ?? false,
            changesThrough: options.changesThrough,
        };
        this.#reads = this.nodes.flatMap((node) => [
            { table: node.table, scopes: node.scopes },
            ...(node.closure === undefined ? [] : [{ table: node.closure, scopes: node.scopes }]),
            ...(node.path?.kind === "junction"
                ? [{ table: node.path.table, scopes: node.scopes }]
                : []),
        ]);

        // refuse unlogged watched tables and concealable columns
        for (const watch of this.watches()) {
            if (watch.table[TABLE].retention === "none") {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `watched table is not logged: ${watch.table[TABLE].name}`,
                );
            }
        }
        for (const node of this.nodes) {
            const concealable = options.audience.concealable(node.table);
            const read = node.reads().filter((name) => concealable.includes(name));
            if (read.length > 0) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `query ${node.name} reads concealable columns of ${node.table[TABLE].name}: ${read.join(", ")}`,
                );
            }
        }

        // build each node's pipeline
        const hasTreeIndex = upstream === undefined;
        for (const node of this.nodes) {
            const pipeline =
                node.kind === "relation"
                    ? upstream === undefined
                        ? new Relation(node, this.#context, hasTreeIndex)
                        : undefined
                    : node.aggregate !== undefined
                      ? upstream === undefined || upstream.isMeasured?.(node) === false
                          ? new Aggregation(node, this.#context, hasTreeIndex)
                          : new Mirror(node, this.#context, hasTreeIndex)
                      : node.isHolding
                        ? new Selection(node, this.#context, hasTreeIndex)
                        : undefined;
            if (pipeline !== undefined) {
                this.#pipelines.set(node, pipeline);
            }
        }

        // link the pipelines and index them by table
        for (const [node, pipeline] of this.#pipelines) {
            pipeline.parent =
                node.parent === undefined ? undefined : this.#pipelines.get(node.parent);
            pipeline.children = node.children.map((child) => this.#pipelines.get(child));
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
        const pipelines = [...this.#pipelines.values()];
        this.#steps = pipelines
            .filter((pipeline) => !(pipeline instanceof Relation))
            .sort((left, right) => left.node.depth - right.node.depth);
        this.#relations = pipelines
            .filter((pipeline): pipeline is Relation => pipeline instanceof Relation)
            .sort((left, right) => right.node.depth - left.node.depth);
        this.#linear = pipelines.filter(
            (pipeline): pipeline is Aggregation =>
                pipeline instanceof Aggregation &&
                !(pipeline instanceof Relation) &&
                !pipeline.isTracked,
        );
        this.#mirrors = pipelines.filter(
            (pipeline): pipeline is Mirror => pipeline instanceof Mirror,
        );
        this.#traced = this.nodes
            .filter((node) => upstream !== undefined && node.kind === "relation")
            .sort((left, right) => right.depth - left.depth);
    }

    /** The held log position, absent before the first hydration. */
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

    /** Forget everything and hold nothing as of a position. */
    forget(position: LogPosition): void {
        // hold nothing as of the position
        this.#position = position;
        for (const pipeline of this.#pipelines.values()) {
            pipeline.forget();
        }
        this.sink.forget();
        this.#trace?.forget();
    }

    /** Hold every root's rows and results as of a run's position, yielding after each page. */
    async *hydrate(run: Run, start?: Row): AsyncGenerator<void> {
        // measure every relation first
        const cost = this.#measure(run);
        for (const relation of this.#relations) {
            await relation.hydrate(run);
        }
        for (const root of this.roots) {
            // open the root's one partition
            const pipeline = this.#pipelines.get(root)!;
            pipeline.openRoot(start);

            // count an aggregate or fill a window
            if (root.aggregate !== undefined || root.limit !== undefined) {
                run.workOf(pipeline).opened.set("", undefined);
                await this.#settle(run);
                await this.sink.emit(run);
                yield;
                continue;
            }

            // hold the root's rows a page at a time
            const selection = pipeline as Selection;
            let after = start;
            for (let isDone = false; !isDone;) {
                const rows = await selection.page(after, run);
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

    /** Hold every root's rows and results as of a view. */
    async fill(view: View, start?: Row): Promise<void> {
        this.forget(view.position);
        const run = new Run(view, this.#context.audience, "collect");
        for await (const _batch of this.hydrate(run, start)) {
            // hold every batch
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
                before: known === undefined ? change.before : known.before,
                after: change.after,
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
        for (const [table, keys] of affected) {
            const rows = await run.view.keyed(table, keys);
            const byKey = run.affected.get(table) ?? new Map<string, Row>();
            run.affected.set(table, byKey);
            for (const [index, key] of keys.entries()) {
                const name = Key.name(table, key);
                const row = rows[index];
                if (row !== undefined) {
                    byKey.set(name, row);
                }
                run.changed.add(name);
                this.#mark(run, table, name, row);
                await this.#moved(run, table, row, row, true);
            }
        }

        // measure the relations, decide the pipelines, and count linear aggregates
        if (this.#trace !== undefined) {
            await this.#trace.relate(
                this.#traced,
                (relation) => this.#pipelines.get(relation.parent!),
                run,
            );
        } else {
            for (const relation of this.#relations) {
                await relation.step(run.workOf(relation), run);
            }
        }
        await this.#settle(run);
        for (const aggregation of this.#linear) {
            await aggregation.tally(run);
        }

        // read a copy's changed aggregates again
        for (const mirror of this.#mirrors) {
            const upstream = this.#context.upstream!;
            if (
                run.changes.some(
                    (change) =>
                        change.table === mirror.node.table ||
                        upstream.groupOf(change)?.query === mirror.node.name,
                )
            ) {
                await mirror.refresh(run);
            }
        }
        await this.sink.emit(run);
        this.#position = run.view.position;
        cost();
        this.#requireCapacity();

        return true;
    }

    /** Read a root's held result: its rows with their includes nested, or its groups. */
    async read(name: string): Promise<Readonly<Record<string, unknown>>[]> {
        // require a materialized dataflow
        if (!this.#context.isMaterialized) {
            throw new TypeError("a dataflow reads its results only when it materializes its rows");
        }
        const root = this.roots.find((entry) => entry.name === name)!;
        const pipeline = this.#pipelines.get(root)!;

        return pipeline instanceof Selection
            ? (await this.#nest(pipeline, [""])).get("")!
            : groupsOf(pipeline, "").map(({ group, values }) => ({ group, values }));
    }

    /** Look up a relation's measures of the rows naming a held value. */
    measured(relation: Node, value: unknown): Readonly<Record<string, Scalar>> {
        const pipeline = this.#pipelines.get(relation) as Relation;
        const tally = pipeline.tallies.get(JSON.stringify(relation.groupFor(value)));

        return tally === undefined || tally.isEmpty ? {} : tally.values();
    }

    /** Prepare nothing. */
    async prepare(): Promise<void> {}

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
                kind:
                    pipeline instanceof Selection
                        ? ("selection" as const)
                        : pipeline instanceof Relation
                          ? ("relation" as const)
                          : pipeline instanceof Aggregation
                            ? ("aggregation" as const)
                            : ("mirror" as const),
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
        const held = this.size;
        if (this.#capacity !== undefined && held > this.#capacity) {
            throw new SyncError(
                "OVER_CAPACITY",
                `queries hold ${held} rows and groups, more than ${this.#capacity}`,
            );
        }
    }

    /** Step every pipeline with work, then show the relation groups. */
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
    #mark(run: Run, table: Table, key: string, row: Row | undefined): void {
        for (const pipeline of this.#deciding.get(table) ?? []) {
            run.workOf(pipeline).dirty.set(key, row);
        }
    }

    /** Decide again the rows a changed row moves through each pipeline's path. */
    async #moved(
        run: Run,
        table: Table,
        before: Row | undefined,
        after: Row | undefined,
        isAccess: boolean,
    ): Promise<void> {
        for (const pipeline of [...this.#steps, ...this.#relations]) {
            const dirty = new Map<string, Row | undefined>();
            await pipeline.input.moves(table, before, after, isAccess, run, dirty);
            for (const [key, row] of dirty) {
                run.workOf(pipeline).dirty.set(key, row);
            }
        }
    }

    /** Read a selection's held rows in order, with concealed columns left out and includes nested. */
    async #nest(
        pipeline: Selection,
        names: readonly string[],
    ): Promise<Map<string, Readonly<Record<string, unknown>>[]>> {
        // take every partition's held rows
        const node = pipeline.node;
        const ordered = names.map((name) =>
            pipeline
                .keys(name)
                .map((key) => pipeline.members.get(key)!.row!)
                .sort((left, right) => node.compare(left, right)),
        );

        // leave out concealed columns
        const concealed = await this.#context.audience.conceals(
            node.table,
            ordered.flat(),
            this.#position!,
        );
        let offset = 0;
        const entries = ordered.map((list) =>
            list.map((row) => {
                const hidden = concealed[offset++]!;

                return Object.fromEntries(
                    Object.entries(row).filter(([column]) => !hidden.includes(column)),
                ) as Record<string, unknown>;
            }),
        );

        // nest each include's rows or groups
        for (const [index, child] of node.children.entries()) {
            const included = pipeline.children[index];
            if (child.kind !== "include" || included === undefined) {
                continue;
            }
            const property = child.name.slice(node.name.length + 1);
            const partitions = ordered.map((list) => list.map((row) => child.valueOf(row)));
            const named = [
                ...new Set(
                    partitions
                        .flat()
                        .filter(isPresent)
                        .map((value) => child.partition(value)),
                ),
            ];
            const nested =
                included instanceof Selection ? await this.#nest(included, named) : undefined;
            const isOne =
                child.path?.kind === "key" &&
                child.key.length === 1 &&
                child.key[0] === child.path.column;
            for (const [position, list] of entries.entries()) {
                for (const [index, entry] of list.entries()) {
                    const value = partitions[position]![index];
                    const name = isPresent(value) ? child.partition(value) : undefined;
                    const rows = name === undefined ? [] : (nested?.get(name) ?? []);
                    entry[property] =
                        nested === undefined
                            ? measuresOf(child, included, name)
                            : isOne
                              ? (rows[0] ?? null)
                              : rows;
                }
            }
        }

        return new Map(names.map((name, index) => [name, entries[index]!]));
    }
}

/** Read an aggregate include's result for one held row's partition. */
function measuresOf(child: Node, pipeline: Pipeline, name: string | undefined): unknown {
    // list the partition's groups
    const listed = (name === undefined ? [] : groupsOf(pipeline, name)).map(
        ({ group: { [child.partitionName!]: _joined, ...group }, values }) => ({ group, values }),
    );

    // take the one group's measures
    return child.aggregate!.groupBy === undefined || child.aggregate!.groupBy.length === 0
        ? (listed[0]?.values ?? child.emptyValues())
        : listed;
}

/** Read an aggregate pipeline's groups in one partition. */
function groupsOf(
    pipeline: Pipeline,
    name: string,
): { readonly group: Record<string, Scalar>; readonly values: Record<string, Scalar> }[] {
    const node = pipeline.node;
    const groups =
        pipeline instanceof Mirror
            ? pipeline.groups()
            : [...(pipeline as Aggregation).tallies.values()]
                  .filter((tally) => !tally.isEmpty)
                  .map((tally) => ({ group: tally.group, values: tally.values() }));

    return groups
        .filter(
            ({ group }) =>
                node.path === undefined || JSON.stringify(group[node.partitionName!]) === name,
        )
        .sort((left, right) => (canonicalize(left.group) < canonicalize(right.group) ? -1 : 1));
}

/** Whether a value is present. */
function isPresent(value: unknown): boolean {
    return value !== null && value !== undefined;
}

/** Read the watched changes between two sequences from a database's log. */
export function changesThroughLog(database: DatabaseConnection): Context["changesThrough"] {
    return async (watches, after, through) => {
        // read the scopes the watches read, every scope when one reads them all
        const scopes = watchedScopes(watches);
        const changes: Change[] = [];
        for (let sequence = after; sequence < through;) {
            // read a page, absent once compacted
            let read;
            try {
                read = await database.log.read({
                    tables: [...new Set(watches.map((entry) => entry.table))],
                    after: sequence,
                    ...(scopes === undefined ? {} : { scopes }),
                });
            } catch (error) {
                if (error instanceof DatabaseError && error.code === "CHANGES_COMPACTED") {
                    return undefined;
                }
                throw error;
            }

            // keep the changes up to the end sequence
            changes.push(...read.changes.filter((change) => change.sequence <= through));
            if (read.sequence <= sequence) {
                break;
            }
            sequence = read.sequence;
        }

        return changes;
    };
}

/** Describe a query or include in JSON. */
function describe(
    query:
        | (Omit<Query, "scopes"> & { readonly scopes?: Query["scopes"] })
        | NonNullable<Query["relations"]>[string],
): unknown {
    return {
        ...query,
        table: query.table[TABLE].sqlName,
        ...("on" in query && (query as Include).on.kind === "junction"
            ? {
                  on: {
                      ...(query as Include).on,
                      table: ((query as Include).on as { table: Table }).table[TABLE].sqlName,
                  },
              }
            : {}),
        include: Object.fromEntries(
            Object.entries("include" in query ? (query.include ?? {}) : {}).map(
                ([name, include]) => [name, describe(include)],
            ),
        ),
        relations: Object.fromEntries(
            Object.entries(query.relations ?? {}).map(([name, relation]) => [
                name,
                describe(relation),
            ]),
        ),
    };
}

/** The inspection of a dataflow. */
export interface DataflowInspection {
    /** The held log position. */
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
        /** The held rows. */
        readonly members: number;
        /** The window rows or groups the pipeline knows. */
        readonly size: number;
    }[];
}
