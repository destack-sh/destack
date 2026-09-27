import type { Row, Table } from "@destack/db";
import type { Change } from "@destack/db/log";
import type { Audience } from "../feed/audience.ts";
import type { Filter } from "./filter.ts";
import type { Pipeline } from "./pipeline.ts";
import { Patch } from "./sink.ts";
import type { View } from "./view.ts";

/**
 * One step of a dataflow: the database as of one position, the decisions made there, and the page the step fills.
 *
 * Every decision reads the view, and each row's visibility is decided once, in batches.
 */
export class Run {
    /** The database as of the position decided. */
    readonly view: View;
    /** Who the dataflow serves. */
    readonly #audience: Audience;
    /** How a hydration treats the rows it holds, absent for a step of changes. */
    readonly walk: Walk | undefined;
    /** The committed changes the step applies, in commit order, none for a hydration. */
    readonly changes: readonly Change[];
    /** The page the run fills. */
    readonly patch = new Patch();
    /** The rows the audience decides again, by table and key, as they are at the position. */
    readonly affected = new Map<Table, Map<string, Row>>();
    /** The keys of rows whose readable columns or access the run changed. */
    readonly changed = new Set<string>();
    /** What each pipeline has to do. */
    readonly work = new Map<Pipeline, Work>();
    /** The rows whose holding the run touched, by key. */
    readonly touched = new Map<string, Touch>();
    /** The rows the run's pipelines decided, which measure its work. */
    decided = 0;
    /** The decision of each row's visibility, shared by the rows one call decides. */
    readonly #deciding = new WeakMap<Row, Promise<void>>();
    /** Each row's visibility once decided, by row. */
    readonly #visible = new WeakMap<Row, boolean>();
    /** The rows each filter reading relations resolved in this run, whose values depend on the run's measures. */
    readonly #resolved = new Map<Filter, WeakMap<Row, Row>>();

    /** Start a run as of a view for an audience: a hydration, or a step of changes. */
    constructor(
        view: View,
        audience: Audience,
        walk: Walk | undefined,
        changes: readonly Change[] = [],
    ) {
        // hold the view, the audience and the changes
        this.view = view;
        this.#audience = audience;
        this.walk = walk;
        this.changes = changes;
    }

    /** Decide the visibility of many rows of a table at once, so that later decisions read it synchronously. */
    async decide(table: Table, rows: readonly Row[]): Promise<void> {
        // ask the audience about the rows no decision covers yet, in one call
        const pending = rows.filter((row) => !this.#deciding.has(row));
        if (pending.length > 0) {
            const decided = this.#audience
                .admits(table, pending, this.view.position)
                .then((held) => {
                    for (const [index, row] of pending.entries()) {
                        this.#visible.set(row, held.has(index));
                    }
                });
            for (const row of pending) {
                this.#deciding.set(row, decided);
            }
        }

        // wait for every row's decision, made here or by an earlier call
        await Promise.all(new Set(rows.map((row) => this.#deciding.get(row)!)));
    }

    /** Read a decided row's visibility. */
    isVisible(row: Row): boolean {
        const visible = this.#visible.get(row);
        if (visible === undefined) {
            throw new TypeError("a row's visibility is read before it is decided");
        }

        return visible;
    }

    /** Keep the rows of a table the audience sees, deciding them at once. */
    async seen(table: Table, rows: readonly Row[]): Promise<Row[]> {
        await this.decide(table, rows);

        return rows.filter((row) => this.isVisible(row));
    }

    /** Read what a run has a pipeline do, starting with nothing. */
    workOf(pipeline: Pipeline): Work {
        let work = this.work.get(pipeline);
        if (work === undefined) {
            work = { opened: new Map(), closed: new Set(), dirty: new Map() };
            this.work.set(pipeline, work);
        }

        return work;
    }

    /** Read the rows a filter reading relations resolved in this run. */
    resolvedBy(filter: Filter): WeakMap<Row, Row> {
        let resolved = this.#resolved.get(filter);
        if (resolved === undefined) {
            resolved = new WeakMap();
            this.#resolved.set(filter, resolved);
        }

        return resolved;
    }
}

/**
 * How a hydration of the queries treats the rows it holds.
 *
 * A snapshot sends every row and result in pages.
 * A rebuild restores what the subscriber holds as of its position, sending only results.
 * A collection gathers every row and result at once, for a reshape to compare.
 */
export type Walk = "snapshot" | "rebuild" | "collect";

/** What a run has one pipeline do: partitions to fill and to empty, and rows to decide again. */
export interface Work {
    /** The partitions held parent rows newly name, by name, with the value naming each. */
    readonly opened: Map<string, unknown>;
    /** The partitions no held parent row names any more. */
    readonly closed: Set<string>;
    /** The rows to decide again, by key: as they are at the position, absent once gone. */
    readonly dirty: Map<string, Row | undefined>;
}

/** A changed row's first and last image in a run. */
export interface Transition {
    /** The row before the run's first change of it, absent when the run inserted it. */
    readonly before?: Row;
    /** The row after the run's last change of it, absent when the run deleted it. */
    readonly after?: Row;
}

/** A row whose holding a run touched: as it is at the position, and whether the subscriber held it before. */
export interface Touch {
    /** The row's table. */
    readonly table: Table;
    /** The row as it is at the position, or its key once gone. */
    row: Row;
    /** Whether the subscriber held the row before the run. */
    readonly wasHeld: boolean;
}

/** What one run cost: the changes it applied, the operations the database ran meanwhile, the rows it decided, the decisions it sent, and its time. */
export interface RunCost {
    /** The committed changes applied, none for a hydration. */
    readonly changes: number;
    /** The operations the database connection ran during the run, other runs' among them when concurrent. */
    readonly operations: number;
    /** The rows the pipelines decided. */
    readonly decided: number;
    /** The row and group decisions the run's page carries. */
    readonly sent: number;
    /** The time the run took, in milliseconds. */
    readonly milliseconds: number;
}
