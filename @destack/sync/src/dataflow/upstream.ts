import { Change, type ColumnValue, type Row, type Scalar } from "@destack/db";
import { canonicalize } from "@destack/schema";
import type { Node } from "../query/node.ts";
import type { Watch } from "../feed/audience.ts";
import type { Arrangement } from "./filter.ts";
import type { Pipeline } from "./pipeline.ts";
import { Input } from "./input.ts";
import type { Run } from "./run.ts";
import type { Average } from "./tally.ts";

/** What a copy knows of its source: the source's groups with the copy's predictions. */
export interface Upstream {
    /** The tables and scopes whose changes move the groups. */
    readonly watches: readonly Watch[];
    /** Read every group of an aggregate or relation node with rows. */
    groups(node: Node): Promise<Group[]>;
    /** Name the group a change of a watched table moves. */
    groupOf(
        change: Change,
    ): { readonly query: string; readonly group: Readonly<Record<string, Scalar>> } | undefined;
    /** Report whether the source measured an aggregate node. */
    readonly isMeasured?: (node: Node) => boolean;
}

/** The arrangement of a copy's relations as its source measured them, with the copy's predictions. */
export class Trace implements Arrangement {
    /** What the copy knows of its source. */
    readonly #upstream: Upstream;
    /** Each relation's measures by group name, read once per run. */
    readonly #read = new WeakMap<
        Run,
        Map<Node, Promise<ReadonlyMap<string, Readonly<Record<string, Scalar>>>>>
    >();
    /** The same measures once read. */
    readonly #loaded = new WeakMap<
        Run,
        Map<Node, ReadonlyMap<string, Readonly<Record<string, Scalar>>>>
    >();
    /** The measures last seen for each parent value, by relation and partition. */
    readonly #seen = new Map<Node, Map<string, string>>();
    /** Each relation's input. */
    readonly #inputs = new Map<Node, Input>();

    /** Create the trace of a copy. */
    constructor(upstream: Upstream) {
        this.#upstream = upstream;
    }

    /** Read the groups of every relation of a node once in the run. */
    async prepare(node: Node, _rows: readonly Row[], run: Run): Promise<void> {
        await this.read(node.relations, run);
    }

    /** Read the groups of some relations once in the run. */
    async read(relations: readonly Node[], run: Run): Promise<void> {
        await Promise.all(relations.map((relation) => this.#groups(relation, run)));
    }

    /** Look up a relation's measures of the rows naming a parent value. */
    measured(
        relation: Node,
        value: ColumnValue | undefined,
        run: Run,
    ): Readonly<Record<string, Scalar>> {
        const groups = this.#loaded.get(run)?.get(relation);
        if (groups === undefined) {
            throw new TypeError(`measures of ${relation.name} are read before they are prepared`);
        }

        return groups.get(canonicalize(relation.groupFor(value))) ?? {};
    }

    /** Decide again the holders with measures a run changed. */
    async relate(
        relations: readonly Node[],
        parentOf: (relation: Node) => Pipeline | undefined,
        run: Run,
    ): Promise<void> {
        for (const node of relations) {
            // follow relations of nodes with rows
            const parent = parentOf(node);
            if (parent === undefined) {
                continue;
            }

            // decide again the holders of the values the run changed
            const values = await this.#changedValues(node, run);
            await this.#redecideHolders(node, parent, values, run);
        }
    }

    /** Collect the parent values the run's changes touch for a relation. */
    async #changedValues(node: Node, run: Run): Promise<Map<string, ColumnValue>> {
        // collect the values the run's rows locate
        const input = this.#inputOf(node);
        const path = node.path;
        if (path === undefined) {
            throw new TypeError(`relation ${node.name} has no path`);
        }
        const link = node.link();
        const values = new Map<string, ColumnValue>();
        const name = (value: ColumnValue | undefined) => {
            if (value !== null && value !== undefined) {
                values.set(node.partition(value), value);
            }
        };
        const images = run.changes.flatMap((change) =>
            change.table === node.table
                ? [Change.before(change), Change.after(change)].filter(
                      (image): image is Row => image !== null,
                  )
                : [],
        );
        for (const located of await input.locate(images, run)) {
            located.forEach(name);
        }

        // collect the values the join rows and source groups point at
        for (const change of run.changes) {
            for (const image of [Change.before(change), Change.after(change)]) {
                if (image !== null && path.kind === "junction" && change.table === path.table) {
                    name(image[path.from.column]);
                }
            }
            const moved = this.#upstream.groupOf(change);
            const value = moved?.query === node.name ? moved.group[link.partitionName] : undefined;
            if (value !== null && value !== undefined) {
                name(link.parent.fromJson(link.parentColumn, value));
            }
        }

        return values;
    }

    /** Read a relation's input and start it on first use. */
    #inputOf(node: Node): Input {
        let input = this.#inputs.get(node);
        if (input === undefined) {
            input = Input.of(node, false);
            this.#inputs.set(node, input);
        }

        return input;
    }

    /** Decide again the parent rows holding the values whose measures changed since last seen. */
    async #redecideHolders(
        node: Node,
        parent: Pipeline,
        values: ReadonlyMap<string, ColumnValue>,
        run: Run,
    ): Promise<void> {
        // collect the values whose measures changed
        const seen = this.#seen.get(node) ?? new Map<string, string>();
        this.#seen.set(node, seen);
        await this.#groups(node, run);
        const moved: ColumnValue[] = [];
        for (const [partition, value] of values) {
            const measured = JSON.stringify(this.measured(node, value, run));
            if (seen.get(partition) !== measured) {
                seen.set(partition, measured);
                moved.push(value);
            }
        }

        // mark their holders dirty
        const column = node.link().parentColumn;
        const holders = await run.view.lookup(
            parent.node.table,
            moved.map((value) => ({ [column]: value })),
        );
        const dirty = run.workOf(parent).dirty;
        for (const row of holders.flat()) {
            dirty.set(parent.node.keyOf(row), row);
        }
    }

    /** Forget the measures seen. */
    forget(): void {
        this.#seen.clear();
    }

    /** Read a relation's groups once in the run. */
    #groups(
        relation: Node,
        run: Run,
    ): Promise<ReadonlyMap<string, Readonly<Record<string, Scalar>>>> {
        // reuse the run's read
        let reads = this.#read.get(run);
        if (reads === undefined) {
            reads = new Map();
            this.#read.set(run, reads);
        }
        const known = reads.get(relation);
        if (known !== undefined) {
            return known;
        }

        // read the groups
        const read = this.#upstream.groups(relation).then((groups) => {
            // index the groups by name
            const byName = new Map(
                groups.map((group) => [canonicalize(group.group), group.values]),
            );
            let loaded = this.#loaded.get(run);
            if (loaded === undefined) {
                loaded = new Map();
                this.#loaded.set(run, loaded);
            }
            loaded.set(relation, byName);

            return byName;
        });
        reads.set(relation, read);

        return read;
    }
}

/** One aggregate group as a copy's source measured it. */
export interface Group {
    /** The group's values in JSON form. */
    readonly group: Record<string, Scalar>;
    /** The group's measures by name. */
    readonly values: Record<string, Scalar>;
    /** The rows of the group. */
    readonly rows: number;
    /** Each average's sum and count of present values. */
    readonly averages: Record<string, Average>;
}
