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

/** One node's operators, with its open partitions and members. */
export abstract class Pipeline {
    /** The node the pipeline evaluates. */
    readonly node: Node;
    /** The source of the node's rows. */
    readonly input: Input;
    /** How the node decides its rows. */
    readonly filter: Filter;
    /** The dataflow's shared context. */
    readonly context: Context;
    /** The pipeline of the parent node, absent for a root. */
    parent: Pipeline | undefined;
    /** The pipelines of the node's children, absent for stateless children. */
    children: readonly (Pipeline | undefined)[] = [];
    /** The open partitions, by name. */
    readonly partitions = new Map<string, Partition>();
    /** The rows the node holds, by key. */
    readonly members = new Map<string, Member>();

    /** Create the pipeline of a node. */
    constructor(node: Node, context: Context, hasTreeIndex: boolean) {
        // read and decide the node's rows
        this.node = node;
        this.context = context;
        this.input = Input.of(node, hasTreeIndex);
        this.filter = new Filter(node, context.arrangement);
    }

    /** Apply a run's work: empty closed partitions, fill opened ones, and decide rows again. */
    abstract step(work: Work, run: Run): Promise<void>;

    /** The rows and groups the pipeline knows. */
    abstract get size(): number;

    /** Forget everything the pipeline holds. */
    forget(): void {
        this.partitions.clear();
        this.members.clear();
    }

    /** Open a root's one partition, starting after a row when given one. */
    openRoot(start?: Row): void {
        this.partitions.set("", {
            ...partitionOf(undefined, 1),
            ...(start === undefined ? {} : { start }),
        });
    }

    /** Count a held value's holders up or down, opening or closing its partition. */
    hold(value: unknown, step: 1 | -1, run: Run): void {
        // name the partition and the run's work
        const name = this.node.partition(value);
        const partition = this.partitions.get(name);
        const work = run.workOf(this);

        // open a partition for its first holder
        if (step > 0 && partition === undefined) {
            this.partitions.set(name, partitionOf(value, 1));
            work.opened.set(name, value);
        } else if (step > 0) {
            partition!.holders += 1;
            work.closed.delete(name);
        }
        // close a partition after its last holder
        else if (partition !== undefined && --partition.holders === 0) {
            if (work.opened.delete(name)) {
                this.partitions.delete(name);
            } else {
                work.closed.add(name);
            }
        }
    }

    /** Decide rows again as of the run's position. */
    async decide(rows: ReadonlyMap<string, Row | undefined>, run: Run): Promise<void> {
        // decide the rows and locate the candidates' partitions
        if (rows.size === 0) {
            return;
        }
        run.decided += rows.size;
        const present = [...rows.values()].filter((row): row is Row => row !== undefined);
        await this.filter.prepare(present, run);
        const candidates = present.filter((row) => this.filter.isCandidate(row, run));
        const located = await this.input.locate(candidates, run);
        const partitions = new Map(candidates.map((row, index) => [row, located[index]!]));

        // admit each row to its open partitions
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

    /** Admit a decided row to its partitions. */
    protected abstract admit(
        key: string,
        row: Row | undefined,
        names: ReadonlySet<string>,
        run: Run,
    ): void;

    /** Move a row between partitions and return the previous member. */
    protected move(key: string, names: ReadonlySet<string>): Member | undefined {
        // leave the old partitions and enter the new ones
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

    /** Read each partition's candidates with their computed values. */
    protected async matched(values: readonly unknown[], run: Run): Promise<Row[][]> {
        // read and decide the partitions' rows
        const members = await this.input.members(values, run);
        await this.filter.prepare(members.flat(), run);

        // keep the candidates with their computed values
        return members.map((rows) =>
            rows
                .filter((row) => this.filter.isCandidate(row, run))
                .map((row) => this.filter.resolve(row, run)),
        );
    }

    /** Read up to a count of each segment's candidates in the node's order after its row. */
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

        // read through the paths and keep the candidates after each segment's row
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

    /** Follow the node's relations as of the run's position. */
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

/** The shared state of a dataflow's pipelines. */
export interface Context {
    /** Who the dataflow serves. */
    readonly audience: Audience;
    /** What the subscriber holds. */
    readonly sink: Sink;
    /** The groups of the relations filters look up. */
    readonly arrangement: Arrangement;
    /** What a copy knows of its source. */
    readonly upstream: Upstream | undefined;
    /** The database read. */
    readonly database: DatabaseConnection;
    /** Whether selections hold their rows. */
    readonly isMaterialized: boolean;
    /** Read the watched changes between two sequences, absent once compacted. */
    changesThrough(
        watches: readonly Watch[],
        after: number,
        through: number,
    ): Promise<Change[] | undefined>;
}

/** A partition of a node that held parent rows opened. */
export interface Partition {
    /** The held parent rows' value naming it, absent for a root. */
    readonly value: unknown;
    /** How many held parent rows name it. */
    holders: number;
    /** The keys of the rows it holds. */
    readonly members: Set<string>;
    /** The first rows of the node's order, when the node is limited. */
    window: Window | undefined;
    /** The log sequence its count read at. */
    sequence: number;
    /** The row a root's partition starts after. */
    start?: Row;
}

/** A row a node holds. */
export interface Member {
    /** The partitions holding the row. */
    partitions: ReadonlySet<string>;
    /** The value naming each child's partition, in child order. */
    joins: readonly unknown[];
    /** The row with its computed values, held when the dataflow materializes. */
    row?: Row;
}

/** Create a partition. */
export function partitionOf(value: unknown, holders: number): Partition {
    return { value, holders, members: new Set(), window: undefined, sequence: -1 };
}

/** Copy a set of partitions without one. */
export function without(names: ReadonlySet<string>, name: string): Set<string> {
    // copy the set without the name
    const next = new Set(names);
    next.delete(name);

    return next;
}

/** Key rows of a node. */
export function keyed(node: Node, rows: readonly Row[]): Map<string, Row> {
    return new Map(rows.map((row) => [node.keyOf(row), row]));
}
