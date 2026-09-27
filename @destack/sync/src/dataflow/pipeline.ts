import type { DatabaseConnection, Row } from "@destack/db";
import type { Change, RelationView } from "@destack/db/log";
import type { Node } from "../query/node.ts";
import type { Audience, Watch } from "../feed/audience.ts";
import { Filter, type Arrangement } from "./filter.ts";
import { Input } from "./input.ts";
import type { Run, Work } from "./run.ts";
import type { Sink } from "./sink.ts";
import type { Upstream } from "./upstream.ts";
import type { Segment } from "./view.ts";
import type { Window } from "./window.ts";

/** The partitions of a row no partition holds. */
export const NOWHERE: ReadonlySet<string> = new Set();

/**
 * One node's operators, with what it holds as of its dataflow's position: its open partitions and members.
 *
 * A run hands each pipeline its work at once: partitions to fill and empty, and rows to decide again.
 */
export abstract class Pipeline {
    /** The node the pipeline evaluates. */
    readonly node: Node;
    /** Where the node's rows come from: a scan of its scopes, or a join to its parent's rows. */
    readonly input: Input;
    /** How the node decides its rows. */
    readonly filter: Filter;
    /** What the pipeline shares with the others of its dataflow. */
    readonly context: Context;
    /** The pipeline of the parent node, absent for a root. */
    parent: Pipeline | undefined;
    /** The pipelines of the node's children, in child order, absent for children keeping no state. */
    children: readonly (Pipeline | undefined)[] = [];
    /** The open partitions, by name. */
    readonly partitions = new Map<string, Partition>();
    /** The rows the node holds, by key. */
    readonly members = new Map<string, Member>();

    /** Evaluate a node within a dataflow. */
    constructor(node: Node, context: Context, hasTreeIndex: boolean) {
        // read and decide the node's rows
        this.node = node;
        this.context = context;
        this.input = Input.of(node, hasTreeIndex);
        this.filter = new Filter(node, context.arrangement);
    }

    /** Apply a run's work: empty closed partitions, fill opened ones, and decide rows again. */
    abstract step(work: Work, run: Run): Promise<void>;

    /** The rows and groups the pipeline knows, which count against the capacity. */
    abstract get size(): number;

    /** Forget everything the pipeline holds. */
    forget(): void {
        this.partitions.clear();
        this.members.clear();
    }

    /** Open a root's one partition, which never closes, starting after a row when given one. */
    openRoot(start?: Row): void {
        this.partitions.set("", {
            ...partitionOf(undefined, 1),
            ...(start === undefined ? {} : { start }),
        });
    }

    /** Count a held value one holder more or less, opening its partition with its first holder and closing it after its last. */
    hold(value: unknown, step: 1 | -1, run: Run): void {
        // name the partition and the run's work
        const name = this.node.partition(value);
        const partition = this.partitions.get(name);
        const work = run.workOf(this);

        // open a partition its first holder names, keeping one its last holder left in this run
        if (step > 0 && partition === undefined) {
            this.partitions.set(name, partitionOf(value, 1));
            work.opened.set(name, value);
        } else if (step > 0) {
            partition!.holders += 1;
            work.closed.delete(name);
        }
        // forget a partition the run opened and never filled once its last holder leaves, and close a filled one
        else if (partition !== undefined && --partition.holders === 0) {
            if (work.opened.delete(name)) {
                this.partitions.delete(name);
            } else {
                work.closed.add(name);
            }
        }
    }

    /** Decide rows again as they are at the run's position: whether each is a candidate, and which open partitions hold it. */
    async decide(rows: ReadonlyMap<string, Row | undefined>, run: Run): Promise<void> {
        // decide the rows' visibility and relations together, then locate the candidates' partitions at once
        if (rows.size === 0) {
            return;
        }
        run.decided += rows.size;
        const present = [...rows.values()].filter((row): row is Row => row !== undefined);
        await this.filter.prepare(present, run);
        const candidates = present.filter((row) => this.filter.isCandidate(row, run));
        const located = await this.input.locate(candidates, run);
        const partitions = new Map(candidates.map((row, index) => [row, located[index]!]));

        // admit each row to the open partitions holding it now
        for (const [key, row] of rows) {
            const values = row === undefined ? undefined : partitions.get(row);
            const names = new Set<string>();
            for (const value of values ?? []) {
                const name = this.node.partition(value);
                if (this.partitions.has(name)) {
                    names.add(name);
                }
            }
            this.admit(
                key,
                values === undefined ? undefined : this.filter.resolve(row!, run),
                names,
                run,
            );
        }
    }

    /** Admit a decided row to the partitions holding it, as it is at the position, absent once it is no candidate. */
    protected abstract admit(
        key: string,
        row: Row | undefined,
        names: ReadonlySet<string>,
        run: Run,
    ): void;

    /** Move a row between the partitions' members, returning the member as it was. */
    protected move(key: string, names: ReadonlySet<string>): Member | undefined {
        // leave the partitions no longer holding the row, and enter the new ones
        const member = this.members.get(key);
        const before = member?.partitions ?? NOWHERE;
        for (const name of before) {
            if (!names.has(name)) {
                this.partitions.get(name)?.members.delete(key);
            }
        }
        for (const name of names) {
            if (!before.has(name)) {
                this.partitions.get(name)!.members.add(key);
            }
        }

        return member;
    }

    /** Read each partition's candidates with their computed values, aligned with the held values naming them. */
    protected async matched(values: readonly unknown[], run: Run): Promise<Row[][]> {
        // read the partitions' rows, deciding their visibility and relations together
        const members = await this.input.members(values, run);
        await this.filter.prepare(members.flat(), run);

        // keep the candidates with their computed values
        return members.map((rows) =>
            rows
                .filter((row) => this.filter.isCandidate(row, run))
                .map((row) => this.filter.resolve(row, run)),
        );
    }

    /**
     * Read up to a count of each segment's candidates, in the node's order after its row, aligned with the segments.
     *
     * A root or a partition a key selects reads in the database's order, one prepared statement per partition at once.
     * Other paths read through the path, and so does a copy's node reading relations, which its source measures instead of its own rows.
     */
    protected async ordered(
        segments: readonly Segment[],
        count: number,
        run: Run,
    ): Promise<Row[][]> {
        // read in the database's order
        const node = this.node;
        const isMeasuredUpstream = this.context.upstream !== undefined && node.relations.length > 0;
        if ((node.path === undefined || node.path.kind === "key") && !isMeasuredUpstream) {
            const read = await run.view.ordered(
                node,
                segments,
                count,
                this.context.audience,
                run,
                node.relations.length === 0 ? undefined : this.relationView(run),
            );

            return read.map((rows) => rows.map((row) => this.filter.adopt(row)));
        }

        // read through the paths, keeping the candidates after each segment's row, in order
        const matched = await this.matched(
            segments.map((segment) => segment.value),
            run,
        );

        return matched.map((rows, index) => {
            const after = segments[index]!.after;

            return rows
                .filter((row) => after === undefined || node.compare(row, after) > 0)
                .sort((left, right) => node.compare(left, right))
                .slice(0, count);
        });
    }

    /** Follow the node's relations as of the run's position, which an ordered read decides changed rows through. */
    protected relationView(run: Run): RelationView {
        const node = this.node;
        const arrangement = this.context.arrangement;

        return {
            decide: async (via, where, row) => {
                // answer by the relation's count of rows naming the row
                const relation = node.relation(via, where);
                await arrangement.prepare(node, [row], run);
                const count = arrangement.measured(relation, relation.valueOf(row), run).count as
                    | number
                    | undefined;

                return (count ?? 0) > 0;
            },
            touched: (after, upto) => run.view.dependents(node, after, upto),
            resolve: async (row) => {
                await arrangement.prepare(node, [row], run);

                return this.filter.related(row, run);
            },
        };
    }
}

/** What a dataflow's pipelines share: who they serve, where they send rows, where measures and changes come from. */
export interface Context {
    /** Who the dataflow serves. */
    readonly audience: Audience;
    /** What the subscriber holds. */
    readonly sink: Sink;
    /** The groups of the relations filters look up. */
    readonly arrangement: Arrangement;
    /** What a copy knows of its source, absent for a source database. */
    readonly upstream: Upstream | undefined;
    /** The database read. */
    readonly database: DatabaseConnection;
    /** Whether selections hold their rows, so that the dataflow reads its results without reading the database. */
    readonly isMaterialized: boolean;
    /** Read every change of watched tables after one sequence through another, absent once the log no longer holds them. */
    changesThrough(
        watches: readonly Watch[],
        after: number,
        through: number,
    ): Promise<Change[] | undefined>;
}

/** A partition of a node that held parent rows open: the rows it holds, in its window when the node is limited. */
export interface Partition {
    /** The held parent rows' value naming it, absent for a root's one partition. */
    readonly value: unknown;
    /** How many held parent rows name it; a root's one partition never closes. */
    holders: number;
    /** The keys of the rows it holds. */
    readonly members: Set<string>;
    /** The first rows of the node's order, when the node is limited. */
    window: Window | undefined;
    /** The log sequence a count of it read at, which later changes continue. */
    sequence: number;
    /** The row a root's one partition starts after, absent from the start of its order. */
    start?: Row;
}

/** A row a node holds: the partitions holding it, and the value naming each child's partition, in child order. */
export interface Member {
    /** The partitions holding the row. */
    partitions: ReadonlySet<string>;
    /** The value naming each child's partition, in child order. */
    joins: readonly unknown[];
    /** The row with its computed values as of the dataflow's position, held when the dataflow materializes its rows. */
    row?: Row;
}

/** Start a partition some held parent rows open. */
export function partitionOf(value: unknown, holders: number): Partition {
    return { value, holders, members: new Set(), window: undefined, sequence: -1 };
}

/** Copy a set of partitions without one. */
export function without(names: ReadonlySet<string>, name: string): Set<string> {
    // copy the set, then take the one out
    const next = new Set(names);
    next.delete(name);

    return next;
}

/** Name rows of a node by key. */
export function keyed(node: Node, rows: readonly Row[]): Map<string, Row> {
    return new Map(rows.map((row) => [node.keyOf(row), row]));
}
