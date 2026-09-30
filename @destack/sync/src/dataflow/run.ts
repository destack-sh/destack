import type { Row, Table } from "@destack/db";
import type { Change } from "@destack/db/log";
import type { Audience } from "../feed/audience.ts";
import type { Filter } from "./filter.ts";
import type { Pipeline } from "./pipeline.ts";
import { Patch } from "./sink.ts";
import type { View } from "./view.ts";

/** One step of a dataflow: the database as of one position, its decisions, and the page it fills. */
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
    readonly patch = new Patch();
    /** The rows the audience decides again, by table and key. */
    readonly affected = new Map<Table, Map<string, Row>>();
    /** The keys of rows whose readable columns or access the run changed. */
    readonly changed = new Set<string>();
    /** What each pipeline has to do. */
    readonly work = new Map<Pipeline, Work>();
    /** The rows whose holding the run touched, by key. */
    readonly touched = new Map<string, Touch>();
    /** The rows the run's pipelines decided. */
    decided = 0;
    /** The pending visibility decision of each row. */
    readonly #deciding = new WeakMap<Row, Promise<void>>();
    /** Each row's decided visibility. */
    readonly #visible = new WeakMap<Row, boolean>();
    /** The rows each relation-reading filter resolved in this run. */
    readonly #resolved = new Map<Filter, WeakMap<Row, Row>>();

    /** Start a run as of a view. */
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

    /** Decide the visibility of many rows of a table at once. */
    async decide(table: Table, rows: readonly Row[]): Promise<void> {
        // ask the audience about the undecided rows in one call
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

        // wait for every row's decision
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

    /** Keep the visible rows of a table. */
    async seen(table: Table, rows: readonly Row[]): Promise<Row[]> {
        await this.decide(table, rows);

        return rows.filter((row) => this.isVisible(row));
    }

    /** Read a pipeline's work in this run. */
    workOf(pipeline: Pipeline): Work {
        let work = this.work.get(pipeline);
        if (work === undefined) {
            work = { opened: new Map(), closed: new Set(), dirty: new Map() };
            this.work.set(pipeline, work);
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

/**
 * How a hydration treats its rows.
 *
 * A snapshot sends every row and result in pages.
 * A rebuild restores the subscriber's state, sending only results.
 * A collection gathers every row and result at once.
 */
export type Walk = "snapshot" | "rebuild" | "collect";

/** What a run has one pipeline do. */
export interface Work {
    /** The newly named partitions, with the value naming each. */
    readonly opened: Map<string, unknown>;
    /** The partitions no held parent row names. */
    readonly closed: Set<string>;
    /** The rows to decide again, by key, absent once gone. */
    readonly dirty: Map<string, Row | undefined>;
}

/** A changed row's first and last image in a run. */
export interface Transition {
    /** The row before the run's first change, absent when inserted. */
    readonly before?: Row;
    /** The row after the run's last change, absent when deleted. */
    readonly after?: Row;
}

/** A row whose holding a run touched. */
export interface Touch {
    /** The row's table. */
    readonly table: Table;
    /** The row at the position, absent until a holder passes it or the sink reads it. */
    row: Row | undefined;
    /** Whether the subscriber held the row before the run. */
    readonly wasHeld: boolean;
}

/** The cost of one run. */
export interface RunCost {
    /** The applied changes. */
    readonly changes: number;
    /** The database operations during the run. */
    readonly operations: number;
    /** The rows the pipelines decided. */
    readonly decided: number;
    /** The row and group decisions the page carries. */
    readonly sent: number;
    /** The run time, in milliseconds. */
    readonly milliseconds: number;
}
