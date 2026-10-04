import {
    type Row,
    type ColumnValue,
    Predicate,
    type Match,
    type Scalar,
    Expression,
    type RelationReader,
    DatabaseError,
} from "@destack/db";
import { measureName, type Node } from "../query/node.ts";
import type { Run } from "./run.ts";

/** Decide a node's rows: compute their values and keep the visible candidates. */
export class Filter {
    /** The node whose rows the filter decides. */
    readonly node: Node;
    /** The arrangement of the relations' measures. */
    readonly #arrangement: Arrangement;
    /** The computed values' names and expressions. */
    readonly #formulas: readonly (readonly [string, Expression])[];
    /** The node's compiled scopes and condition. */
    readonly #match: Match;
    /** Each row with its computed values, for nodes without lookups. */
    readonly #computed = new WeakMap<Row, Row>();
    /** The rows with their computed values. */
    readonly #resolved = new WeakSet<Row>();

    /** Create the filter of a node. */
    constructor(node: Node, arrangement: Arrangement) {
        // compile the computed values and condition
        this.node = node;
        this.#arrangement = arrangement;
        this.#formulas = Object.entries(node.extras);
        this.#match = Predicate.compile(node.resolve(node.condition()), node.table);
    }

    /** Decide some rows' visibility and read their relations' measures. */
    async prepare(rows: readonly Row[], run: Run): Promise<void> {
        await run.decide(this.node.table, rows);
        await this.measure(rows, run);
    }

    /** Read the relations' measures of some rows or positions, which their computed values need. */
    async measure(rows: readonly Row[], run: Run): Promise<void> {
        if (this.node.relations.length > 0) {
            await this.#arrangement.prepare(this.node, rows, run);
        }
    }

    /** Decide whether a prepared row is a visible, matching candidate. */
    isCandidate(row: Row, run: Run): boolean {
        return run.isVisible(row) && this.meets(row, run);
    }

    /** Decide whether a prepared row is in the node's scopes and meets its condition. */
    meets(row: Row, run: Run): boolean {
        const resolved = this.resolve(row, run);

        return (
            this.#match({
                field: (name) => resolved[name],
                placeholder: Predicate.unbound,
                exists: (via, where) => {
                    // count the related rows meeting the condition
                    const relation = this.node.relation(via, where);
                    const measured = this.#arrangement.measured(
                        relation,
                        relation.valueOf(row),
                        run,
                    );
                    const count = measured["count"];

                    return typeof count === "number" && count > 0;
                },
            }) === true
        );
    }

    /** Read a prepared row with the node's computed values. */
    resolve(row: Row, run: Run): Row {
        // keep rows computing nothing or already resolved
        if (this.#formulas.length === 0 || this.#resolved.has(row)) {
            return row;
        }

        // reuse the row's computed values
        const byRow = this.node.isRelating ? run.resolvedBy(this) : this.#computed;
        const known = byRow.get(row);
        if (known !== undefined) {
            return known;
        }

        // compute each value
        const related = this.related(row, run);
        const values: Record<string, ColumnValue> = { ...row };
        for (const [name, expression] of this.#formulas) {
            values[name] = Expression.evaluate(expression, row, related);
        }
        byRow.set(row, values);
        this.#resolved.add(values);

        return values;
    }

    /** Read a prepared row's lookups and rollups. */
    related(row: Row, run: Run): RelationReader {
        const node = this.node;

        return {
            lookup: (via, column) => {
                // read the related row's column as the minimum over its one related row
                const relation = node.relation(via, {});
                const measured = this.#arrangement.measured(relation, relation.valueOf(row), run);

                return relation.fromJson(column, measured[measureName("min", column)] ?? null);
            },
            rollup: (measure, via, column, where) => {
                // read the relation's measures of the row
                const relation = node.relation(via, where);
                const measured = this.#arrangement.measured(relation, relation.valueOf(row), run);

                // count the related rows as zero without a group
                if (measure === "count") {
                    return measured["count"] ?? 0;
                }

                // read another measure of a column
                if (column === undefined) {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `rollup ${measure} over ${via} has no column`,
                    );
                }

                return relation.fromJson(column, measured[measureName(measure, column)] ?? null);
            },
        };
    }

    /** Take a row as having its computed values. */
    adopt(row: Row): Row {
        this.#resolved.add(row);

        return row;
    }
}

/** The groups of a query's relations, by the parent value naming them. */
export interface Arrangement {
    /** Read the relation measures of some rows at once. */
    prepare(node: Node, rows: readonly Row[], run: Run): Promise<void>;
    /** Read a relation's measures of the rows naming a parent value. */
    measured(
        relation: Node,
        value: ColumnValue | undefined,
        run: Run,
    ): Readonly<Record<string, Scalar>>;
}
