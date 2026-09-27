import { Key, type Row, type Table } from "@destack/db";
import { Order } from "@destack/db/query";
import { Node } from "../query/node.ts";
import type { Run } from "./run.ts";

/**
 * Where a node's rows come from: a scan of a root's scopes, or a join of each held parent row to the rows its path names.
 *
 * It reads each partition's rows and locates each row's partitions, both ways, a batch of partitions or rows per call.
 * A key join names the rows holding a value; a junction join the rows its join rows name; a tree join every row below or above a held row, through rows the audience sees.
 */
export abstract class Input {
    /** The node whose rows the input reads. */
    readonly node: Node;

    /** Read a node's rows. */
    constructor(node: Node) {
        this.node = node;
    }

    /** Read a node's rows by its path: a scan for a root, else a join, following trees through their index unless the database holds none. */
    static of(node: Node, hasTreeIndex: boolean): Input {
        const path = node.path;

        return path === undefined
            ? new Scan(node)
            : path.kind === "key"
              ? new KeyJoin(node)
              : path.kind === "junction"
                ? new JunctionJoin(node)
                : new TreeJoin(node, hasTreeIndex);
    }

    /** Read each partition's rows, before the node's condition and the audience's view of them, aligned with the held values naming them. */
    abstract members(values: readonly unknown[], run: Run): Promise<Row[][]>;

    /** Read the held values naming the partitions each row belongs to, aligned with the rows. */
    abstract locate(rows: readonly Row[], run: Run): Promise<unknown[][]>;

    /** Add the rows of the node's table a change of a row moves beyond itself to the rows to decide again, as they are at the position. */
    async moves(
        _table: Table,
        _before: Row | undefined,
        _after: Row | undefined,
        _isAccess: boolean,
        _run: Run,
        _dirty: Map<string, Row | undefined>,
    ): Promise<void> {}

    /** Keep the rows of a table in the node's scopes the audience sees, which a path may pass through. */
    protected async passable(table: Table, rows: readonly Row[], run: Run): Promise<Row[]> {
        return run.seen(
            table,
            rows.filter((row) => Node.isScoped(row, this.node.scopes)),
        );
    }
}

/** A root's one partition: every row of its scopes. */
class Scan extends Input {
    /** Read every row of the scopes. */
    async members(values: readonly unknown[], run: Run): Promise<Row[][]> {
        const rows = await run.view.matching(this.node.table, Node.scoped(this.node.scopes));

        return values.map(() => rows);
    }

    /** Place every row in the one partition. */
    async locate(rows: readonly Row[]): Promise<unknown[][]> {
        return rows.map(() => [undefined]);
    }
}

/** A key path: the rows whose column holds a held row's value. */
class KeyJoin extends Input {
    /** The included row's column. */
    get #column(): string {
        return (this.node.path as { readonly column: string }).column;
    }

    /** Read the rows holding each value. */
    async members(values: readonly unknown[], run: Run): Promise<Row[][]> {
        const read = await run.view.lookup(
            this.node.table,
            values.map((value) => ({ [this.#column]: value })),
        );

        return read.map((rows) => [...rows]);
    }

    /** Name each row's own value. */
    async locate(rows: readonly Row[]): Promise<unknown[][]> {
        return rows.map((row) => {
            const value = row[this.#column];

            return value === null || value === undefined ? [] : [value];
        });
    }
}

/** A junction path: the rows the join rows of a held row name, through join rows the audience sees. */
class JunctionJoin extends Input {
    /** The path. */
    get #path(): Extract<NonNullable<Node["path"]>, { readonly kind: "junction" }> {
        return this.node.path as Extract<NonNullable<Node["path"]>, { readonly kind: "junction" }>;
    }

    /** Read the rows the seen join rows of each value name, once each. */
    async members(values: readonly unknown[], run: Run): Promise<Row[][]> {
        // read the join rows naming each value, keeping those the path passes through
        const path = this.#path;
        const joins = await run.view.lookup(
            path.table,
            values.map((value) => ({ [path.from.column]: value })),
        );
        const passable = new Set(await this.passable(path.table, joins.flat(), run));
        const named = joins.map((rows) => rows.filter((join) => passable.has(join)));

        // read the rows every join row names at once, then each value's rows once each
        const targets = await run.view.lookup(
            this.node.table,
            named.flat().map((join) => ({ [path.to.key]: join[path.to.column] })),
        );

        return unflatten(targets, named).map((reached) => [
            ...new Map(reached.flat().map((row) => [this.node.keyOf(row), row])).values(),
        ]);
    }

    /** Name the held values the seen join rows naming each row join, once each. */
    async locate(rows: readonly Row[], run: Run): Promise<unknown[][]> {
        // read the join rows naming each row, keeping those the path passes through
        const path = this.#path;
        const joins = await run.view.lookup(
            path.table,
            rows.map((row) => ({ [path.to.column]: row[path.to.key] })),
        );
        const passable = new Set(await this.passable(path.table, joins.flat(), run));

        // name each distinct present value once, since a row belongs to a partition once however many join rows name it
        return joins.map((entries) => {
            const values = new Map<string, unknown>();
            for (const join of entries) {
                const value = join[path.from.column];
                if (passable.has(join) && value !== null && value !== undefined) {
                    values.set(this.node.partition(value), value);
                }
            }

            return [...values.values()];
        });
    }

    /** Decide again the rows a changed join row names, as it was and is. */
    async moves(
        table: Table,
        before: Row | undefined,
        after: Row | undefined,
        _isAccess: boolean,
        run: Run,
        dirty: Map<string, Row | undefined>,
    ): Promise<void> {
        // follow the join row's images to the rows they name
        const path = this.#path;
        if (table !== path.table) {
            return;
        }
        const images = [before, after].filter((image): image is Row => image !== undefined);
        const named = await run.view.lookup(
            this.node.table,
            images.map((image) => ({ [path.to.key]: image[path.to.column] })),
        );
        for (const row of named.flat()) {
            dirty.set(this.node.keyOf(row), row);
        }
    }
}

/**
 * A tree path: every row below a held row, or above it, along its table's parent column, through rows the audience sees.
 *
 * A database's tree index lists every ancestor of every row with its depth; a copy holds none and walks the parent column.
 */
export class TreeJoin extends Input {
    /** Whether the database holds the tree index. */
    readonly #isIndexed: boolean;
    /** Whether the path reaches down, to descendants. */
    readonly #isDown: boolean;
    /** The column naming a row's parent. */
    readonly #column: string;
    /** The table's one key column. */
    readonly #key: string;

    /** Follow a node's tree path, through its index when the database holds one. */
    constructor(node: Node, isIndexed: boolean) {
        // note which way the path reaches, and its columns
        super(node);
        this.#isIndexed = isIndexed;
        this.#isDown = node.path!.kind === "descendants";
        this.#column = (node.path as { readonly column: string }).column;
        this.#key = node.key[0]!;
    }

    /** Read the rows below each held row, or above it. */
    async members(values: readonly unknown[], run: Run): Promise<Row[][]> {
        // read the held rows, then every row the path reaches from each
        const held = await run.view.keyed(
            this.node.table,
            values.map((value) => ({ [this.#key]: value })),
        );
        const present = held.filter((row): row is Row => row !== undefined);
        const reached = this.#isDown
            ? await this.below(present, run)
            : await this.above(present, run);
        const byRow = new Map(present.map((row, index) => [row, reached[index]!]));

        return held.map((row) => (row === undefined ? [] : byRow.get(row)!));
    }

    /** Name the rows whose partitions each row belongs to: every seen row above it, or below it. */
    async locate(rows: readonly Row[], run: Run): Promise<unknown[][]> {
        const reached = this.#isDown ? await this.above(rows, run) : await this.below(rows, run);

        return reached.map((entries) => entries.map((entry) => entry[this.#key]));
    }

    /**
     * Decide again the rows a changed row carries along: those a tree index row gains or loses, those below or above a row that moved, and those whose paths pass a row whose visibility may change.
     *
     * A copy, which holds no index, walks from the row's images instead.
     */
    async moves(
        table: Table,
        before: Row | undefined,
        after: Row | undefined,
        isAccess: boolean,
        run: Run,
        dirty: Map<string, Row | undefined>,
    ): Promise<void> {
        // follow a copy's tree row, whose move or visibility carries the rows its images reach
        const node = this.node;
        const end = this.#isDown ? "descendant" : "ancestor";
        if (!this.#isIndexed && table === node.table) {
            const isMoved =
                isAccess ||
                before === undefined ||
                after === undefined ||
                Order.missingFirst(before[this.#column], after[this.#column]) !== 0 ||
                (await this.#passes(before, run)) !== (await this.#passes(after, run));
            const images = isMoved ? [before, after].filter((image) => image !== undefined) : [];
            const reached = this.#isDown
                ? await this.below(images, run)
                : await this.above(images, run);
            for (const row of reached.flat()) {
                dirty.set(node.keyOf(row), row);
            }
        }
        // follow a tree index row to the row whose partitions it changes
        else if (this.#isIndexed && table === node.closure) {
            const ends = [before, after]
                .filter((image): image is Row => image !== undefined && image.depth !== 0)
                .map((image) => image[end]);
            await this.#mark(ends, run, dirty);
        }
        // follow a tree row whose visibility may change to every row whose path passes through it
        else if (
            this.#isIndexed &&
            table === node.table &&
            (isAccess || (await this.#passes(before, run)) !== (await this.#passes(after, run)))
        ) {
            const other = end === "descendant" ? "ancestor" : "descendant";
            const [paths] = await run.view.lookup(node.closure!, [
                node.paths((after ?? before)!, other),
            ]);
            const ends = paths!.filter((path) => path.depth !== 0).map((path) => path[end]);
            await this.#mark(ends, run, dirty);
        }
    }

    /**
     * Read the rows strictly between each member and the held row whose value names a partition holding it, through rows the audience sees.
     *
     * A member the path no longer reaches from the held row has no rows between.
     */
    async between(
        pairs: readonly { readonly member: Row; readonly value: unknown }[],
        run: Run,
    ): Promise<Row[][]> {
        // walk up from the lower row of each pair to the upper one
        const key = this.#key;
        const held = this.#isDown
            ? []
            : await run.view.keyed(
                  this.node.table,
                  pairs.map(({ value }) => ({ [key]: value })),
              );
        const lower = pairs.map(({ member }, index) => (this.#isDown ? member : held[index]));
        const present = lower.filter((row): row is Row => row !== undefined);
        const reached = await this.above(present, run);
        const byRow = new Map(present.map((row, index) => [row, reached[index]!]));

        // keep the rows up to the upper row, none when the walk misses it
        return pairs.map(({ member, value }, index) => {
            // walk up from the lower row until the upper one
            const upper = this.#isDown ? value : member[key];
            const rows: Row[] = [];
            const row = lower[index];
            for (const entry of row === undefined ? [] : byRow.get(row)!) {
                if (Order.values(entry[key], upper) === 0) {
                    return rows;
                }
                rows.push(entry);
            }

            return [];
        });
    }

    /** Read the seen rows above each row, nearest first, stopping at a row the path may not pass through. */
    async above(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk the parent column up without an index
        if (!this.#isIndexed) {
            return this.#walkUp(rows, run);
        }

        // read every row's ancestors from the index, nearest first, with their visibility at once
        const node = this.node;
        const paths = await run.view.lookup(
            node.closure!,
            rows.map((row) => node.paths(row, "descendant")),
        );
        const nearest = paths.map((entries) =>
            entries
                .filter((path) => (path.depth as number) > 0)
                .sort((left, right) => (left.depth as number) - (right.depth as number)),
        );
        const ancestors = await run.view.keyed(
            node.table,
            nearest.flat().map((path) => ({ [this.#key]: path.ancestor })),
        );
        const passable = new Set(
            await this.passable(
                node.table,
                ancestors.filter((row): row is Row => row !== undefined),
                run,
            ),
        );

        // keep each row's ancestors up to the first the path may not pass through
        return unflatten(ancestors, nearest).map((entries) => {
            const blocked = entries.findIndex(
                (ancestor) => ancestor === undefined || !passable.has(ancestor),
            );

            return (blocked === -1 ? entries : entries.slice(0, blocked)) as Row[];
        });
    }

    /** Read the seen rows below each row, reached through seen rows only, shallowest first. */
    async below(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk the children of each level without an index
        if (!this.#isIndexed) {
            return this.#walkDown(rows, run);
        }

        // read every row's descendants from the index, shallowest first, with their visibility at once
        const node = this.node;
        const paths = await run.view.lookup(
            node.closure!,
            rows.map((row) => node.paths(row, "ancestor")),
        );
        const shallowest = paths.map((entries) =>
            entries
                .filter((path) => (path.depth as number) > 0)
                .sort((left, right) => (left.depth as number) - (right.depth as number)),
        );
        const descendants = await run.view.keyed(
            node.table,
            shallowest.flat().map((path) => ({ [this.#key]: path.descendant })),
        );
        const passable = new Set(
            await this.passable(
                node.table,
                descendants.filter((row): row is Row => row !== undefined),
                run,
            ),
        );

        // reach each seen descendant whose parent the path reached
        return unflatten(descendants, shallowest).map((entries, position) => {
            // start from the held row, adding each descendant the path reaches
            const reached = new Set<unknown>([rows[position]![this.#key]]);
            const kept: Row[] = [];
            for (const descendant of entries) {
                if (
                    descendant !== undefined &&
                    passable.has(descendant) &&
                    reached.has(descendant[this.#column])
                ) {
                    reached.add(descendant[this.#key]);
                    kept.push(descendant);
                }
            }

            return kept;
        });
    }

    /** Walk each row's parent column up through the seen rows a copy holds, a level of every row at once, nearest first. */
    async #walkUp(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk every row up at once, guarding against cycles
        const key = this.#key;
        const walks = rows.map((row) => ({
            rows: [] as Row[],
            seen: new Set<unknown>([row[key]]),
            current: row as Row | undefined,
        }));
        for (;;) {
            // read the next parent of every walk still open
            const open = walks.filter((walk) => {
                // close a walk at the top or at a cycle
                const parent = walk.current?.[this.#column];
                const isOpen = parent !== null && parent !== undefined && !walk.seen.has(parent);
                if (!isOpen) {
                    walk.current = undefined;
                }

                return isOpen;
            });
            if (open.length === 0) {
                return walks.map((walk) => walk.rows);
            }
            const parents = await run.view.keyed(
                this.node.table,
                open.map((walk) => ({ [key]: walk.current![this.#column] })),
            );
            const passable = new Set(
                await this.passable(
                    this.node.table,
                    parents.filter((row): row is Row => row !== undefined),
                    run,
                ),
            );

            // step each walk up, stopping at a row the path may not pass through
            for (const [index, walk] of open.entries()) {
                const parent = parents[index];
                if (parent === undefined || !passable.has(parent)) {
                    walk.current = undefined;
                } else {
                    walk.seen.add(parent[key]);
                    walk.rows.push(parent);
                    walk.current = parent;
                }
            }
        }
    }

    /** Walk the children of each level down through the seen rows a copy holds, a level of every row at once. */
    async #walkDown(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk every row down at once, guarding against cycles
        const key = this.#key;
        const walks = rows.map((row) => ({
            rows: [] as Row[],
            seen: new Set<unknown>([row[key]]),
            level: [row],
        }));
        while (walks.some((walk) => walk.level.length > 0)) {
            // read the children of every walk's level at once
            const parents = walks.flatMap((walk) => walk.level);
            const children = await run.view.lookup(
                this.node.table,
                parents.map((parent) => ({ [this.#column]: parent[key] })),
            );
            const passable = new Set(await this.passable(this.node.table, children.flat(), run));

            // take each walk's next level of seen rows once each
            const levels = unflatten(
                children,
                walks.map((walk) => walk.level),
            );
            for (const [position, walk] of walks.entries()) {
                const next: Row[] = [];
                for (const child of levels[position]!.flat()) {
                    if (passable.has(child) && !walk.seen.has(child[key])) {
                        walk.seen.add(child[key]);
                        next.push(child);
                    }
                }
                walk.rows.push(...next);
                walk.level = next;
            }
        }

        return walks.map((walk) => walk.rows);
    }

    /** Decide whether the path may pass through a row: present, of the node's scopes, and seen by the audience. */
    async #passes(row: Row | undefined, run: Run): Promise<boolean> {
        return row !== undefined && (await this.passable(this.node.table, [row], run)).length > 0;
    }

    /** Decide the rows at some ends of tree paths again, by key, as they are at the position. */
    async #mark(
        ends: readonly unknown[],
        run: Run,
        dirty: Map<string, Row | undefined>,
    ): Promise<void> {
        const keys = ends.map((end) => ({ [this.#key]: end }));
        const rows = await run.view.keyed(this.node.table, keys);
        for (const [index, key] of keys.entries()) {
            dirty.set(Key.name(this.node.table, key), rows[index]);
        }
    }
}

/** Split a flat list into consecutive groups as long as some lists, undoing their flattening. */
function unflatten<Item>(flat: readonly Item[], lists: readonly (readonly unknown[])[]): Item[][] {
    // take each list's length of items in turn
    let offset = 0;

    return lists.map((list) => {
        const group = flat.slice(offset, offset + list.length);
        offset += list.length;

        return group;
    });
}
