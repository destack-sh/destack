import { Key, type ColumnValue, type Row, type Table, Order } from "@destack/db";
import { found, zip } from "@destack/schema";
import { Node } from "../query/node.ts";
import type { JunctionPath, KeyPath, TreePath } from "@destack/db";
import type { Run } from "./run.ts";

/** The source of a node's rows: a scan of a root's scopes, or a join to selected parent rows. */
export abstract class Input {
    /** The node whose rows the input reads. */
    readonly node: Node;

    /** Read a node's rows. */
    constructor(node: Node) {
        this.node = node;
    }

    /** Create the input of a node's path. */
    static of(node: Node, hasTreeIndex: boolean): Input {
        const path = node.path;
        switch (path?.kind) {
            case undefined:
                return new Scan(node);
            case "key":
                return new KeyJoin(node, path);
            case "junction":
                return new JunctionJoin(node, path);
            case "descendants":
            case "ancestors":
                return new TreeJoin(node, path, hasTreeIndex);
        }
    }

    /** Read each partition's rows, aligned with the parent values naming them. */
    abstract members(values: readonly ColumnValue[], run: Run): Promise<Row[][]>;

    /** Read the parent values naming each row's partitions. */
    abstract locate(rows: readonly Row[], run: Run): Promise<ColumnValue[][]>;

    /** Add the rows a changed row moves beyond itself to the rows to decide again. */
    async moves(
        _table: Table,
        _before: Row | null,
        _after: Row | null,
        _isAccess: boolean,
        _run: Run,
        _dirty: Map<string, Row | null>,
    ): Promise<void> {}

    /** Keep the visible rows of a table in the node's scopes. */
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
    async members(values: readonly ColumnValue[], run: Run): Promise<Row[][]> {
        const rows = await run.view.matching(this.node.table, Node.scoped(this.node.scopes));

        return values.map(() => rows);
    }

    /** Place every row in the one partition. */
    async locate(rows: readonly Row[]): Promise<ColumnValue[][]> {
        return rows.map(() => [null]);
    }
}

/** A key path: the rows whose column has a selected row's value. */
class KeyJoin extends Input {
    /** The path. */
    readonly #path: KeyPath;

    /** Create the input of a key path. */
    constructor(node: Node, path: KeyPath) {
        super(node);
        this.#path = path;
    }

    /** Read the rows with each value. */
    async members(values: readonly ColumnValue[], run: Run): Promise<Row[][]> {
        const read = await run.view.lookup(
            this.node.table,
            values.map((value) => ({ [this.#path.column]: value })),
        );

        return read.map((rows) => [...rows]);
    }

    /** Name each row's own value. */
    async locate(rows: readonly Row[]): Promise<ColumnValue[][]> {
        return rows.map((row) => {
            const value = row[this.#path.column];

            return value === null || value === undefined ? [] : [value];
        });
    }
}

/** A junction path: the rows a selected row's visible join rows name. */
class JunctionJoin extends Input {
    /** The path. */
    readonly #path: JunctionPath;

    /** Create the input of a junction path. */
    constructor(node: Node, path: JunctionPath) {
        super(node);
        this.#path = path;
    }

    /** Read the rows each value's visible join rows name, once each. */
    async members(values: readonly ColumnValue[], run: Run): Promise<Row[][]> {
        // read the visible join rows naming each value
        const path = this.#path;
        const joins = await run.view.lookup(
            path.table,
            values.map((value) => ({ [path.from.column]: value })),
        );
        const passable = new Set(await this.passable(path.table, joins.flat(), run));
        const named = joins.map((rows) => rows.filter((join) => passable.has(join)));

        // read the named rows at once
        const targets = await run.view.lookup(
            this.node.table,
            named.flat().map((join) => ({ [path.to.key]: valueOf(join, path.to.column) })),
        );

        return unflatten(targets, named).map((reached) => [
            ...new Map(reached.flat().map((row) => [this.node.keyOf(row), row])).values(),
        ]);
    }

    /** Name the parent values each row's visible join rows join. */
    async locate(rows: readonly Row[], run: Run): Promise<ColumnValue[][]> {
        // read the visible join rows naming each row
        const path = this.#path;
        const joins = await run.view.lookup(
            path.table,
            rows.map((row) => ({ [path.to.column]: valueOf(row, path.to.key) })),
        );
        const passable = new Set(await this.passable(path.table, joins.flat(), run));

        // name each distinct value once
        return joins.map((entries) => {
            const values = new Map<string, ColumnValue>();
            for (const join of entries) {
                const value = join[path.from.column];
                if (passable.has(join) && value !== null && value !== undefined) {
                    values.set(this.node.partition(value), value);
                }
            }

            return [...values.values()];
        });
    }

    /** Decide again the rows a changed join row names. */
    override async moves(
        table: Table,
        before: Row | null,
        after: Row | null,
        _isAccess: boolean,
        run: Run,
        dirty: Map<string, Row | null>,
    ): Promise<void> {
        // follow the join row's images
        const path = this.#path;
        if (table !== path.table) {
            return;
        }
        const images = [before, after].filter((image): image is Row => image !== null);
        const named = await run.view.lookup(
            this.node.table,
            images.map((image) => ({ [path.to.key]: valueOf(image, path.to.column) })),
        );
        for (const row of named.flat()) {
            dirty.set(this.node.keyOf(row), row);
        }
    }
}

/** A tree path: every visible row below or above a selected row. */
export class TreeJoin extends Input {
    /** Whether the database has the tree index. */
    readonly #isIndexed: boolean;
    /** Whether the path reaches down, to descendants. */
    readonly #isDown: boolean;
    /** The column naming a row's parent. */
    readonly #column: string;
    /** The table's one key column. */
    readonly #key: string;

    /** Create the input of a tree path. */
    constructor(node: Node, path: TreePath, isIndexed: boolean) {
        // note the direction and columns
        super(node);
        const [key] = node.key;
        if (key === undefined || node.key.length !== 1) {
            throw new TypeError(`${node.name} follows a tree by one key column`);
        }
        this.#isIndexed = isIndexed;
        this.#isDown = path.kind === "descendants";
        this.#column = path.column;
        this.#key = key;
    }

    /** Read the rows below each selected row, or above it. */
    async members(values: readonly ColumnValue[], run: Run): Promise<Row[][]> {
        // read the parent rows and the rows they relate to
        const parents = await run.view.keyed(
            this.node.table,
            values.map((value) => ({ [this.#key]: value })),
        );
        const present = parents.filter((row): row is Row => row !== null);
        const reached = this.#isDown
            ? await this.below(present, run)
            : await this.above(present, run);
        const byRow = new Map(zip(present, reached));

        return parents.map((row) => (row === null ? [] : found(byRow, row)));
    }

    /** Name the visible rows above or below each row. */
    async locate(rows: readonly Row[], run: Run): Promise<ColumnValue[][]> {
        const reached = this.#isDown ? await this.above(rows, run) : await this.below(rows, run);

        return reached.map((entries) => entries.map((entry) => valueOf(entry, this.#key)));
    }

    /** Decide again the rows below a changed row along a tree. */
    override async moves(
        table: Table,
        before: Row | null,
        after: Row | null,
        isAccess: boolean,
        run: Run,
        dirty: Map<string, Row | null>,
    ): Promise<void> {
        // follow a copy's tree row
        const node = this.node;
        const end = this.#isDown ? "descendant" : "ancestor";
        if (!this.#isIndexed && table === node.table) {
            const isMoved =
                isAccess ||
                before === null ||
                after === null ||
                Order.missingFirst(before[this.#column], after[this.#column]) !== 0 ||
                (await this.#passes(before, run)) !== (await this.#passes(after, run));
            const images = isMoved
                ? [before, after].filter((image): image is Row => image !== null)
                : [];
            const reached = this.#isDown
                ? await this.below(images, run)
                : await this.above(images, run);
            for (const row of reached.flat()) {
                dirty.set(node.keyOf(row), row);
            }
        }
        // follow a tree index row
        else if (this.#isIndexed && table === node.closure) {
            const ends = [before, after]
                .filter((image): image is Row => image !== null && image["depth"] !== 0)
                .map((image) => valueOf(image, end));
            await this.#mark(ends, run, dirty);
        }
        // follow a tree row whose visibility may change
        else if (
            this.#isIndexed &&
            table === node.table &&
            (isAccess || (await this.#passes(before, run)) !== (await this.#passes(after, run)))
        ) {
            const image = after ?? before;
            if (image === null) {
                return;
            }
            const other = end === "descendant" ? "ancestor" : "descendant";
            const [paths = []] = await run.view.lookup(this.#closure(), [node.paths(image, other)]);
            const ends = paths
                .filter((path) => path["depth"] !== 0)
                .map((path) => valueOf(path, end));
            await this.#mark(ends, run, dirty);
        }
    }

    /** Read the rows strictly between each member and its parent row. */
    async between(
        pairs: readonly { readonly member: Row; readonly value: ColumnValue }[],
        run: Run,
    ): Promise<Row[][]> {
        // walk up from the lower row of each pair
        const key = this.#key;
        const parents = this.#isDown
            ? pairs.map(() => null)
            : await run.view.keyed(
                  this.node.table,
                  pairs.map(({ value }) => ({ [key]: value })),
              );
        const lower = zip(pairs, parents).map(([{ member }, parent]) =>
            this.#isDown ? member : parent,
        );
        const present = lower.filter((row): row is Row => row !== null);
        const byRow = new Map(zip(present, await this.above(present, run)));

        // keep the rows up to the upper row
        return zip(pairs, lower).map(([{ member, value }, row]) => {
            // walk up until the upper row
            const upper = this.#isDown ? value : member[key];
            const rows: Row[] = [];
            for (const entry of row === null ? [] : found(byRow, row)) {
                if (Order.values(entry[key], upper) === 0) {
                    return rows;
                }
                rows.push(entry);
            }

            return [];
        });
    }

    /** Read the visible rows above each row, nearest first. */
    async above(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk up without an index
        if (!this.#isIndexed) {
            return this.#walkUp(rows, run);
        }

        // read the ancestors from the index
        const node = this.node;
        const paths = await run.view.lookup(
            this.#closure(),
            rows.map((row) => node.paths(row, "descendant")),
        );
        const nearest = paths.map((entries) => byDepth(entries));
        const ancestors = await run.view.keyed(
            node.table,
            nearest.flat().map((path) => ({ [this.#key]: valueOf(path, "ancestor") })),
        );
        const passable = new Set(
            await this.passable(
                node.table,
                ancestors.filter((row): row is Row => row !== null),
                run,
            ),
        );

        // keep ancestors up to the first impassable row
        return unflatten(ancestors, nearest).map((entries) => {
            const kept: Row[] = [];
            for (const ancestor of entries) {
                if (ancestor === null || !passable.has(ancestor)) {
                    break;
                }
                kept.push(ancestor);
            }

            return kept;
        });
    }

    /** Read the visible rows below each row, shallowest first. */
    async below(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk down without an index
        if (!this.#isIndexed) {
            return this.#walkDown(rows, run);
        }

        // read the descendants from the index
        const node = this.node;
        const paths = await run.view.lookup(
            this.#closure(),
            rows.map((row) => node.paths(row, "ancestor")),
        );
        const shallowest = paths.map((entries) => byDepth(entries));
        const descendants = await run.view.keyed(
            node.table,
            shallowest.flat().map((path) => ({ [this.#key]: valueOf(path, "descendant") })),
        );
        const passable = new Set(
            await this.passable(
                node.table,
                descendants.filter((row): row is Row => row !== null),
                run,
            ),
        );

        // keep each descendant whose parent the path reached
        return zip(rows, unflatten(descendants, shallowest)).map(([row, entries]) => {
            // start from the parent row
            const reached = new Set<ColumnValue | undefined>([row[this.#key]]);
            const kept: Row[] = [];
            for (const descendant of entries) {
                if (
                    descendant !== null &&
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

    /** Read the tree index, which an indexed path reads. */
    #closure(): Table {
        const closure = this.node.closure;
        if (closure === undefined) {
            throw new TypeError(`${this.node.name} reads a tree index its node lacks`);
        }

        return closure;
    }

    /** Walk each row's parent column up, a level at a time. */
    async #walkUp(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk every row up, guarding against cycles
        const key = this.#key;
        const walks: {
            readonly rows: Row[];
            readonly seen: Set<ColumnValue | undefined>;
            current: Row | null;
        }[] = rows.map((row) => ({
            rows: [],
            seen: new Set<ColumnValue | undefined>([row[key]]),
            current: row,
        }));
        for (;;) {
            // read the next parent of every open walk and close walks at the top or at a cycle
            const open: { readonly walk: (typeof walks)[number]; readonly parent: ColumnValue }[] =
                [];
            for (const walk of walks) {
                const parent = walk.current?.[this.#column];
                if (parent === null || parent === undefined || walk.seen.has(parent)) {
                    walk.current = null;
                } else {
                    open.push({ walk, parent });
                }
            }
            if (open.length === 0) {
                return walks.map((walk) => walk.rows);
            }
            const parents = await run.view.keyed(
                this.node.table,
                open.map(({ parent }) => ({ [key]: parent })),
            );
            const passable = new Set(
                await this.passable(
                    this.node.table,
                    parents.filter((row): row is Row => row !== null),
                    run,
                ),
            );

            // step each walk up
            for (const [{ walk }, parent] of zip(open, parents)) {
                if (parent === null || !passable.has(parent)) {
                    walk.current = null;
                } else {
                    walk.seen.add(parent[key]);
                    walk.rows.push(parent);
                    walk.current = parent;
                }
            }
        }
    }

    /** Walk each row's children down, a level at a time. */
    async #walkDown(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk every row down, guarding against cycles
        const key = this.#key;
        const walks: {
            readonly rows: Row[];
            readonly seen: Set<ColumnValue | undefined>;
            level: Row[];
        }[] = rows.map((row) => ({
            rows: [],
            seen: new Set<ColumnValue | undefined>([row[key]]),
            level: [row],
        }));
        while (walks.some((walk) => walk.level.length > 0)) {
            // read the children of every walk's level
            const parents = walks.flatMap((walk) => walk.level);
            const children = await run.view.lookup(
                this.node.table,
                parents.map((parent) => ({ [this.#column]: valueOf(parent, key) })),
            );
            const passable = new Set(await this.passable(this.node.table, children.flat(), run));

            // take each walk's next level once
            const levels = unflatten(
                children,
                walks.map((walk) => walk.level),
            );
            for (const [walk, level] of zip(walks, levels)) {
                const next: Row[] = [];
                for (const child of level.flat()) {
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

    /** Decide whether the path may pass through a row. */
    async #passes(row: Row | null, run: Run): Promise<boolean> {
        return row !== null && (await this.passable(this.node.table, [row], run)).length > 0;
    }

    /** Decide again the rows at some tree path ends. */
    async #mark(
        ends: readonly ColumnValue[],
        run: Run,
        dirty: Map<string, Row | null>,
    ): Promise<void> {
        const keys = ends.map((end) => ({ [this.#key]: end }));
        const rows = await run.view.keyed(this.node.table, keys);
        for (const [key, row] of zip(keys, rows)) {
            dirty.set(Key.name(this.node.table, key), row);
        }
    }
}

/** Read a row's value of a column, which the row's table declares. */
function valueOf(row: Row, column: string): ColumnValue {
    const value = row[column];
    if (value === undefined) {
        throw new TypeError(`row has no column ${column}`);
    }

    return value;
}

/** Keep the strict tree index paths, nearest first. */
function byDepth(paths: readonly Row[]): Row[] {
    return paths
        .filter((path) => depthOf(path) > 0)
        .toSorted((left, right) => depthOf(left) - depthOf(right));
}

/** Read a tree index path's depth. */
function depthOf(path: Row): number {
    const depth = path["depth"];
    if (typeof depth !== "number") {
        throw new TypeError("a tree index path has no depth");
    }

    return depth;
}

/** Split a flat list into groups as long as some lists. */
function unflatten<Entry>(
    flat: readonly Entry[],
    lists: readonly (readonly unknown[])[],
): Entry[][] {
    // take each list's length of items in turn
    let offset = 0;

    return lists.map((list) => {
        const group = flat.slice(offset, offset + list.length);
        offset += list.length;

        return group;
    });
}
