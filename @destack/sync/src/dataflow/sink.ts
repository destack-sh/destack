import { Key, TABLE, type Row, type Table, type LogPosition, type Scalar } from "@destack/db";
import { aligned, zip } from "@destack/schema";
import type { Node } from "../query/node.ts";
import type { Page, ResultChange, RowChange } from "../query/page.ts";
import type { Audience } from "../feed/audience.ts";
import type { Cache } from "./view.ts";
import type { Run, Touch } from "./run.ts";
import type { Result } from "./tally.ts";

/** What the subscriber has: each row by its holder count, and each shown group. */
export class Sink {
    /** How many members and chains keep each row, by key. */
    readonly #rows = new Map<string, number>();
    /** The aggregate groups the subscriber has, by query and group. */
    readonly #groups = new Set<string>();

    /** Decide whether the subscriber has a row. */
    has(key: string): boolean {
        return this.#rows.has(key);
    }

    /** Note a row a run touched, and whether the subscriber had it before. */
    touch(run: Run, table: Table, key: string, row: Row | null): void {
        const touched = run.touched.get(key);
        if (touched === undefined) {
            run.touched.set(key, { table, row, wasPresent: this.#rows.has(key) });
        } else if (row !== null) {
            touched.row = row;
        }
    }

    /** Count a row's holders up or down. */
    retain(run: Run, table: Table, key: string, row: Row | null, step: 1 | -1): void {
        this.touch(run, table, key, row);
        const count = (this.#rows.get(key) ?? 0) + step;
        if (count === 0) {
            this.#rows.delete(key);
        } else {
            this.#rows.set(key, count);
        }
    }

    /** Record a shown group's values while it has rows, or its leaving. */
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

    /** Send the rows a run moved into, out of or within the subscriber's set. */
    async emit(run: Run): Promise<void> {
        // read the present rows no holder passed at the run's position
        const unread = new Map<Table, [string, Touch][]>();
        for (const entry of run.touched) {
            const [key, touched] = entry;
            if (touched.row === null && this.#rows.has(key)) {
                unread.set(touched.table, [...(unread.get(touched.table) ?? []), entry]);
            }
        }
        for (const [table, entries] of unread) {
            const rows = await run.view.keyed(
                table,
                entries.map(([key]) => Key.parse(table, key)),
            );
            for (const [[key, touched], row] of zip(entries, rows)) {
                // refuse a present row missing at the position
                if (row === null) {
                    throw new TypeError(
                        `subscriber has ${key}, missing at ${run.view.position.sequence}`,
                    );
                }
                touched.row = row;
            }
        }

        // decide each touched row's move
        for (const [key, touched] of run.touched) {
            const isPresent = this.#rows.has(key);
            const table = touched.table;
            if (run.walk === "rebuild") {
                continue;
            }
            // send a row entering the set
            else if (isPresent && !touched.wasPresent) {
                run.patch.rows.set(key, {
                    table,
                    row: touchedRow(key, touched),
                    operation: "insert",
                });
            }
            // send the key of a row leaving it
            else if (!isPresent && touched.wasPresent) {
                run.patch.rows.set(key, {
                    table,
                    row: Key.parse(table, key),
                    operation: "delete",
                });
            }
            // send a present row the run changed
            else if (isPresent && run.changed.has(key)) {
                run.patch.rows.set(key, {
                    table,
                    row: touchedRow(key, touched),
                    operation: "update",
                });
            }
        }
        run.touched.clear();
    }

    /** Forget everything the subscriber has. */
    forget(): void {
        this.#rows.clear();
        this.#groups.clear();
    }
}

/** The row and group decisions of a page. */
export class Patch {
    /** The row decisions, by row key. */
    readonly rows = new Map<string, Decision>();
    /** The group results, by query and group. */
    readonly results = new Map<string, ResultChange>();
    /** The groups the page shows first. */
    readonly #entered = new Set<string>();

    /** The decisions of the patch. */
    get size(): number {
        return this.rows.size + this.results.size;
    }

    /** Record a group's new values, or its leaving. */
    result(
        node: Node,
        group: Readonly<Record<string, Scalar>>,
        tally: Result | undefined,
        isNew = false,
    ): void {
        // take a group the page showed first out again
        const key = `${node.name}${JSON.stringify(group)}`;
        const isPresent = tally !== undefined && !tally.isEmpty;
        if (!isPresent && this.#entered.delete(key)) {
            this.results.delete(key);
            return;
        } else if (isNew) {
            this.#entered.add(key);
        }

        // record the values or the leaving
        const parts = isPresent ? tally.parts() : {};
        this.results.set(key, {
            query: node.name,
            group: { ...group },
            values: isPresent ? tally.values() : null,
            ...(isPresent ? { rows: tally.rows } : {}),
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

    /** Encode the patch into a page at a position without concealed columns. */
    async page(
        position: LogPosition,
        audience: Audience,
        cache: Cache,
        flags: { readonly reset: boolean; readonly complete: boolean },
    ): Promise<Page> {
        // group the decisions by table
        const byTable = new Map<Table, [string, Decision][]>();
        for (const [key, decision] of this.rows) {
            const decisions = byTable.get(decision.table) ?? [];
            decisions.push([key, decision]);
            byTable.set(decision.table, decisions);
        }

        // encode each table's rows
        const changes: RowChange[] = [];
        for (const [table, decisions] of byTable) {
            const kept = decisions.filter(([, decision]) => decision.operation !== "delete");
            const hidden = await audience.conceals(
                table,
                kept.map(([, decision]) => decision.row),
                position,
            );
            let index = 0;
            for (const [key, decision] of decisions) {
                changes.push(
                    decision.operation === "delete"
                        ? {
                              table: table[TABLE].sqlName,
                              operation: "delete",
                              row: table[TABLE].encode(keyOf(table, decision.row)),
                          }
                        : encode(table, key, decision, aligned(hidden, index++), position, cache),
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

    /** Build the patch from one collection to another at one position. */
    static difference(
        before: Patch,
        after: Patch,
        nodes: readonly Node[],
        isResent: (table: Table) => boolean,
    ): Patch {
        // send what only the new collection has or may read differently
        const patch = new Patch();
        for (const [key, decision] of after.rows) {
            if (!before.rows.has(key)) {
                patch.rows.set(key, decision);
            } else if (isResent(decision.table)) {
                patch.rows.set(key, { ...decision, operation: "update" });
            }
        }

        // let go of what only the old collection had
        for (const [key, decision] of before.rows) {
            if (!after.rows.has(key)) {
                patch.rows.set(key, { ...decision, operation: "delete" });
            }
        }

        // replace every group
        for (const node of nodes) {
            if (node.aggregate !== undefined) {
                patch.clear(node);
            }
        }
        for (const [key, result] of after.results) {
            patch.results.set(key, result);
        }

        return patch;
    }
}

/** One row decision of a page. */
export interface Decision {
    /** The row's table. */
    readonly table: Table;
    /** The row, or its key once let go. */
    readonly row: Row;
    /** Whether the row enters, changes or leaves. */
    readonly operation: RowChange["operation"];
}

/** Encode a selected row's logged columns at a position without concealed ones. */
function encode(
    table: Table,
    key: string,
    decision: Decision,
    concealed: readonly string[],
    position: LogPosition,
    cache: Cache,
): RowChange {
    return cache.encodings.share(
        position.sequence,
        `encoded:${key}:${decision.operation}:${concealed.join(",")}`,
        () => ({
            table: table[TABLE].sqlName,
            operation: decision.operation,
            row: table[TABLE].encodeLogged(decision.row, concealed),
            ...(concealed.length === 0 ? {} : { concealed: [...concealed] }),
        }),
    );
}

/** Read the values of a row's key. */
function keyOf(table: Table, row: Row): Row {
    return Object.fromEntries(
        table[TABLE].key.map((column) => [column, Key.value(table, row, column)]),
    );
}

/** Read the row a touched present row keeps at the run's position. */
function touchedRow(key: string, touched: Touch): Row {
    if (touched.row === null) {
        throw new TypeError(`present row ${key} is sent before it is read`);
    }

    return touched.row;
}
