import { Key, type ColumnValue, type Row, Order } from "@destack/db";
import { found, zip } from "@destack/schema";
import type { Node } from "../query/node.ts";
import { NOWHERE, Pipeline, keyed, without, type Context } from "./pipeline.ts";
import { TreeJoin } from "./input.ts";
import type { Run, Work } from "./run.ts";
import type { Segment } from "./view.ts";
import { Window } from "./window.ts";

/** The most rows of one root read: about 1 MB at about 1 KB a row. */
export const PAGE_ROWS = 1000;

/** The rows a node selects per partition: every candidate, or a window of the first ones. */
export class Selection extends Pipeline {
    /** The partitions arranging each candidate row, by key. */
    readonly #arranged = new Map<string, Set<string>>();
    /** The rows the open partitions' windows know. */
    #windowed = 0;
    /** The windows that lost rows and wait for a refill. */
    readonly #refills = new Set<string>();
    /** The tree path whose members keep chains of the rows between. */
    readonly #chaining: TreeJoin | undefined;
    /** The rows between each member and its partitions' selected rows, by member and partition. */
    readonly #chains = new Map<string, Map<string, readonly Row[]>>();
    /** How many chains keep each row, by key. */
    readonly #chained = new Map<string, number>();
    /** The members whose chains the run moved. */
    readonly #rechains = new Map<string, Row | null>();

    /** Create the selection of a node. */
    constructor(node: Node, context: Context, hasTreeIndex: boolean) {
        super(node, context, hasTreeIndex);
        this.#chaining =
            this.input instanceof TreeJoin && (node.limit !== undefined || node.where !== undefined)
                ? this.input
                : undefined;
    }

    /** The rows the windows know. */
    get size(): number {
        return this.#windowed;
    }

    /** Forget everything. */
    override forget(): void {
        // forget the partitions and members, then the windows and chains
        super.forget();
        this.#arranged.clear();
        this.#windowed = 0;
        this.#refills.clear();
        this.#chains.clear();
        this.#chained.clear();
        this.#rechains.clear();
    }

    /** Empty closed partitions, fill opened ones, and decide dirty rows again. */
    async step(work: Work, run: Run): Promise<void> {
        // empty the closed partitions
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition === undefined || partition.holders > 0) {
                continue;
            }
            for (const key of partition.members) {
                this.#assign(key, null, without(found(this.members, key).partitions, name), run);
            }
            for (const key of partition.window?.isArranged === true
                ? partition.window.known()
                : []) {
                this.#unarrange(key, name);
            }
            this.#windowed -= partition.window?.size ?? 0;
            this.partitions.delete(name);
        }

        // fill the opened partitions and decide the dirty rows
        const opened = [...work.opened].filter(([name]) => this.partitions.has(name));
        await this.#fill(opened, run);
        await this.decide(work.dirty, run);
        await this.settle(run);
    }

    /** Select the next page of a root's rows after a row, returning the rows read. */
    async page(after: Row | undefined, run: Run): Promise<Row[]> {
        // resolve the start position's computed values
        if (after !== undefined) {
            await this.filter.measure([after], run);
        }
        const start = after === undefined ? undefined : this.filter.resolve(after, run);
        const [rows] = await this.ordered([segmentOf(null, start)], PAGE_ROWS, run);
        if (rows === undefined) {
            throw new RangeError("a page read lacks its one window");
        }
        await this.decide(keyed(this.node, rows), run);
        await this.settle(run);

        return rows;
    }

    /** Refill the windows and chain the moved members. */
    async settle(run: Run): Promise<void> {
        await this.#refill(run);
        await this.#rechain(run);
    }

    /** Read the keys of a partition's selected rows. */
    keys(name: string): readonly string[] {
        const partition = this.partitions.get(name);

        return partition?.window?.keys() ?? [...(partition?.members ?? [])];
    }

    /** Fill opened partitions. */
    async #fill(opened: readonly (readonly [string, ColumnValue])[], run: Run): Promise<void> {
        // read each window's first rows, or every candidate
        if (opened.length === 0) {
            return;
        }
        const read = await this.#readOpened(opened, run);
        const limit = this.node.limit;

        // select the rows of each opened partition
        for (const [[name, value], rows] of zip(opened, read)) {
            // select every candidate of an unlimited node
            if (limit === undefined) {
                for (const row of rows) {
                    this.#join(this.node.keyOf(row), row, name, run);
                }
            }
            // order a window or arrange every candidate
            else {
                this.#openWindow(name, value, rows, limit, run);
            }
        }
    }

    /** Read each opened partition's first rows or candidates after its start. */
    async #readOpened(
        opened: readonly (readonly [string, ColumnValue])[],
        run: Run,
    ): Promise<Row[][]> {
        // resolve each partition's start position
        const node = this.node;
        const limit = node.limit;
        const unresolved = opened.map(([name]) => found(this.partitions, name).start);
        await this.filter.measure(
            unresolved.filter((start): start is Row => start !== undefined),
            run,
        );
        const starts = unresolved.map((start) =>
            start === undefined ? undefined : this.filter.resolve(start, run),
        );

        // read each window's first rows
        if (limit !== undefined && !node.isArranged) {
            const segments = zip(opened, starts).map(([[, value], start]) =>
                segmentOf(value, start),
            );

            return this.ordered(segments, limit, run);
        }
        // read every candidate after each start
        else {
            const matched = await this.matched(
                opened.map(([, value]) => value),
                run,
            );

            return zip(matched, starts).map(([rows, start]) =>
                start === undefined ? rows : rows.filter((row) => node.compare(row, start) > 0),
            );
        }
    }

    /** Open a partition's window over its read rows and select its first rows. */
    #openWindow(
        name: string,
        value: ColumnValue,
        rows: readonly Row[],
        limit: number,
        run: Run,
    ): void {
        // order a window or arrange every candidate
        const node = this.node;
        const byKey = keyed(node, rows);
        const window = new Window(
            (left, right) => node.compare(left, right),
            limit,
            value,
            [...byKey].map(([key, row]) => ({
                key,
                row: node.isArranged ? node.orderOf(row) : row,
            })),
            node.isArranged,
        );
        found(this.partitions, name).window = window;
        this.#windowed += window.size;

        // select the first rows and note the arrangements
        for (const key of node.isArranged ? byKey.keys() : []) {
            this.#arrange(key, name);
        }
        for (const key of window.keys()) {
            this.#join(key, found(byKey, key), name, run);
        }
    }

    /** Admit a decided row to its windows or partitions. */
    protected admit(key: string, row: Row | null, names: ReadonlySet<string>, run: Run): void {
        this.context.sink.touch(run, this.node.table, key, row);
        if (this.node.limit === undefined) {
            this.#assign(key, row, names, run);
        } else {
            this.#place(key, row, names, run);
        }
    }

    /** Place a row in its partitions' windows and take it out of the others. */
    #place(key: string, row: Row | null, names: ReadonlySet<string>, run: Run): void {
        // start from the windows with or arranging the row
        const node = this.node;
        const partitions = new Set(this.members.get(key)?.partitions ?? NOWHERE);
        for (const name of new Set([
            ...partitions,
            ...(this.#arranged.get(key) ?? NOWHERE),
            ...names,
        ])) {
            // skip partitions without a window
            const window = this.partitions.get(name)?.window;
            if (window === undefined) {
                continue;
            }

            // place the row in order, evicting the last row of a full window
            if (names.has(name)) {
                if (row === null) {
                    throw new TypeError(`${node.name} places a missing row ${key}`);
                }
                const wasPresent = window.has(key);
                const size = window.size;
                const placed = window.place(key, window.isArranged ? node.orderOf(row) : row);
                this.#windowed += window.size - size;
                if (window.isArranged) {
                    this.#arrange(key, name);
                }
                if (placed.isPresent) {
                    partitions.add(name);
                } else {
                    partitions.delete(name);
                }
                if (wasPresent && !placed.isPresent) {
                    this.#refills.add(name);
                }

                // let go of the evicted row
                const evictedKey = placed.evicted;
                const evicted = evictedKey === undefined ? undefined : this.members.get(evictedKey);
                if (evictedKey !== undefined && evicted !== undefined) {
                    this.#assign(evictedKey, null, without(evicted.partitions, name), run);
                } else if (evictedKey !== undefined && !window.isArranged) {
                    throw new TypeError(
                        `window of ${node.name} pushed out absent row ${evictedKey}`,
                    );
                }
            }
            // take the row out of a window it left, refilling it
            else {
                const size = window.size;
                const wasPresent = window.remove(key);
                this.#windowed += window.size - size;
                this.#unarrange(key, name);
                partitions.delete(name);
                if (wasPresent) {
                    this.#refills.add(name);
                }
            }
        }
        this.#assign(key, row, partitions, run);
    }

    /** Select a row in one more partition. */
    #join(key: string, row: Row, name: string, run: Run): void {
        const partitions = this.members.get(key)?.partitions ?? NOWHERE;
        this.context.sink.touch(run, this.node.table, key, row);
        this.#assign(key, row, new Set([...partitions, name]), run);
    }

    /** Set the partitions with a row and retain or let go of its children's partitions. */
    #assign(key: string, row: Row | null, names: ReadonlySet<string>, run: Run): void {
        // move the row between partitions
        const node = this.node;
        const sink = this.context.sink;
        const member = this.move(key, names);

        // enter, leave or move its children's partitions
        const joins =
            row === null
                ? (member?.joins ?? [])
                : node.children.map((child) => child.valueOf(row) ?? null);
        if (names.size === 0) {
            if (member !== undefined) {
                this.members.delete(key);
                sink.retain(run, node.table, key, row, -1);
                this.#retainChildren(member.joins, -1, run);
            }
        } else if (member === undefined) {
            if (row === null) {
                throw new TypeError(`${node.name} selects a missing row ${key}`);
            }
            this.members.set(key, {
                partitions: names,
                joins,
                ...(this.context.isMaterialized ? { row } : {}),
            });
            sink.retain(run, node.table, key, row, 1);
            this.#retainChildren(joins, 1, run);
        } else {
            member.partitions = names;
            if (this.context.isMaterialized && row !== null) {
                member.row = row;
            }
            if (!sameValues(member.joins, joins)) {
                this.#retainChildren(member.joins, -1, run);
                this.#retainChildren(joins, 1, run);
                member.joins = joins;
            }
        }

        // chain the member again
        if (this.#chaining !== undefined) {
            this.#rechains.set(key, row ?? this.#rechains.get(key) ?? null);
        }
    }

    /** Retain or let go of the children's partitions a member names. */
    #retainChildren(joins: readonly ColumnValue[], step: 1 | -1, run: Run): void {
        for (const [child, value] of zip(this.children, joins)) {
            if (child !== undefined && value !== null) {
                child.retain(value, step, run);
            }
        }
    }

    /** Refill the windows that lost rows, all at once. */
    async #refill(run: Run): Promise<void> {
        // select the arranged rows that moved into the window
        const names = [...this.#refills];
        this.#refills.clear();
        const promoted: [string, string][] = [];
        const segments: [string, Window][] = [];
        for (const name of names) {
            const partition = this.partitions.get(name);
            const window = partition?.window;
            if (partition !== undefined && window?.isArranged === true) {
                for (const key of window.keys().filter((entry) => !partition.members.has(entry))) {
                    promoted.push([name, key]);
                }
            }
            // note the windows lacking rows
            else if (window !== undefined && !window.isExhaustive && !window.isFull) {
                segments.push([name, window]);
            }
        }
        await this.#joinPromoted(promoted, run);

        // read the missing rows after each window's last row
        const limit = this.node.limit;
        if (limit === undefined || segments.length === 0) {
            return;
        }
        await this.#extendWindows(segments, limit, run);
    }

    /** Select the arranged rows that moved into their windows. */
    async #joinPromoted(promoted: readonly (readonly [string, string])[], run: Run): Promise<void> {
        const promotedRows = await this.#rowsOf(
            promoted.map(([, key]) => key),
            run,
        );
        for (const [[name, key], row] of zip(promoted, promotedRows)) {
            this.#join(key, row, name, run);
        }
    }

    /** Extend and select each window lacking rows by the rows after its last one. */
    async #extendWindows(
        segments: readonly (readonly [string, Window])[],
        limit: number,
        run: Run,
    ): Promise<void> {
        // read every window's missing rows together
        const read = await Promise.all(
            segments.map(async ([name, window]) => {
                const missing = limit - window.size;
                const rows = await this.#readMissing(window, missing, run);

                return { name, window, missing, rows };
            }),
        );

        // extend each window and select its new rows
        for (const { name, window, missing, rows } of read) {
            const size = window.size;
            window.extend(
                rows.map((row) => ({ key: this.node.keyOf(row), row })),
                missing,
            );
            this.#windowed += window.size - size;
            for (const row of rows) {
                this.#join(this.node.keyOf(row), row, name, run);
            }
        }
    }

    /** Read a window's missing rows after its last row. */
    async #readMissing(window: Window, missing: number, run: Run): Promise<Row[]> {
        const [rows] = await this.ordered([segmentOf(window.value, window.last)], missing, run);

        // require the one window's rows
        if (rows === undefined) {
            throw new RangeError("a refill read lacks its one window");
        }

        return rows;
    }

    /** Read arranged candidate rows by key with their computed values. */
    async #rowsOf(keys: readonly string[], run: Run): Promise<Row[]> {
        // read and decide the rows
        const node = this.node;
        const read = await run.view.keyed(
            node.table,
            keys.map((key) => Key.parse(node.table, key)),
        );
        const rows = read.map((row, index) => {
            if (row === null) {
                throw new TypeError(`arrangement of ${node.name} keeps missing row ${keys[index]}`);
            }

            return row;
        });
        await this.filter.prepare(rows, run);

        // require each to be a candidate
        return rows.map((row, index) => {
            if (!this.filter.isCandidate(row, run)) {
                throw new TypeError(
                    `arrangement of ${node.name} keeps non-candidate row ${keys[index]}`,
                );
            }

            return this.filter.resolve(row, run);
        });
    }

    /** Note that a partition arranges a row. */
    #arrange(key: string, name: string): void {
        let names = this.#arranged.get(key);
        if (names === undefined) {
            names = new Set();
            this.#arranged.set(key, names);
        }
        names.add(name);
    }

    /** Note that a partition no longer arranges a row. */
    #unarrange(key: string, name: string): void {
        const names = this.#arranged.get(key);
        names?.delete(name);
        if (names?.size === 0) {
            this.#arranged.delete(key);
        }
    }

    /** Chain the moved members again to the rows between them and their selected rows. */
    async #rechain(run: Run): Promise<void> {
        // read the rows between each moved member and its selected rows
        const moved = [...this.#rechains];
        this.#rechains.clear();
        if (moved.length === 0) {
            return;
        }
        const chained = await this.#readChains(moved, run);

        // chain each member in each partition
        for (const [key, row] of moved) {
            this.#rechainMember(key, row, chained, run);
        }
    }

    /** Read the rows between each present moved member and its partitions' selected rows. */
    async #readChains(
        moved: readonly (readonly [string, Row | null])[],
        run: Run,
    ): Promise<Map<string, Map<string, readonly Row[]>>> {
        // require the tree path the chains follow
        const chaining = this.#chaining;
        if (chaining === undefined) {
            throw new TypeError(`${this.node.name} chains members without a tree path`);
        }

        // pair a present member with each of its partitions
        const pairs = moved.flatMap(([key, row]) =>
            row === null
                ? []
                : [...(this.members.get(key)?.partitions ?? NOWHERE)].map((name) => ({
                      key,
                      name,
                      member: row,
                      value: found(this.partitions, name).value,
                  })),
        );

        // read the chains and index them by member and partition
        const chains = await chaining.between(pairs, run);
        const chained = new Map<string, Map<string, readonly Row[]>>();
        for (const [pair, rows] of zip(pairs, chains)) {
            const partitions = chained.get(pair.key) ?? new Map<string, readonly Row[]>();
            chained.set(pair.key, partitions.set(pair.name, rows));
        }

        return chained;
    }

    /** Chain one moved member in each of its partitions. */
    #rechainMember(
        key: string,
        row: Row | null,
        chained: ReadonlyMap<string, ReadonlyMap<string, readonly Row[]>>,
        run: Run,
    ): void {
        // take a present member's read chain or keep the known one
        const known = this.#chains.get(key);
        const next = new Map<string, readonly Row[]>();
        for (const name of this.members.get(key)?.partitions ?? NOWHERE) {
            const chain = row === null ? known?.get(name) : chained.get(key)?.get(name);
            if (chain !== undefined) {
                next.set(name, chain);
            }
        }

        // count the new chains in and the old ones out
        this.#chainEach(next.values(), 1, run);
        this.#chainEach(known?.values() ?? [], -1, run);
        if (next.size === 0) {
            this.#chains.delete(key);
        } else {
            this.#chains.set(key, next);
        }
    }

    /** Count every row of some chains up or down. */
    #chainEach(chains: Iterable<readonly Row[]>, step: 1 | -1, run: Run): void {
        for (const rows of chains) {
            for (const between of rows) {
                this.#chain(between, step, run);
            }
        }
    }

    /** Count a row's chains up or down and keep it while any chain does. */
    #chain(row: Row, step: 1 | -1, run: Run): void {
        // keep the row with its first chain and let go after its last
        const node = this.node;
        const key = node.keyOf(row);
        const count = (this.#chained.get(key) ?? 0) + step;
        if (count === 0) {
            this.#chained.delete(key);
            this.context.sink.retain(run, node.table, key, null, -1);
        } else {
            this.#chained.set(key, count);
            if (count === 1 && step > 0) {
                this.context.sink.retain(run, node.table, key, row, 1);
            }
        }
    }
}

/** Read one partition's rows after an optional start row. */
function segmentOf(value: ColumnValue, after: Row | undefined): Segment {
    return after === undefined ? { value } : { value, after };
}

/** Whether two lists have equal values in order. */
function sameValues(left: readonly unknown[], right: readonly unknown[]): boolean {
    return (
        left.length === right.length &&
        left.every((value, index) => Order.missingFirst(value, right[index]) === 0)
    );
}
