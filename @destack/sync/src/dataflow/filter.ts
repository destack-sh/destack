import type { Row } from "@destack/db";
import { Condition, Expression, type Match, type Related, type Scalar } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { measureName, type Node } from "../query/node.ts";
import type { Run } from "./run.ts";

/**
 * Decide a node's rows: compute their values, and keep those of its scopes meeting its condition that the audience sees.
 *
 * A batch prepares at once what its rows' decisions read: their visibility, and their relations' measures.
 */
export class Filter {
    /** The node whose rows the filter decides. */
    readonly node: Node;
    /** The arrangement holding the relations' measures. */
    readonly #arrangement: Arrangement;
    /** The computed values' names and expressions, in order. */
    readonly #formulas: readonly (readonly [string, Expression])[];
    /** The node's scopes and condition, compiled to decide rows in memory. */
    readonly #match: Match;
    /** Each row with its computed values, by the row it computes them from, for nodes looking nothing up. */
    readonly #computed = new WeakMap<Row, Row>();
    /** The rows holding their computed values. */
    readonly #resolved = new WeakSet<Row>();

    /** Decide a node's rows, reading its relations' measures from an arrangement. */
    constructor(node: Node, arrangement: Arrangement) {
        // compile the node's computed values and condition once
        this.node = node;
        this.#arrangement = arrangement;
        this.#formulas = Object.entries(node.computed);
        this.#match = Condition.compile(node.condition(), node.table);
    }

    /** Decide the visibility of some rows and read their relations' measures, at once. */
    async prepare(rows: readonly Row[], run: Run): Promise<void> {
        await run.decide(this.node.table, rows);
        if (this.node.relations.length > 0) {
            await this.#arrangement.prepare(this.node, rows, run);
        }
    }

    /** Decide whether a prepared row is a candidate: seen by the audience, of the node's scopes and meeting its condition. */
    isCandidate(row: Row, run: Run): boolean {
        return run.isVisible(row) && this.meets(row, run);
    }

    /** Decide whether a prepared row lives in the node's scopes and meets its condition, answering relations by their counts. */
    meets(row: Row, run: Run): boolean {
        const resolved = this.resolve(row, run);

        return (
            this.#match({
                column: (name) => resolved[name],
                parameter: unbound,
                exists: (via, where) => {
                    const relation = this.node.relation(via, where);
                    const count = this.#arrangement.measured(relation, relation.valueOf(row), run)
                        .count as number | undefined;

                    return (count ?? 0) > 0;
                },
            }) === true
        );
    }

    /**
     * Read a prepared row with the node's computed values, once per row, and once per run for nodes reading relations.
     *
     * Lookups and rollups read the relations' measures at the run's position, so one row may compute other values in other runs.
     */
    resolve(row: Row, run: Run): Row {
        // keep rows computing nothing, and rows holding their values already
        if (this.#formulas.length === 0 || this.#resolved.has(row)) {
            return row;
        }

        // reuse the values computed for the row, per run where the node reads relations
        const byRow = this.node.isRelating ? run.resolvedBy(this) : this.#computed;
        const known = byRow.get(row);
        if (known !== undefined) {
            return known;
        }

        // compute each value, reading lookups and rollups from the measures
        const related = this.related(row, run);
        const values: Record<string, unknown> = { ...row };
        for (const [name, expression] of this.#formulas) {
            values[name] = Expression.evaluate(expression, row, related);
        }
        byRow.set(row, values);
        this.#resolved.add(values);

        return values;
    }

    /** Read what a prepared row's lookups and rollups read: the one related row's column, or a measure of the related rows. */
    related(row: Row, run: Run): Related {
        const node = this.node;

        return {
            lookup: (via, column) => {
                const relation = node.relation(via, undefined);
                const measured = this.#arrangement.measured(relation, relation.valueOf(row), run);

                return relation.fromJson(
                    column,
                    measured[measureName("min", column)] ?? null,
                ) as Scalar;
            },
            rollup: (measure, via, column, where) => {
                const relation = node.relation(via, where);
                const measured = this.#arrangement.measured(relation, relation.valueOf(row), run);

                return measure === "count"
                    ? (measured.count ?? 0)
                    : (relation.fromJson(
                          column!,
                          measured[measureName(measure, column!)] ?? null,
                      ) as Scalar);
            },
        };
    }

    /** Take a row as holding its computed values already, as a read selecting them returns it. */
    adopt(row: Row): Row {
        this.#resolved.add(row);

        return row;
    }
}

/**
 * The groups of a query's relations, indexed by the held value naming them, which filters look up.
 *
 * A dataflow arranges them in its own relations' tallies; a copy reads its source's.
 */
export interface Arrangement {
    /** Read the measures of the relations of some rows of a node at once, so that filters read them synchronously. */
    prepare(node: Node, rows: readonly Row[], run: Run): Promise<void>;
    /** Read a relation's measures of the rows naming a held value: none when no row names it. */
    measured(relation: Node, value: unknown, run: Run): Readonly<Record<string, Scalar>>;
}

/** Refuse parameters, which query conditions never bind. */
function unbound(name: string): never {
    throw new DatabaseError("INVALID_QUERY", `query conditions bind no parameters: ${name}`);
}
