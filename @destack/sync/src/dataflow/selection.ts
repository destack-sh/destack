import { Key, type Row } from "@destack/db";
import { Order } from "@destack/db/query";
import type { Node } from "../query/node.ts";
import { NOWHERE, Pipeline, keyed, without, type Context } from "./pipeline.ts";
import { TreeJoin } from "./input.ts";
import type { Run, Work } from "./run.ts";
import { Window } from "./window.ts";

/** The most rows of one root read: about 1 MB at about 1 KB a row. */
export const PAGE_ROWS = 1000;

/** The rows a node holds per partition: every candidate, or a window of the first ones. */
export class Selection extends Pipeline {
    /** The partitions arranging each candidate row, by key. */
    readonly #arranged = new Map<string, Set<string>>();
    /** The rows the open partitions' windows know. */
    #windowed = 0;
    /** The windows that lost rows and wait for a refill. */
    readonly #refills = new Set<string>();
    /** The tree path whose members hold chains of the rows between. */
    readonly #chaining: TreeJoin | undefined;
    /** The rows between each member and its partitions' held rows, by member and partition. */
    readonly #chains = new Map<string, Map<string, readonly Row[]>>();
    /** How many chains hold each row, by key. */
    readonly #chained = new Map<string, number>();
    /** The members whose chains the run moved. */
    readonly #rechains = new Map<string, Row | undefined>();

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
    forget(): void {
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
                this.#assign(key, undefined, without(this.members.get(key)!.partitions, name), run);
            }
            for (const key of partition.window?.isArranged === true
                ? partition.window.keys()
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

    /** Hold the next page of a root's rows after a row, returning the rows read. */
    async page(after: Row | undefined, run: Run): Promise<Row[]> {
        // resolve the start row's computed values
        if (after !== undefined) {
            await this.filter.prepare([after], run);
        }
        const start = after === undefined ? undefined : this.filter.resolve(after, run);
        const [rows] = await this.ordered(
            [start === undefined ? {} : { after: start }],
            PAGE_ROWS,
            run,
        );
        await this.decide(keyed(this.node, rows!), run);
        await this.settle(run);

        return rows!;
    }

    /** Refill the windows and chain the moved members. */
    async settle(run: Run): Promise<void> {
        await this.#refill(run);
        await this.#rechain(run);
    }

    /** Read the keys of a partition's held rows. */
    keys(name: string): readonly string[] {
        const partition = this.partitions.get(name);

        return partition?.window?.held() ?? [...(partition?.members ?? [])];
    }

    /** Fill opened partitions. */
    async #fill(opened: readonly (readonly [string, unknown])[], run: Run): Promise<void> {
        // read each window's first rows, or every candidate
        const node = this.node;
        const limit = node.limit;
        if (opened.length === 0) {
            return;
        }
        const starts = opened.map(([name]) => this.partitions.get(name)!.start);
        await this.filter.prepare(
            starts.filter((start): start is Row => start !== undefined),
            run,
        );
        for (const [index, start] of starts.entries()) {
            starts[index] = start === undefined ? undefined : this.filter.resolve(start, run);
        }
        const read =
            limit !== undefined && !node.isArranged
                ? await this.ordered(
                      opened.map(([, value], index) => {
                          const start = starts[index];

                          return start === undefined ? { value } : { value, after: start };
                      }),
                      limit,
                      run,
                  )
                : (
                      await this.matched(
                          opened.map(([, value]) => value),
                          run,
                      )
                  ).map((rows, index) => {
                      const start = starts[index];

                      return start === undefined
                          ? rows
                          : rows.filter((row) => node.compare(row, start) > 0);
                  });

        // hold every candidate of an unlimited node
        for (const [index, [name, value]] of opened.entries()) {
            const rows = read[index]!;
            if (limit === undefined) {
                for (const row of rows) {
                    this.#join(node.keyOf(row), row, name, run);
                }
                continue;
            }

            // order a window or arrange every candidate
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
            this.partitions.get(name)!.window = window;
            this.#windowed += window.size;

            // hold the first rows and note the arrangements
            for (const key of node.isArranged ? byKey.keys() : []) {
                this.#arrange(key, name);
            }
            for (const key of window.held()) {
                this.#join(key, byKey.get(key)!, name, run);
            }
        }
    }

    /** Admit a decided row to its windows or partitions. */
    protected admit(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        this.context.sink.touch(run, this.node.table, key, row);
        if (this.node.limit === undefined) {
            this.#assign(key, row, names, run);
        } else {
            this.#place(key, row, names, run);
        }
    }

    /** Place a row in its partitions' windows and take it out of the others. */
    #place(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        // start from the windows holding or arranging the row
        const node = this.node;
        const held = new Set(this.members.get(key)?.partitions ?? NOWHERE);
        for (const name of new Set([...held, ...(this.#arranged.get(key) ?? NOWHERE), ...names])) {
            // skip partitions without a window
            const window = this.partitions.get(name)?.window;
            if (window === undefined) {
                continue;
            }

            // place the row in order, evicting the last row of a full window
            if (names.has(name)) {
                const wasHeld = window.holds(key);
                const size = window.size;
                const placed = window.place(key, window.isArranged ? node.orderOf(row!) : row!);
                this.#windowed += window.size - size;
                if (window.isArranged) {
                    this.#arrange(key, name);
                }
                if (placed.isHeld) {
                    held.add(name);
                } else {
                    held.delete(name);
                }
                if (wasHeld && !placed.isHeld) {
                    this.#refills.add(name);
                }

                // let go of the evicted row
                const evicted =
                    placed.evicted === undefined ? undefined : this.members.get(placed.evicted);
                if (evicted !== undefined) {
                    this.#assign(
                        placed.evicted!,
                        undefined,
                        without(evicted.partitions, name),
                        run,
                    );
                } else if (placed.evicted !== undefined && !window.isArranged) {
                    throw new TypeError(
                        `window of ${node.name} pushed out unheld row ${placed.evicted}`,
                    );
                }
            }
            // take the row out of a window it left, refilling it
            else {
                const size = window.size;
                const wasHeld = window.remove(key);
                this.#windowed += window.size - size;
                this.#unarrange(key, name);
                held.delete(name);
                if (wasHeld) {
                    this.#refills.add(name);
                }
            }
        }
        this.#assign(key, row, held, run);
    }

    /** Hold a row in one more partition. */
    #join(key: string, row: Row, name: string, run: Run): void {
        const partitions = this.members.get(key)?.partitions ?? NOWHERE;
        this.context.sink.touch(run, this.node.table, key, row);
        this.#assign(key, row, new Set([...partitions, name]), run);
    }

    /** Set the partitions holding a row, holding or letting go of what it holds. */
    #assign(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        // move the row between partitions
        const node = this.node;
        const sink = this.context.sink;
        const member = this.move(key, names);

        // enter, leave or move its children's partitions
        const joins =
            row === undefined
                ? (member?.joins ?? [])
                : node.children.map((child) => child.valueOf(row) ?? null);
        if (names.size === 0) {
            if (member !== undefined) {
                this.members.delete(key);
                sink.retain(run, node.table, key, row, -1);
                this.#holdChildren(member.joins, -1, run);
            }
        } else if (member === undefined) {
            this.members.set(key, {
                partitions: names,
                joins,
                ...(this.context.isMaterialized ? { row: row! } : {}),
            });
            sink.retain(run, node.table, key, row, 1);
            this.#holdChildren(joins, 1, run);
        } else {
            member.partitions = names;
            if (this.context.isMaterialized && row !== undefined) {
                member.row = row;
            }
            if (!sameValues(member.joins, joins)) {
                this.#holdChildren(member.joins, -1, run);
                this.#holdChildren(joins, 1, run);
                member.joins = joins;
            }
        }

        // chain the member again
        if (this.#chaining !== undefined) {
            this.#rechains.set(key, row ?? this.#rechains.get(key));
        }
    }

    /** Hold or let go of the children's partitions a member names. */
    #holdChildren(joins: readonly unknown[], step: 1 | -1, run: Run): void {
        for (const [index, child] of this.children.entries()) {
            const value = joins[index];
            if (child !== undefined && value !== null && value !== undefined) {
                child.hold(value, step, run);
            }
        }
    }

    /** Refill the windows that lost rows, all at once. */
    async #refill(run: Run): Promise<void> {
        // hold the arranged rows that moved into the window
        const names = [...this.#refills];
        this.#refills.clear();
        const promoted: [string, string][] = [];
        const segments: [string, Window][] = [];
        for (const name of names) {
            const partition = this.partitions.get(name);
            const window = partition?.window;
            if (window?.isArranged === true) {
                for (const key of window.held().filter((entry) => !partition!.members.has(entry))) {
                    promoted.push([name, key]);
                }
            }
            // note the windows lacking rows
            else if (window !== undefined && !window.isExhaustive && !window.isFull) {
                segments.push([name, window]);
            }
        }
        const rows = await this.#rowsOf(
            promoted.map(([, key]) => key),
            run,
        );
        for (const [index, [name, key]] of promoted.entries()) {
            this.#join(key, rows[index]!, name, run);
        }

        // read the missing rows after each window's last row
        const missing = segments.map(([, window]) => this.node.limit! - window.size);
        const read = await Promise.all(
            segments.map(([, window], index) =>
                this.ordered(
                    [
                        window.last === undefined
                            ? { value: window.value }
                            : { value: window.value, after: window.last },
                    ],
                    missing[index]!,
                    run,
                ),
            ),
        );
        for (const [index, [name, window]] of segments.entries()) {
            const rows = read[index]![0]!;
            const size = window.size;
            window.extend(
                rows.map((row) => ({ key: this.node.keyOf(row), row })),
                missing[index]!,
            );
            this.#windowed += window.size - size;
            for (const row of rows) {
                this.#join(this.node.keyOf(row), row, name, run);
            }
        }
    }

    /** Read arranged candidate rows by key with their computed values. */
    async #rowsOf(keys: readonly string[], run: Run): Promise<Row[]> {
        // read and decide the rows
        const node = this.node;
        const read = await run.view.keyed(
            node.table,
            keys.map((key) => Key.parse(node.table, key) as Row),
        );
        const rows = read.map((row, index) => {
            if (row === undefined) {
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

    /** Chain the moved members again to the rows between them and their held rows. */
    async #rechain(run: Run): Promise<void> {
        // read the rows between each moved member and its held rows
        const moved = [...this.#rechains];
        this.#rechains.clear();
        if (moved.length === 0) {
            return;
        }
        const pairs: {
            readonly key: string;
            readonly name: string;
            readonly member: Row;
            readonly value: unknown;
        }[] = [];
        for (const [key, row] of moved) {
            const names =
                row === undefined ? NOWHERE : (this.members.get(key)?.partitions ?? NOWHERE);
            for (const name of names) {
                pairs.push({ key, name, member: row!, value: this.partitions.get(name)!.value });
            }
        }
        const between = await this.#chaining!.between(pairs, run);
        const found = new Map<string, Map<string, readonly Row[]>>();
        for (const [index, pair] of pairs.entries()) {
            const chains = found.get(pair.key) ?? new Map<string, readonly Row[]>();
            found.set(pair.key, chains.set(pair.name, between[index]!));
        }

        // chain each member in each partition
        for (const [key, row] of moved) {
            const known = this.#chains.get(key);
            const next = new Map<string, readonly Row[]>();
            for (const name of this.members.get(key)?.partitions ?? NOWHERE) {
                const chain = row === undefined ? known?.get(name) : found.get(key)?.get(name);
                if (chain !== undefined) {
                    next.set(name, chain);
                }
            }

            // count the new chains in and the old ones out
            for (const rows of next.values()) {
                for (const between of rows) {
                    this.#chain(between, 1, run);
                }
            }
            for (const rows of known?.values() ?? []) {
                for (const between of rows) {
                    this.#chain(between, -1, run);
                }
            }
            if (next.size === 0) {
                this.#chains.delete(key);
            } else {
                this.#chains.set(key, next);
            }
        }
    }

    /** Count a row's chains up or down, holding it while any chain does. */
    #chain(row: Row, step: 1 | -1, run: Run): void {
        // hold the row with its first chain and let go after its last
        const node = this.node;
        const key = node.keyOf(row);
        const count = (this.#chained.get(key) ?? 0) + step;
        if (count === 0) {
            this.#chained.delete(key);
            this.context.sink.retain(run, node.table, key, undefined, -1);
        } else {
            this.#chained.set(key, count);
            if (count === 1 && step > 0) {
                this.context.sink.retain(run, node.table, key, row, 1);
            }
        }
    }
}

/** Whether two lists hold equal values in order. */
function sameValues(left: readonly unknown[], right: readonly unknown[]): boolean {
    return (
        left.length === right.length &&
        left.every((value, index) => Order.missingFirst(value, right[index]) === 0)
    );
}
