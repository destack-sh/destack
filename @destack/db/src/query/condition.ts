import { Param, sql, type SQL, type SQLWrapper } from "drizzle-orm";
import * as language from "@destack/schema/expression";
import type { Operand, Operator, Scalar } from "@destack/schema/expression";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Order } from "./order.ts";
import type { Namespace } from "./namespace.ts";
import { DatabaseError } from "../error/error.ts";
import { combine, inArray } from "./predicate.ts";

/**
 * The most terms one condition holds.
 *
 * At about 40 bytes a term, a condition renders at most about 40 KB of SQL.
 */
const CONDITION_TERMS = 1000;

/** The SQL each comparison operator renders. */
const OPERATORS = { eq: "=", ne: "<>", lt: "<", lte: "<=", gt: ">", gte: ">=" } as const;

/** How a condition's names resolve. */
export interface Binding<Value> {
    /** Resolve a column. */
    column(name: string): Value;
    /** Resolve a parameter's value. */
    parameter(name: string): Scalar;
    /** Resolve whether a related row meets a condition. */
    exists(via: string, where: Condition | undefined): Value;
}

export { Operand, Operator, Scalar } from "@destack/schema/expression";

/** A predicate over a row, decided alike in SQL and memory. */
export type Condition = language.Condition;

/** A predicate over a row: the language, with its SQL rendering and compiled matching over tables. */
export const Condition = {
    ...language.Condition,
    require: requireComparable,
    bind,
    render: renderCondition,
    compile,
    matches,
};

/** Decide a compiled condition on a row with relations unknown. */
function matches(match: Match, row: Readonly<Record<string, unknown>>): boolean {
    return (
        match({ column: (name) => row[name], parameter: () => null, exists: () => undefined }) ===
        true
    );
}

/** A compiled condition, deciding a row in three-valued logic. */
export type Match = (binding: Binding<unknown>) => boolean | undefined;

/** Require a bounded condition over comparable columns and computed values. */
function requireComparable(
    condition: Condition,
    table: Table,
    namespace: Namespace = { computed: {} },
): void {
    // bound the size
    const size = language.Condition.terms(condition);
    if (size > CONDITION_TERMS) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `condition holds ${size} terms, more than ${CONDITION_TERMS}`,
        );
    }

    // require comparable columns or computed values
    for (const name of language.Condition.columns(condition)) {
        if (!Object.hasOwn(namespace.computed, name)) {
            Order.column(table, name);
        }
    }
}

/** Bind a condition to a table, its namespace and parameters. */
function bind(
    table: Table,
    parameters: Readonly<Record<string, Scalar>> = {},
    namespace: Namespace = { computed: {} },
): Binding<SQLWrapper> {
    return {
        exists: (via, where) => {
            if (namespace.exists === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `condition follows relation ${via}, which ${table[TABLE].name} does not declare`,
                );
            }

            return namespace.exists(via, where);
        },
        column: (name) =>
            Object.hasOwn(namespace.computed, name)
                ? Order.expression(table, name, namespace)
                : Order.column(table, name),
        parameter: (name) => {
            if (!Object.hasOwn(parameters, name)) {
                throw new DatabaseError("INVALID_QUERY", `condition parameter is unbound: ${name}`);
            }

            return parameters[name]!;
        },
    };
}

/** Render a condition as SQL. */
function renderCondition(condition: Condition, binding: Binding<SQLWrapper>): SQL {
    switch (condition.kind) {
        case "compare": {
            const isOrdered = condition.operator !== "eq" && condition.operator !== "ne";
            const left = render(condition.left, condition.right, binding, isOrdered);
            const right = render(condition.right, condition.left, binding, isOrdered);

            return sql`(${left} ${sql.raw(OPERATORS[condition.operator])} ${right})`;
        }
        case "in": {
            // match an OR chain, or an IN list beyond the chain limit
            const operand = render(condition.operand, undefined, binding, false);
            const values = condition.values.map((value) =>
                render(language.Condition.literal(value), condition.operand, binding, false),
            );

            return inArray(operand, values);
        }
        case "null":
            return sql`(${render(condition.operand, undefined, binding, false)} IS NULL)`;
        case "all":
        case "any": {
            // render empty conjunctions true and disjunctions false
            if (condition.conditions.length === 0) {
                return condition.kind === "all" ? sql`true` : sql`false`;
            }
            const joined = condition.conditions.map((entry) => renderCondition(entry, binding));

            return combine(joined, condition.kind === "all" ? "AND" : "OR");
        }
        case "not":
            return sql`(NOT ${renderCondition(condition.condition, binding)})`;
        case "exists":
            return sql`${binding.exists(condition.via, condition.where)}`;
    }
}

/** Compile a condition over a table. */
function compile(condition: Condition, table: Table): Match {
    switch (condition.kind) {
        case "compare": {
            const left = operandOf(condition.left, condition.right, table);
            const right = operandOf(condition.right, condition.left, table);
            const operator = condition.operator;

            return (binding) => {
                const order = Order.values(left(binding), right(binding));

                return order === undefined ? undefined : holds(operator, order);
            };
        }
        case "in": {
            // match nothing for an empty list and unknown for a missing operand
            const operand = operandOf(condition.operand, undefined, table);
            const column = condition.operand.kind === "column" ? condition.operand.name : undefined;
            const values =
                column === undefined
                    ? [...condition.values]
                    : condition.values.map((entry) => decode(table, column, entry));

            return (binding) => {
                const value = operand(binding);
                if (values.length === 0) {
                    return false;
                } else if (value === null || value === undefined) {
                    return undefined;
                }

                return values.some((entry) => Order.values(value, entry) === 0);
            };
        }
        case "null": {
            const operand = operandOf(condition.operand, undefined, table);

            return (binding) => {
                const value = operand(binding);

                return value === null || value === undefined;
            };
        }
        case "all":
        case "any": {
            // settle on the deciding value
            const isDecidedBy = condition.kind === "any";
            const compiled = condition.conditions.map((entry) => compile(entry, table));

            return (binding) => {
                let isUnknown = false;
                for (const match of compiled) {
                    const matched = match(binding);
                    if (matched === isDecidedBy) {
                        return isDecidedBy;
                    }
                    isUnknown ||= matched === undefined;
                }

                return isUnknown ? undefined : !isDecidedBy;
            };
        }
        case "exists": {
            // ask the binding
            const { via, where } = condition;

            return (binding) => binding.exists(via, where) as boolean | undefined;
        }
        case "not": {
            const match = compile(condition.condition, table);

            return (binding) => {
                const matched = match(binding);

                return matched === undefined ? undefined : !matched;
            };
        }
    }
}

/** Render one operand. */
function render(
    operand: Operand,
    other: Operand | undefined,
    binding: Binding<SQLWrapper>,
    isOrdered: boolean,
): SQLWrapper {
    // read a column, ordering text by byte
    if (operand.kind === "column") {
        const resolved = binding.column(operand.name);

        return isOrdered && resolved instanceof Column ? Order.text(resolved) : resolved;
    }

    // bind a value through the other side's column
    const scalar = operand.kind === "parameter" ? binding.parameter(operand.name) : operand.value;
    const target = other?.kind === "column" ? binding.column(other.name) : undefined;

    return target instanceof Column && scalar !== null
        ? sql`${new Param(target.definition.fromJson(scalar), target)}`
        : sql`${scalar}`;
}

/** Compile an operand's value. */
function operandOf(
    operand: Operand,
    other: Operand | undefined,
    table: Table,
): (binding: Binding<unknown>) => unknown {
    // read a column's value
    if (operand.kind === "column") {
        const name = operand.name;

        return (binding) => binding.column(name);
    }

    // decode a parameter on each evaluation
    const column = other?.kind === "column" ? other.name : undefined;
    if (operand.kind === "parameter") {
        const name = operand.name;

        return (binding) => {
            const scalar = binding.parameter(name);

            return column === undefined || scalar === null ? scalar : decode(table, column, scalar);
        };
    }

    // decode a literal once
    const value =
        column === undefined || operand.value === null
            ? operand.value
            : decode(table, column, operand.value);

    return () => value;
}

/** Decode a scalar as a column holds it. */
function decode(table: Table, column: string, scalar: Scalar): unknown {
    const definition = table[TABLE].columns[column]?.definition;

    return definition === undefined ? scalar : definition.fromJson(scalar);
}

/** Decide whether an operator holds for an order. */
function holds(operator: Operator, order: number): boolean {
    switch (operator) {
        case "eq":
            return order === 0;
        case "ne":
            return order !== 0;
        case "lt":
            return order < 0;
        case "lte":
            return order <= 0;
        case "gt":
            return order > 0;
        case "gte":
            return order >= 0;
    }
}
