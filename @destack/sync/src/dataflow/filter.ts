import type { Row } from "@destack/db";
import { Condition, type Match, type Scalar } from "@destack/db/query";
import { Expression, type Related } from "@destack/db/expression";
import { DatabaseError } from "@destack/db/error";
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
    /** The rows holding their computed values. */
    readonly #resolved = new WeakSet<Row>();

    /** Create the filter of a node. */
    constructor(node: Node, arrangement: Arrangement) {
        // compile the computed values and condition
        this.node = node;
        this.#arrangement = arrangement;
        this.#formulas = Object.entries(node.computed);
        this.#match = Condition.compile(node.condition(), node.table);
    }

    /** Decide some rows' visibility and read their relations' measures. */
    async prepare(rows: readonly Row[], run: Run): Promise<void> {
        await run.decide(this.node.table, rows);
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
        const values: Record<string, unknown> = { ...row };
        for (const [name, expression] of this.#formulas) {
            values[name] = Expression.evaluate(expression, row, related);
        }
        byRow.set(row, values);
        this.#resolved.add(values);

        return values;
    }

    /** Read a prepared row's lookups and rollups. */
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

    /** Take a row as holding its computed values. */
    adopt(row: Row): Row {
        this.#resolved.add(row);

        return row;
    }
}

/** The groups of a query's relations, by the held value naming them. */
export interface Arrangement {
    /** Read the relation measures of some rows at once. */
    prepare(node: Node, rows: readonly Row[], run: Run): Promise<void>;
    /** Read a relation's measures of the rows naming a held value. */
    measured(relation: Node, value: unknown, run: Run): Readonly<Record<string, Scalar>>;
}

/** Refuse parameters. */
function unbound(name: string): never {
    throw new DatabaseError("INVALID_QUERY", `query conditions bind no parameters: ${name}`);
}
