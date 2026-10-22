import type { ColumnValue, Row, Table, Change } from "@destack/db";
import type { Audience } from "../feed/audience.ts";
import type { Filter } from "./filter.ts";
import type { Pipeline } from "./pipeline.ts";
import { Patch } from "./sink.ts";
import type { View } from "./view.ts";

/** One step of a dataflow: the database at one position, its decisions, and its page. */
export class Run {
    /** The database as of the position. */
    readonly view: View;
    /** Who the dataflow serves. */
    readonly #audience: Audience;
    /** How a hydration treats its rows, absent for a step. */
    readonly walk: Walk | undefined;
    /** The committed changes the step applies, in commit order. */
    readonly changes: readonly Change[];
    /** The page the run fills. */
    readonly patch: Patch;
    /** The rows the audience decides again, by table and key. */
    readonly affected: Map<Table, Map<string, Row>>;
    /** The keys of rows whose readable columns or access the run changed. */
    readonly changed: Set<string>;
    /** What each pipeline has to do. */
    readonly pipelines: Map<Pipeline, Work>;
    /** The rows whose presence the run touched, by key. */
    readonly touched: Map<string, Touch>;
    /** The rows the run's pipelines decided. */
    decided: number;
    /** The pending visibility decision of each row, by the permission it is decided under. */
    readonly #deciding: Map<string | undefined, WeakMap<Row, Promise<void>>>;
    /** Each row's decided visibility, by the permission it is decided under. */
    readonly #visible: Map<string | undefined, WeakMap<Row, boolean>>;
    /** The rows each relation-reading filter resolved in this run. */
    readonly #resolved: Map<Filter, WeakMap<Row, Row>>;

    /** Start a run as of a view. */
    constructor(
        view: View,
        audience: Audience,
        walk: Walk | undefined,
        changes: readonly Change[] = [],
    ) {
        // start with empty state
        this.patch = new Patch();
        this.affected = new Map<Table, Map<string, Row>>();
        this.changed = new Set<string>();
        this.pipelines = new Map<Pipeline, Work>();
        this.touched = new Map<string, Touch>();
        this.decided = 0;

        // start the rest of the state empty
        this.#deciding = new Map<string | undefined, WeakMap<Row, Promise<void>>>();
        this.#visible = new Map<string | undefined, WeakMap<Row, boolean>>();
        this.#resolved = new Map<Filter, WeakMap<Row, Row>>();

        // store the view, the audience and the changes
        this.view = view;
        this.#audience = audience;
        this.walk = walk;
        this.changes = changes;
    }

    /** Decide many rows' visibility at once, under a query's permission too. */
    async decide(table: Table, rows: readonly Row[], permission?: string): Promise<void> {
        // ask the audience about the undecided rows in one call
        const deciding = Run.#under(this.#deciding, permission);
        const visible = Run.#under(this.#visible, permission);
        const pending = rows.filter((row) => !deciding.has(row));
        if (pending.length > 0) {
            const decided = this.#audience
                .visible(table, pending, this.view.position, permission)
                .then((shown) => {
                    for (const [index, row] of pending.entries()) {
                        visible.set(row, shown.has(index));
                    }
                });
            for (const row of pending) {
                deciding.set(row, decided);
            }
        }

        // wait for every row's decision
        await Promise.all(
            new Set(
                rows.map((row) => {
                    const decision = deciding.get(row);
                    if (decision === undefined) {
                        throw new TypeError("a row waits for a decision nobody asked for");
                    }

                    return decision;
                }),
            ),
        );
    }

    /** Read a decided row's visibility under a permission. */
    isVisible(row: Row, permission?: string): boolean {
        const visible = Run.#under(this.#visible, permission).get(row);
        if (visible === undefined) {
            throw new TypeError("a row's visibility is read before it is decided");
        }

        return visible;
    }

    /** Filter the visible rows of a table, under a query's permission too. */
    async seen(table: Table, rows: readonly Row[], permission?: string): Promise<Row[]> {
        await this.decide(table, rows, permission);

        return rows.filter((row) => this.isVisible(row, permission));
    }

    /** Read the decisions under a permission, the listing alone without one. */
    static #under<Value>(
        decisions: Map<string | undefined, WeakMap<Row, Value>>,
        permission: string | undefined,
    ): WeakMap<Row, Value> {
        // add the permission's decisions on first use
        let under = decisions.get(permission);
        if (under === undefined) {
            under = new WeakMap<Row, Value>();
            decisions.set(permission, under);
        }

        return under;
    }

    /** Read a pipeline's work in this run. */
    work(pipeline: Pipeline): Work {
        let work = this.pipelines.get(pipeline);
        if (work === undefined) {
            work = { opened: new Map(), closed: new Set(), dirty: new Map() };
            this.pipelines.set(pipeline, work);
        }

        return work;
    }

    /** Read the rows a relation-reading filter resolved in this run. */
    resolvedBy(filter: Filter): WeakMap<Row, Row> {
        let resolved = this.#resolved.get(filter);
        if (resolved === undefined) {
            resolved = new WeakMap();
            this.#resolved.set(filter, resolved);
        }

        return resolved;
    }
}

/** How a hydration treats its rows. */
export type Walk = "snapshot" | "rebuild" | "collect";

/** What a run has one pipeline do. */
export interface Work {
    /** The new partitions, with the value of each. */
    readonly opened: Map<string, ColumnValue>;
    /** The partitions no selected parent row references. */
    readonly closed: Set<string>;
    /** The rows to decide again, by key, null once gone. */
    readonly dirty: Map<string, Row | null>;
}

/** A changed row's first and last image in a run. */
export interface Transition {
    /** The row before the run's first change, null when inserted. */
    readonly before: Row | null;
    /** The row after the run's last change, null when deleted. */
    readonly after: Row | null;
}

/** A row whose presence a run touched. */
export interface Touch {
    /** The row's table. */
    readonly table: Table;
    /** The row at the position, null until a holder passes it or the sink reads it. */
    row: Row | null;
    /** Whether the subscriber had the row before the run. */
    readonly wasPresent: boolean;
}

/** The cost of one run. */
export interface RunCost {
    /** The applied changes. */
    readonly changes: number;
    /** The database operations during the run. */
    readonly operations: number;
    /** The rows the pipelines decided. */
    readonly decided: number;
    /** The row and group decisions of the page. */
    readonly sent: number;
    /** The run time, in milliseconds. */
    readonly milliseconds: number;
}
