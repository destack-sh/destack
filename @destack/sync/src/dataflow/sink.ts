import { Key, TABLE, type Row, type Table, type LogPosition, type Scalar } from "@destack/db";
import { nth, zip } from "@destack/schema";
import type { Node } from "../query/node.ts";
import type { Page, ResultChange, RowChange } from "../query/page.ts";
import type { Audience } from "../feed/audience.ts";
import type { Cache } from "./view.ts";
import type { Run, Touch } from "./run.ts";
import type { Result } from "./tally.ts";

/** The offset bases of the two FNV-1a passes a row's image digest combines. */
const FNV_OFFSETS = [2_166_136_261, 3_323_878_721] as const;

/** FNV-1a's 32-bit prime. */
const FNV_PRIME = 16_777_619;

/** The bits the second pass adds below the first's, keeping the digest within 53 exact bits. */
const LOW_BITS = 21;

/** What the subscriber has: each row with its table and count, and each shown group. */
export class Sink {
    /** Each row's table and how many members and chains retain it, by key. */
    readonly #rows: Map<string, { readonly table: Table; readonly count: number }>;
    /** The aggregate groups the subscriber has, by query and group. */
    readonly #groups: Set<string>;

    /** Start empty. */
    constructor() {
        // start with empty state
        this.#rows = new Map<string, { readonly table: Table; readonly count: number }>();
        this.#groups = new Set<string>();
    }

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

    /** Count up or down how many members and chains retain a row. */
    retain(run: Run, table: Table, key: string, row: Row | null, step: 1 | -1): void {
        this.touch(run, table, key, row);
        const count = (this.#rows.get(key)?.count ?? 0) + step;
        if (count === 0) {
            this.#rows.delete(key);
        } else {
            this.#rows.set(key, { table, count });
        }
    }

    /** List the rows the subscriber has, each key with its table. */
    rows(): Map<string, Table> {
        return new Map([...this.#rows].map(([key, { table }]) => [key, table]));
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
        // read the present rows no member or chain passed at the run's position
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
                        `subscriber has row ${key}, but it is missing at ${run.view.position.sequence}`,
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
                    row: Sink.#touchedRow(key, touched),
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
                    row: Sink.#touchedRow(key, touched),
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

    /** Read the row a touched present row has at the run's position. */
    static #touchedRow(key: string, touched: Touch): Row {
        if (touched.row === null) {
            throw new TypeError(`cannot send present row ${key} before reading it`);
        }

        return touched.row;
    }
}

/** The row and group decisions of a page. */
export class Patch {
    /** The row decisions, by row key. */
    readonly rows: Map<string, Decision>;
    /** The group results, by query and group. */
    readonly results: Map<string, ResultChange>;
    /** The groups the page shows first. */
    readonly #entered: Set<string>;

    /** Start empty. */
    constructor() {
        // start with empty state
        this.rows = new Map<string, Decision>();
        this.results = new Map<string, ResultChange>();
        this.#entered = new Set<string>();
    }

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
        const averages = isPresent ? tally.averages() : {};
        this.results.set(key, {
            query: node.name,
            group: { ...group },
            values: isPresent ? tally.values() : null,
            ...(isPresent ? { rows: tally.rows } : {}),
            ...(Object.keys(averages).length === 0 ? {} : { averages }),
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

    /** Encode the patch into a page without concealed columns or rows held as they are. */
    async page(
        position: LogPosition,
        audience: Audience,
        cache: Cache,
        record: ViewRecord,
        flags: { readonly reset: boolean; readonly complete: boolean },
    ): Promise<Page> {
        // start over on a reset, which replaces everything the subscriber holds
        if (flags.reset) {
            record.clear();
        }

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
            const present = decisions.filter(([, decision]) => decision.operation !== "delete");
            const hidden = await audience.conceals(
                table,
                present.map(([, decision]) => decision.row),
                position,
            );
            let index = 0;
            for (const [key, decision] of decisions) {
                // let go of a deleted row
                if (decision.operation === "delete") {
                    record.forget(key);
                    changes.push({
                        table: table[TABLE].sqlName,
                        operation: "delete",
                        row: table[TABLE].encode(Patch.#key(table, decision.row)),
                    });
                    continue;
                }

                // send a row whose image differs from the one the subscriber holds
                const encoded = Patch.#encode(
                    table,
                    key,
                    decision,
                    nth(hidden, index++),
                    position,
                    cache,
                );
                if (record.update(key, encoded.image)) {
                    changes.push(encoded.change);
                }
            }
        }

        return {
            ...flags,
            changes,
            ...(this.results.size === 0 ? {} : { results: [...this.results.values()] }),
            position,
        };
    }

    /** Build the patch from the rows a subscriber has to a collection at one position. */
    static difference(
        before: ReadonlyMap<string, Table>,
        after: Patch,
        nodes: readonly Node[],
        isResent: (table: Table, key: string) => boolean,
    ): Patch {
        // send what only the new collection has or may read differently
        const patch = new Patch();
        for (const [key, decision] of after.rows) {
            if (!before.has(key)) {
                patch.rows.set(key, decision);
            } else if (isResent(decision.table, key)) {
                patch.rows.set(key, { ...decision, operation: "update" });
            }
        }

        // let go of what only the subscriber had
        for (const [key, table] of before) {
            if (!after.rows.has(key)) {
                patch.rows.set(key, { table, row: Key.parse(table, key), operation: "delete" });
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

    /** Encode a selected row's logged columns without concealed ones, and its image. */
    static #encode(
        table: Table,
        key: string,
        decision: Decision,
        concealed: readonly string[],
        position: LogPosition,
        cache: Cache,
    ): Encoding {
        return cache.encodings.share(
            position.sequence,
            `encoded:${key}:${decision.operation}:${concealed.join(",")}`,
            () => {
                // encode the row once for every subscriber, digesting the image they hold
                const row = table[TABLE].encodeLogged(decision.row, concealed);
                const change: RowChange = {
                    table: table[TABLE].sqlName,
                    operation: decision.operation,
                    row,
                    ...(concealed.length === 0 ? {} : { concealed: [...concealed] }),
                };

                return { change, image: ViewRecord.digest(JSON.stringify([row, concealed])) };
            },
        );
    }

    /** Read the values of a row's key. */
    static #key(table: Table, row: Row): Row {
        return Object.fromEntries(
            table[TABLE].key.map((column) => [column, Key.value(table, row, column)]),
        );
    }
}

/** A row's change as a page sends it, with the digest of the image it shows. */
export interface Encoding {
    /** The change. */
    readonly change: RowChange;
    /** The digest of the row's image: its logged columns and what they conceal. */
    readonly image: number;
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

/** The images of the rows a subscriber holds, by key, as digests: a client view record. */
export class ViewRecord {
    /** The digest of each held row's image, by key. */
    readonly #images = new Map<string, number>();

    /** Record a row's image, reporting whether it differs from the one held. */
    update(key: string, image: number): boolean {
        // compare the image's digest with the held one
        if (this.#images.get(key) === image) {
            return false;
        }
        this.#images.set(key, image);

        return true;
    }

    /** Forget a row the subscriber let go of. */
    forget(key: string): void {
        this.#images.delete(key);
    }

    /** Forget every row, as a reset replaces them. */
    clear(): void {
        this.#images.clear();
    }

    /** Digest a text in 53 bits: two 32-bit FNV-1a passes, the second below the first. */
    static digest(text: string): number {
        // fold each code unit into both passes with exact 32-bit products
        let high: number = FNV_OFFSETS[0];
        let low: number = FNV_OFFSETS[1];
        for (let index = 0; index < text.length; index += 1) {
            const unit = text.charCodeAt(index);
            high = Math.imul(high ^ unit, FNV_PRIME);
            low = Math.imul(low ^ unit, FNV_PRIME);
        }

        return (high >>> 0) * 2 ** LOW_BITS + ((low >>> 0) >>> (32 - LOW_BITS));
    }
}
