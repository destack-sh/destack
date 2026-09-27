import { Key, type Row } from "@destack/db";
import { Order } from "@destack/db/query";
import type { Node } from "../query/node.ts";
import { NOWHERE, Pipeline, keyed, without, type Context } from "./pipeline.ts";
import { TreeJoin } from "./input.ts";
import type { Run, Work } from "./run.ts";
import { Window } from "./window.ts";

/** The most rows one read of a root's rows holds: about 1 MB at about 1 KB a row, a page of a snapshot. */
export const PAGE_ROWS = 1000;

/**
 * The rows a node holds per partition: every candidate, or the first of its order in a window when limited.
 *
 * Held rows hold their children's partitions open; a tree include whose members skip rows holds the rows between as chains.
 */
export class Selection extends Pipeline {
    /** The partitions whose arrangements keep each candidate row, by key. */
    readonly #arranged = new Map<string, Set<string>>();
    /** The rows the open partitions' windows know. */
    #windowed = 0;
    /** The windows that lost held rows, which refill once the run decided the node's rows. */
    readonly #refills = new Set<string>();
    /** The tree path whose members hold the rows between them and their partitions' held rows, absent unless they skip rows. */
    readonly #chaining: TreeJoin | undefined;
    /** The rows between each member and the held rows whose partitions hold it, by member and partition. */
    readonly #chains = new Map<string, Map<string, readonly Row[]>>();
    /** How many chains hold each row, by key. */
    readonly #chained = new Map<string, number>();
    /** The members whose chains the run moved, as they are at the position when known. */
    readonly #rechains = new Map<string, Row | undefined>();

    /** Hold a node's rows within a dataflow. */
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

    /** Forget everything the selection holds. */
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

    /** Empty the closed partitions, fill the opened ones, and decide the dirty rows again. */
    async step(work: Work, run: Run): Promise<void> {
        // empty and forget the closed partitions
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

        // fill the opened partitions, then decide the dirty rows again
        const opened = [...work.opened].filter(([name]) => this.partitions.has(name));
        await this.#fill(opened, run);
        await this.decide(work.dirty, run);
        await this.settle(run);
    }

    /** Hold the next page of a root's rows in order after a row, returning the rows read. */
    async page(after: Row | undefined, run: Run): Promise<Row[]> {
        // read after the row with its computed values, as the order compares them
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

    /** Refill the windows that lost rows and chain the members that moved, once the run decided the node's rows. */
    async settle(run: Run): Promise<void> {
        await this.#refill(run);
        await this.#rechain(run);
    }

    /** Read the keys of a partition's held rows, in order for a window. */
    keys(name: string): readonly string[] {
        const partition = this.partitions.get(name);

        return partition?.window?.held() ?? [...(partition?.members ?? [])];
    }

    /** Fill opened partitions: the first rows of each window, every candidate of an arrangement, or every candidate. */
    async #fill(opened: readonly (readonly [string, unknown])[], run: Run): Promise<void> {
        // read the first rows of each window, or every candidate of each partition
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

            // order a window of the first rows, or arrange every candidate by the values it sorts by
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

            // hold the first rows, noting where each candidate is arranged
            for (const key of node.isArranged ? byKey.keys() : []) {
                this.#arrange(key, name);
            }
            for (const key of window.held()) {
                this.#join(key, byKey.get(key)!, name, run);
            }
        }
    }

    /** Admit a decided row: place it in its windows when limited, else hold it in its partitions. */
    protected admit(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        this.context.sink.touch(run, this.node.table, key, row);
        if (this.node.limit === undefined) {
            this.#assign(key, row, names, run);
        } else {
            this.#place(key, row, names, run);
        }
    }

    /** Place a row in the windows of the partitions it belongs to, and take it out of the others, letting go of what a full window pushes out. */
    #place(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        // start from the windows holding or arranging the row now
        const node = this.node;
        const held = new Set(this.members.get(key)?.partitions ?? NOWHERE);
        for (const name of new Set([...held, ...(this.#arranged.get(key) ?? NOWHERE), ...names])) {
            // skip partitions without a window, which a fill decides
            const window = this.partitions.get(name)?.window;
            if (window === undefined) {
                continue;
            }

            // place the row where the order puts it, letting go of the row a full window pushes out
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

                // let go of the row pushed out, which an arrangement may not hold yet while a refill waits
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
            // take the row out of a window it no longer belongs to, refilling it
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

    /**
     * Set the partitions holding a row, as the row is at the position when known.
     *
     * A row entering holds its children's partitions and the subscriber's copy; one leaving lets go of them; one staying moves the partitions its join values name.
     */
    #assign(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        // move the row between the partitions' members
        const node = this.node;
        const sink = this.context.sink;
        const member = this.move(key, names);

        // enter, leave, or move the partitions the row's join values name
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

        // chain the member again once the run decided the node's rows
        if (this.#chaining !== undefined) {
            this.#rechains.set(key, row ?? this.#rechains.get(key));
        }
    }

    /** Hold or let go of the children's partitions a member's join values name. */
    #holdChildren(joins: readonly unknown[], step: 1 | -1, run: Run): void {
        for (const [index, child] of this.children.entries()) {
            const value = joins[index];
            if (child !== undefined && value !== null && value !== undefined) {
                child.hold(value, step, run);
            }
        }
    }

    /** Refill the windows that lost rows with the candidates following their last ones, all at once. */
    async #refill(run: Run): Promise<void> {
        // hold the arranged rows that moved up among the first, reading them by key
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
            // note the windows lacking rows unless none follow
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

        // read as many rows as each window lacks, in order after its last row, all at once
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

    /** Read candidate rows by key with their computed values, which an arrangement knows to be candidates as of the run. */
    async #rowsOf(keys: readonly string[], run: Run): Promise<Row[]> {
        // read the rows and decide their visibility and relations at once
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

        // require each to be a candidate, as the arrangement holds it
        return rows.map((row, index) => {
            if (!this.filter.isCandidate(row, run)) {
                throw new TypeError(
                    `arrangement of ${node.name} keeps non-candidate row ${keys[index]}`,
                );
            }

            return this.filter.resolve(row, run);
        });
    }

    /** Note that a partition's arrangement keeps a row. */
    #arrange(key: string, name: string): void {
        let names = this.#arranged.get(key);
        if (names === undefined) {
            names = new Set();
            this.#arranged.set(key, names);
        }
        names.add(name);
    }

    /** Note that a partition's arrangement no longer keeps a row. */
    #unarrange(key: string, name: string): void {
        const names = this.#arranged.get(key);
        names?.delete(name);
        if (names?.size === 0) {
            this.#arranged.delete(key);
        }
    }

    /**
     * Chain again the members whose partitions or rows moved: hold the rows strictly between each and every held row whose partition holds it.
     *
     * A row a chain holds stays held while any chain or member holds it.
     */
    async #rechain(run: Run): Promise<void> {
        // read the rows between each moved member and each of its partitions' held rows at once
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

        // chain each member in each partition holding it, keeping the chains of a member whose row the run did not read
        for (const [key, row] of moved) {
            const known = this.#chains.get(key);
            const next = new Map<string, readonly Row[]>();
            for (const name of this.members.get(key)?.partitions ?? NOWHERE) {
                const chain = row === undefined ? known?.get(name) : found.get(key)?.get(name);
                if (chain !== undefined) {
                    next.set(name, chain);
                }
            }

            // count the rows of the new chains in, then those of the old ones out
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

    /** Count a row one chain more or less, holding it while any chain does. */
    #chain(row: Row, step: 1 | -1, run: Run): void {
        // hold the row with its first chain and let it go after its last
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
