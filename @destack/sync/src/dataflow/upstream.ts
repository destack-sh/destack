import type { Row } from "@destack/db";
import type { Scalar } from "@destack/db/query";
import type { Change } from "@destack/db/log";
import { canonicalize } from "@destack/schema/json";
import type { Node } from "../query/node.ts";
import type { Watch } from "../feed/audience.ts";
import type { Arrangement } from "./filter.ts";
import type { Pipeline } from "./pipeline.ts";
import { Input } from "./input.ts";
import type { Run } from "./run.ts";
import type { Part } from "./tally.ts";

/** What a copy knows of its source: the source's groups with the copy's predictions. */
export interface Upstream {
    /** The tables and scopes whose changes move the groups. */
    readonly watches: readonly Watch[];
    /** Read every group of an aggregate or relation node holding rows. */
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
    readonly #held = new WeakMap<
        Run,
        Map<Node, ReadonlyMap<string, Readonly<Record<string, Scalar>>>>
    >();
    /** The measures last seen for each held value, by relation and partition. */
    readonly #seen = new Map<Node, Map<string, string>>();
    /** Each relation's input. */
    readonly #inputs = new Map<Node, Input>();

    /** Create the trace of a copy. */
    constructor(upstream: Upstream) {
        this.#upstream = upstream;
    }

    /** Read the groups of every relation of a node once in the run. */
    async prepare(node: Node, _rows: readonly Row[], run: Run): Promise<void> {
        await Promise.all(node.relations.map((relation) => this.#groups(relation, run)));
    }

    /** Look up a relation's measures of the rows naming a held value. */
    measured(relation: Node, value: unknown, run: Run): Readonly<Record<string, Scalar>> {
        const groups = this.#held.get(run)?.get(relation);
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
            // follow relations of nodes holding rows
            const parent = parentOf(node);
            if (parent === undefined) {
                continue;
            }
            let input = this.#inputs.get(node);
            if (input === undefined) {
                input = Input.of(node, false);
                this.#inputs.set(node, input);
            }

            // name the values the run's rows and source groups name
            const path = node.path!;
            const values = new Map<string, unknown>();
            const name = (value: unknown) => {
                if (value !== null && value !== undefined) {
                    values.set(node.partition(value), value);
                }
            };
            const images = run.changes
                .filter((change) => change.table === node.table)
                .flatMap((change) => [change.before, change.after])
                .filter((image): image is Row => image !== undefined);
            for (const located of await input.locate(images, run)) {
                located.forEach(name);
            }
            for (const change of run.changes) {
                for (const image of [change.before, change.after]) {
                    if (
                        image !== undefined &&
                        path.kind === "junction" &&
                        change.table === path.table
                    ) {
                        name(image[path.from.column]);
                    }
                }
                const moved = this.#upstream.groupOf(change);
                const value =
                    moved?.query === node.name ? moved.group[node.partitionName!] : undefined;
                if (value !== null && value !== undefined) {
                    name(node.parent!.fromJson(node.parentColumn!, value));
                }
            }

            // decide again the holders of changed values
            const seen = this.#seen.get(node) ?? new Map<string, string>();
            this.#seen.set(node, seen);
            await this.#groups(node, run);
            const moved: unknown[] = [];
            for (const [partition, value] of values) {
                const measured = JSON.stringify(this.measured(node, value, run));
                if (seen.get(partition) !== measured) {
                    seen.set(partition, measured);
                    moved.push(value);
                }
            }
            const column = node.parentColumn!;
            const holders = await run.view.lookup(
                parent.node.table,
                moved.map((value) => ({ [column]: value })),
            );
            const dirty = run.workOf(parent).dirty;
            for (const row of holders.flat()) {
                dirty.set(parent.node.keyOf(row), row);
            }
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
            let held = this.#held.get(run);
            if (held === undefined) {
                held = new Map();
                this.#held.set(run, held);
            }
            held.set(relation, byName);

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
    /** The rows the group holds. */
    readonly rows: number;
    /** Each average's sum and count of present values. */
    readonly parts: Record<string, Part>;
}
