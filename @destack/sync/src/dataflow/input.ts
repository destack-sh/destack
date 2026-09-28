import { Key, type Row, type Table } from "@destack/db";
import { Order } from "@destack/db/query";
import { Node } from "../query/node.ts";
import type { Run } from "./run.ts";

/** The source of a node's rows: a scan of a root's scopes, or a join to held parent rows. */
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

        return path === undefined
            ? new Scan(node)
            : path.kind === "key"
              ? new KeyJoin(node)
              : path.kind === "junction"
                ? new JunctionJoin(node)
                : new TreeJoin(node, hasTreeIndex);
    }

    /** Read each partition's rows, aligned with the held values naming them. */
    abstract members(values: readonly unknown[], run: Run): Promise<Row[][]>;

    /** Read the held values naming each row's partitions. */
    abstract locate(rows: readonly Row[], run: Run): Promise<unknown[][]>;

    /** Add the rows a changed row moves beyond itself to the rows to decide again. */
    async moves(
        _table: Table,
        _before: Row | undefined,
        _after: Row | undefined,
        _isAccess: boolean,
        _run: Run,
        _dirty: Map<string, Row | undefined>,
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

/** A junction path: the rows a held row's visible join rows name. */
class JunctionJoin extends Input {
    /** The path. */
    get #path(): Extract<NonNullable<Node["path"]>, { readonly kind: "junction" }> {
        return this.node.path as Extract<NonNullable<Node["path"]>, { readonly kind: "junction" }>;
    }

    /** Read the rows each value's visible join rows name, once each. */
    async members(values: readonly unknown[], run: Run): Promise<Row[][]> {
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
            named.flat().map((join) => ({ [path.to.key]: join[path.to.column] })),
        );

        return unflatten(targets, named).map((reached) => [
            ...new Map(reached.flat().map((row) => [this.node.keyOf(row), row])).values(),
        ]);
    }

    /** Name the held values each row's visible join rows join. */
    async locate(rows: readonly Row[], run: Run): Promise<unknown[][]> {
        // read the visible join rows naming each row
        const path = this.#path;
        const joins = await run.view.lookup(
            path.table,
            rows.map((row) => ({ [path.to.column]: row[path.to.key] })),
        );
        const passable = new Set(await this.passable(path.table, joins.flat(), run));

        // name each distinct value once
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

    /** Decide again the rows a changed join row names. */
    async moves(
        table: Table,
        before: Row | undefined,
        after: Row | undefined,
        _isAccess: boolean,
        run: Run,
        dirty: Map<string, Row | undefined>,
    ): Promise<void> {
        // follow the join row's images
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

/** A tree path: every visible row below or above a held row. */
export class TreeJoin extends Input {
    /** Whether the database holds the tree index. */
    readonly #isIndexed: boolean;
    /** Whether the path reaches down, to descendants. */
    readonly #isDown: boolean;
    /** The column naming a row's parent. */
    readonly #column: string;
    /** The table's one key column. */
    readonly #key: string;

    /** Create the input of a tree path. */
    constructor(node: Node, isIndexed: boolean) {
        // note the direction and columns
        super(node);
        this.#isIndexed = isIndexed;
        this.#isDown = node.path!.kind === "descendants";
        this.#column = (node.path as { readonly column: string }).column;
        this.#key = node.key[0]!;
    }

    /** Read the rows below each held row, or above it. */
    async members(values: readonly unknown[], run: Run): Promise<Row[][]> {
        // read the held rows and the rows they reach
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

    /** Name the visible rows above or below each row. */
    async locate(rows: readonly Row[], run: Run): Promise<unknown[][]> {
        const reached = this.#isDown ? await this.above(rows, run) : await this.below(rows, run);

        return reached.map((entries) => entries.map((entry) => entry[this.#key]));
    }

    /** Decide again the rows a changed row carries along a tree. */
    async moves(
        table: Table,
        before: Row | undefined,
        after: Row | undefined,
        isAccess: boolean,
        run: Run,
        dirty: Map<string, Row | undefined>,
    ): Promise<void> {
        // follow a copy's tree row
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
        // follow a tree index row
        else if (this.#isIndexed && table === node.closure) {
            const ends = [before, after]
                .filter((image): image is Row => image !== undefined && image.depth !== 0)
                .map((image) => image[end]);
            await this.#mark(ends, run, dirty);
        }
        // follow a tree row whose visibility may change
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

    /** Read the rows strictly between each member and its held row. */
    async between(
        pairs: readonly { readonly member: Row; readonly value: unknown }[],
        run: Run,
    ): Promise<Row[][]> {
        // walk up from the lower row of each pair
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

        // keep the rows up to the upper row
        return pairs.map(({ member, value }, index) => {
            // walk up until the upper row
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

    /** Read the visible rows above each row, nearest first. */
    async above(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk up without an index
        if (!this.#isIndexed) {
            return this.#walkUp(rows, run);
        }

        // read the ancestors from the index
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

        // keep ancestors up to the first impassable row
        return unflatten(ancestors, nearest).map((entries) => {
            const blocked = entries.findIndex(
                (ancestor) => ancestor === undefined || !passable.has(ancestor),
            );

            return (blocked === -1 ? entries : entries.slice(0, blocked)) as Row[];
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

        // keep each descendant whose parent the path reached
        return unflatten(descendants, shallowest).map((entries, position) => {
            // start from the held row
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

    /** Walk each row's parent column up, a level at a time. */
    async #walkUp(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk every row up, guarding against cycles
        const key = this.#key;
        const walks = rows.map((row) => ({
            rows: [] as Row[],
            seen: new Set<unknown>([row[key]]),
            current: row as Row | undefined,
        }));
        for (;;) {
            // read the next parent of every open walk
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

            // step each walk up
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

    /** Walk each row's children down, a level at a time. */
    async #walkDown(rows: readonly Row[], run: Run): Promise<Row[][]> {
        // walk every row down, guarding against cycles
        const key = this.#key;
        const walks = rows.map((row) => ({
            rows: [] as Row[],
            seen: new Set<unknown>([row[key]]),
            level: [row],
        }));
        while (walks.some((walk) => walk.level.length > 0)) {
            // read the children of every walk's level
            const parents = walks.flatMap((walk) => walk.level);
            const children = await run.view.lookup(
                this.node.table,
                parents.map((parent) => ({ [this.#column]: parent[key] })),
            );
            const passable = new Set(await this.passable(this.node.table, children.flat(), run));

            // take each walk's next level once
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

    /** Decide whether the path may pass through a row. */
    async #passes(row: Row | undefined, run: Run): Promise<boolean> {
        return row !== undefined && (await this.passable(this.node.table, [row], run)).length > 0;
    }

    /** Decide again the rows at some tree path ends. */
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

/** Split a flat list into groups as long as some lists. */
function unflatten<Item>(flat: readonly Item[], lists: readonly (readonly unknown[])[]): Item[][] {
    // take each list's length of items in turn
    let offset = 0;

    return lists.map((list) => {
        const group = flat.slice(offset, offset + list.length);
        offset += list.length;

        return group;
    });
}
