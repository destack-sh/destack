import { encodeColumns, encodeRow, Key, TABLE, type Row, type Table } from "@destack/db";
import type { LogPosition } from "@destack/db/log";
import type { Scalar } from "@destack/db/query";
import type { Node } from "../query/node.ts";
import type { QueryPage, ResultChange, RowChange } from "../query/page.ts";
import type { Audience } from "../feed/audience.ts";
import type { Cache } from "./view.ts";
import type { Run } from "./run.ts";
import type { Result } from "./tally.ts";

/**
 * What the subscriber holds: each row by how many members and chains hold it, and each aggregate group it is shown.
 *
 * A row enters when its first holder holds it and leaves with its last, so rows several nodes hold travel once.
 */
export class Sink {
    /** How many members and chains hold each row, by key. */
    readonly #rows = new Map<string, number>();
    /** The aggregate groups the subscriber holds, by query and group. */
    readonly #groups = new Set<string>();

    /** Decide whether the subscriber holds a row. */
    holds(key: string): boolean {
        return this.#rows.has(key);
    }

    /** Note a row a run touched, as it is at the position when known, and whether the subscriber held it before. */
    touch(run: Run, table: Table, key: string, row: Row | undefined): void {
        const touched = run.touched.get(key);
        if (touched === undefined) {
            run.touched.set(key, {
                table,
                row: row ?? (Key.parse(table, key) as Row),
                wasHeld: this.#rows.has(key),
            });
        } else if (row !== undefined) {
            touched.row = row;
        }
    }

    /** Count a row one holder more or less, noting how the run touched it. */
    retain(run: Run, table: Table, key: string, row: Row | undefined, step: 1 | -1): void {
        this.touch(run, table, key, row);
        const count = (this.#rows.get(key) ?? 0) + step;
        if (count === 0) {
            this.#rows.delete(key);
        } else {
            this.#rows.set(key, count);
        }
    }

    /** Record an aggregate group's values while its tally holds rows and the subscriber is shown it, or its leaving once it is not. */
    result(
        run: Run,
        node: Node,
        group: Readonly<Record<string, Scalar>>,
        tally: Result | undefined,
        isShown = true,
    ): void {
        const key = `${node.name}${JSON.stringify(group)}`;
        if (tally !== undefined && !tally.isEmpty && isShown) {
            const isNew = !this.#groups.has(key);
            this.#groups.add(key);
            run.patch.result(node, group, tally, isNew);
        } else if (this.#groups.delete(key)) {
            run.patch.result(node, group, undefined);
        }
    }

    /** Send what a run changed: rows entering, leaving, or changing within what the subscriber holds; a rebuild sends none. */
    emit(run: Run): void {
        for (const [key, touched] of run.touched) {
            const isHeld = this.#rows.has(key);
            const decision = (operation: RowChange["operation"]) =>
                run.patch.rows.set(key, { table: touched.table, row: touched.row, operation });
            if (run.walk === "rebuild") {
                continue;
            } else if (isHeld && !touched.wasHeld) {
                decision("insert");
            } else if (!isHeld && touched.wasHeld) {
                decision("delete");
            } else if (isHeld && run.changed.has(key)) {
                decision("update");
            }
        }
        run.touched.clear();
    }

    /** Forget everything the subscriber holds. */
    forget(): void {
        this.#rows.clear();
        this.#groups.clear();
    }
}

/** What a page decides: the rows the subscriber holds or lets go of, and the aggregate groups it updates. */
export class Patch {
    /** The row decisions, by row key. */
    readonly rows = new Map<string, Decision>();
    /** The group results, by query and group. */
    readonly results = new Map<string, ResultChange>();
    /** The groups the page shows first, which leaving within it takes out of the page. */
    readonly #entered = new Set<string>();

    /** The decisions the patch holds. */
    get size(): number {
        return this.rows.size + this.results.size;
    }

    /** Record a group's new values, noting whether the subscriber held it before, or its leaving, which takes out a group the page showed first. */
    result(
        node: Node,
        group: Readonly<Record<string, Scalar>>,
        tally: Result | undefined,
        isNew = false,
    ): void {
        // take a group the page showed first out of it again
        const key = `${node.name}${JSON.stringify(group)}`;
        const isHeld = tally !== undefined && !tally.isEmpty;
        if (!isHeld && this.#entered.delete(key)) {
            this.results.delete(key);
            return;
        } else if (isNew) {
            this.#entered.add(key);
        }

        // record the group's values, or its leaving
        const parts = isHeld ? tally.parts() : {};
        this.results.set(key, {
            query: node.name,
            group: { ...group },
            values: isHeld ? tally.values() : null,
            ...(isHeld ? { rows: tally.rows } : {}),
            ...(Object.keys(parts).length === 0 ? {} : { parts }),
        });
    }

    /** Forget the decisions a page sent. */
    drain(): void {
        this.rows.clear();
        this.results.clear();
        this.#entered.clear();
    }

    /** Record that the subscriber lets go of every group of a query. */
    clear(node: Node): void {
        this.results.set(node.name, { query: node.name, group: null, values: null });
    }

    /**
     * Turn the patch into a page at a position, encoding held rows without the columns the audience conceals and naming them.
     *
     * Each row encodes once for every subscriber sending it alike at the position.
     */
    async page(
        position: LogPosition,
        audience: Audience,
        cache: Cache,
        flags: { readonly reset: boolean; readonly complete: boolean },
    ): Promise<QueryPage> {
        // group the decisions by table
        const byTable = new Map<Table, [string, Decision][]>();
        for (const [key, decision] of this.rows) {
            const decisions = byTable.get(decision.table) ?? [];
            decisions.push([key, decision]);
            byTable.set(decision.table, decisions);
        }

        // encode each table's rows, deciding the concealed columns of its held rows at once
        const changes: RowChange[] = [];
        for (const [table, decisions] of byTable) {
            const held = decisions.filter(([, decision]) => decision.operation !== "delete");
            const hidden = await audience.conceals(
                table,
                held.map(([, decision]) => decision.row),
                position,
            );
            let index = 0;
            for (const [key, decision] of decisions) {
                changes.push(
                    decision.operation === "delete"
                        ? {
                              table: table[TABLE].sqlName,
                              operation: "delete",
                              row: encodeRow(table, keyOf(table, decision.row)),
                          }
                        : encode(table, key, decision, hidden[index++]!, position, cache),
                );
            }
        }

        return {
            ...flags,
            changes,
            ...(this.results.size === 0 ? {} : { results: [...this.results.values()] }),
            position,
        };
    }

    /**
     * Decide what moves a subscriber from what one collection held to what another holds, at one position.
     *
     * Rows of tables whose readable columns may differ are sent again whenever both hold them.
     */
    static difference(
        held: Patch,
        holding: Patch,
        nodes: readonly Node[],
        isResent: (table: Table) => boolean,
    ): Patch {
        // hold what only the new collection holds, and what may read differently
        const patch = new Patch();
        for (const [key, decision] of holding.rows) {
            if (!held.rows.has(key)) {
                patch.rows.set(key, decision);
            } else if (isResent(decision.table)) {
                patch.rows.set(key, { ...decision, operation: "update" });
            }
        }

        // let go of what only the earlier collection held
        for (const [key, decision] of held.rows) {
            if (!holding.rows.has(key)) {
                patch.rows.set(key, { ...decision, operation: "delete" });
            }
        }

        // replace every aggregate group with the new collection's
        for (const node of nodes) {
            if (node.aggregate !== undefined) {
                patch.clear(node);
            }
        }
        for (const [key, result] of holding.results) {
            patch.results.set(key, result);
        }

        return patch;
    }
}

/** One row a page decides: held with its values, or let go. */
export interface Decision {
    /** The row's table. */
    readonly table: Table;
    /** The row, or its key alone once let go. */
    readonly row: Row;
    /** Whether the row enters, changes or leaves. */
    readonly operation: RowChange["operation"];
}

/** Encode a held row's logged columns at a position, leaving out and naming the concealed ones, once for every subscriber sending it alike. */
function encode(
    table: Table,
    key: string,
    decision: Decision,
    concealed: readonly string[],
    position: LogPosition,
    cache: Cache,
): RowChange {
    return cache.share(
        position.sequence,
        `encoded:${key}:${decision.operation}:${concealed.join(",")}`,
        () => ({
            table: table[TABLE].sqlName,
            operation: decision.operation,
            row: encodeColumns(Object.entries(table[TABLE].logged), decision.row, concealed),
            ...(concealed.length === 0 ? {} : { concealed: [...concealed] }),
        }),
    );
}

/** Read the values of a row's key. */
function keyOf(table: Table, row: Row): Row {
    return Object.fromEntries(table[TABLE].key.map((column) => [column, row[column]]));
}
