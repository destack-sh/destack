import { Param, sql, type SQL, type SQLWrapper } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Order } from "./order.ts";
import type { Namespace } from "./namespace.ts";
import { DatabaseError } from "../error/error.ts";
import { combine, inArray } from "./predicate.ts";

/**
 * The most terms one condition holds, counting groups and each listed value.
 *
 * At about 40 bytes of SQL a term, a condition renders at most about 40 KB, which Turso prepares in about 4 ms.
 */
const CONDITION_TERMS = 1000;

/** The SQL each comparison operator renders. */
const OPERATORS = { eq: "=", ne: "<>", lt: "<", lte: "<=", gt: ">", gte: ">=" } as const;

/** A scalar a condition compares, in its JSON form. */
export const Scalar = defineSchema(
    schema.union([schema.string(), schema.number().finite(), schema.boolean(), schema.null()]),
);
/** A scalar a condition compares, in its JSON form. */
export type Scalar = schema.Infer<typeof Scalar>;

/** A comparison operator. */
export const Operator = defineSchema(schema.enum(["eq", "ne", "lt", "lte", "gt", "gte"]));
/** A comparison operator. */
export type Operator = schema.Infer<typeof Operator>;

/** One side of a comparison: a column by property, a literal, or a parameter bound per evaluation. */
export const Operand = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Read a column. */
            kind: schema.literal("column"),
            /** The column's property. */
            name: schema.string().min(1),
        }),
        schema.object({
            /** Embed a value. */
            kind: schema.literal("literal"),
            /** The value, in the JSON form of the column it meets. */
            value: Scalar,
        }),
        schema.object({
            /** Read a parameter. */
            kind: schema.literal("parameter"),
            /** The parameter's name. */
            name: schema.string().min(1),
        }),
    ]),
);
/** One side of a comparison: a column by property, a literal, or a parameter bound per evaluation. */
export type Operand = schema.Infer<typeof Operand>;

/** How a condition's names resolve: columns by property, parameters by name, and relations by the name the host declares. */
export interface Binding<Value> {
    /** Resolve a column by property: SQL when rendering, the row's value when matching. */
    column(name: string): Value;
    /** Resolve a parameter's value. */
    parameter(name: string): Scalar;
    /** Resolve whether a related row meets a condition: SQL when rendering, the answer when matching. */
    exists(via: string, where: Condition | undefined): Value;
}

/** A predicate over a row's columns, which SQL and memory decide alike in three-valued logic. */
export type Condition =
    | {
          readonly kind: "compare";
          readonly operator: Operator;
          readonly left: Operand;
          readonly right: Operand;
      }
    | {
          readonly kind: "in";
          readonly operand: Operand;
          readonly values: readonly Exclude<Scalar, null>[];
      }
    | { readonly kind: "null"; readonly operand: Operand }
    | { readonly kind: "all" | "any"; readonly conditions: readonly Condition[] }
    | { readonly kind: "not"; readonly condition: Condition }
    | { readonly kind: "exists"; readonly via: string; readonly where?: Condition };

/** The schema of a condition. */
const conditionSchema: schema.Schema<Condition> = schema.lazy(() =>
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Compare two operands. */
            kind: schema.literal("compare"),
            /** The operator. */
            operator: Operator,
            /** The left operand. */
            left: Operand,
            /** The right operand. */
            right: Operand,
        }),
        schema.object({
            /** Match one of some values. */
            kind: schema.literal("in"),
            /** The operand. */
            operand: Operand,
            /** The values, none of them missing. */
            values: schema.array(
                schema.union([schema.string(), schema.number().finite(), schema.boolean()]),
            ),
        }),
        schema.object({
            /** Match a missing value. */
            kind: schema.literal("null"),
            /** The operand. */
            operand: Operand,
        }),
        schema.object({
            /** Require every condition, or any. */
            kind: schema.enum(["all", "any"]),
            /** The conditions. */
            conditions: schema.array(conditionSchema),
        }),
        schema.object({
            /** Negate a condition. */
            kind: schema.literal("not"),
            /** The negated condition. */
            condition: conditionSchema,
        }),
        schema.object({
            /** Match rows a related row meets a condition for. */
            kind: schema.literal("exists"),
            /** The relation, by the name its host declares. */
            via: schema.string().min(1),
            /** The condition the related row meets, over its table's columns. */
            where: conditionSchema.optional(),
        }),
    ]),
);

/** A predicate over a row's columns, which SQL and memory decide alike in three-valued logic. */
export const Condition = {
    /** The schema of a condition. */
    schema: conditionSchema,
    compare,
    /** Match rows whose column equals a value. */
    eq: (left: Operand | string, right: Operand | Scalar) => compare("eq", left, right),
    /** Match rows whose column differs from a value. */
    ne: (left: Operand | string, right: Operand | Scalar) => compare("ne", left, right),
    /** Match rows whose column is below a value. */
    lt: (left: Operand | string, right: Operand | Scalar) => compare("lt", left, right),
    /** Match rows whose column is at most a value. */
    lte: (left: Operand | string, right: Operand | Scalar) => compare("lte", left, right),
    /** Match rows whose column is above a value. */
    gt: (left: Operand | string, right: Operand | Scalar) => compare("gt", left, right),
    /** Match rows whose column is at least a value. */
    gte: (left: Operand | string, right: Operand | Scalar) => compare("gte", left, right),
    oneOf,
    missing,
    all,
    any,
    not,
    exists,
    column,
    literal,
    parameter,
    columns,
    relations,
    require: requireComparable,
    bind,
    render: renderCondition,
    compile,
    matches,
};

/** Decide a compiled condition over columns on a row, as SQL does, leaving relations unknown. */
function matches(match: Match, row: Readonly<Record<string, unknown>>): boolean {
    return (
        match({ column: (name) => row[name], parameter: () => null, exists: () => undefined }) ===
        true
    );
}

/** A condition compiled over a table: it decides a row as SQL does, in three-valued logic. */
export type Match = (binding: Binding<unknown>) => boolean | undefined;

/** Compare a column with a value or another operand. */
function compare(operator: Operator, left: Operand | string, right: Operand | Scalar): Condition {
    return {
        kind: "compare",
        operator,
        left: typeof left === "string" ? column(left) : left,
        right: typeof right === "object" && right !== null ? right : literal(right),
    };
}

/** Match rows whose column holds one of some values. */
function oneOf(operand: Operand | string, values: readonly Exclude<Scalar, null>[]): Condition {
    return {
        kind: "in",
        operand: typeof operand === "string" ? column(operand) : operand,
        values,
    };
}

/** Match rows whose column holds no value. */
function missing(operand: Operand | string): Condition {
    return {
        kind: "null",
        operand: typeof operand === "string" ? column(operand) : operand,
    };
}

/** Require every condition. */
function all(...conditions: Condition[]): Condition {
    return { kind: "all", conditions };
}

/** Require any condition. */
function any(...conditions: Condition[]): Condition {
    return { kind: "any", conditions };
}

/** Negate a condition. */
function not(condition: Condition): Condition {
    return { kind: "not", condition };
}

/** Match rows a related row meets a condition for. */
function exists(via: string, where?: Condition): Condition {
    return where === undefined ? { kind: "exists", via } : { kind: "exists", via, where };
}

/** Name a column operand. */
function column(name: string): Operand {
    return { kind: "column", name };
}

/** Embed a literal operand. */
function literal(value: Scalar): Operand {
    return { kind: "literal", value };
}

/** Name a parameter operand. */
function parameter(name: string): Operand {
    return { kind: "parameter", name };
}

/** List the columns a condition reads, by property. */
function columns(condition: Condition, names = new Set<string>()): Set<string> {
    // collect the column operands of a comparison or test
    if (condition.kind === "compare" || condition.kind === "in" || condition.kind === "null") {
        const operands =
            condition.kind === "compare" ? [condition.left, condition.right] : [condition.operand];
        for (const operand of operands) {
            if (operand.kind === "column") {
                names.add(operand.name);
            }
        }
    }
    // collect through combinations, leaving relations to their host
    else if (condition.kind !== "exists") {
        const nested = condition.kind === "not" ? [condition.condition] : condition.conditions;
        for (const entry of nested) {
            columns(entry, names);
        }
    }

    return names;
}

/** List the relations a condition follows, each with the condition its related rows meet. */
function relations(
    condition: Condition,
    found: { readonly via: string; readonly where: Condition | undefined }[] = [],
): { readonly via: string; readonly where: Condition | undefined }[] {
    // collect a relation, and the relations of combinations
    if (condition.kind === "exists") {
        found.push({ via: condition.via, where: condition.where });
    } else if (condition.kind === "all" || condition.kind === "any") {
        for (const entry of condition.conditions) {
            relations(entry, found);
        }
    } else if (condition.kind === "not") {
        relations(condition.condition, found);
    }

    return found;
}

/** Require a condition of bounded size over a table's comparable columns and computed values. */
function requireComparable(
    condition: Condition,
    table: Table,
    namespace: Namespace = { computed: {} },
): void {
    // bound the condition's size, which clients choose
    const size = terms(condition);
    if (size > CONDITION_TERMS) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `condition holds ${size} terms, more than ${CONDITION_TERMS}`,
        );
    }

    // require comparable columns, or computed values
    for (const name of columns(condition)) {
        if (!Object.hasOwn(namespace.computed, name)) {
            Order.column(table, name);
        }
    }
}

/** Count a condition's terms: each comparison, missing check and group, and each listed value. */
function terms(condition: Condition): number {
    switch (condition.kind) {
        case "compare":
        case "null":
            return 1;
        case "in":
            return Math.max(1, condition.values.length);
        case "all":
        case "any":
            return condition.conditions.reduce((total, entry) => total + terms(entry), 1);
        case "not":
            return 1 + terms(condition.condition);
        case "exists":
            return 1 + (condition.where === undefined ? 0 : terms(condition.where));
    }
}

/** Bind a condition's columns to a table and its namespace, and its parameters to values. */
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

/** Render a condition as SQL, binding literals through the columns they meet. */
function renderCondition(condition: Condition, binding: Binding<SQLWrapper>): SQL {
    switch (condition.kind) {
        case "compare": {
            const isOrdered = condition.operator !== "eq" && condition.operator !== "ne";
            const left = render(condition.left, condition.right, binding, isOrdered);
            const right = render(condition.right, condition.left, binding, isOrdered);

            return sql`(${left} ${sql.raw(OPERATORS[condition.operator])} ${right})`;
        }
        case "in": {
            // match a flat OR chain, which SQLite plans faster than an IN list, and an IN list beyond a chain's limit
            const operand = render(condition.operand, undefined, binding, false);
            const values = condition.values.map((value) =>
                render(literal(value), condition.operand, binding, false),
            );

            return inArray(operand, values);
        }
        case "null":
            return sql`(${render(condition.operand, undefined, binding, false)} IS NULL)`;
        case "all":
        case "any": {
            // render an empty conjunction true and an empty disjunction false
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

/** Compile a condition over a table, decoding its literals as their columns hold them once. */
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
            // match nothing in an empty list, and stay unknown for a missing operand
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
            // settle on the deciding value, and stay unknown when any condition is
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
            // ask the binding, which answers true, false or unknown until the relation is read
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

/** Render one operand, binding a literal through the column the other side names. */
function render(
    operand: Operand,
    other: Operand | undefined,
    binding: Binding<SQLWrapper>,
    isOrdered: boolean,
): SQLWrapper {
    // read a column, ordering text by byte, and comparing it for equality through its index
    if (operand.kind === "column") {
        const resolved = binding.column(operand.name);

        return isOrdered && resolved instanceof Column ? Order.text(resolved) : resolved;
    }

    // bind a parameter or literal as the other side's column encodes it
    const scalar = operand.kind === "parameter" ? binding.parameter(operand.name) : operand.value;
    const target = other?.kind === "column" ? binding.column(other.name) : undefined;

    return target instanceof Column && scalar !== null
        ? sql`${new Param(target.definition.fromJson(scalar), target)}`
        : sql`${scalar}`;
}

/** Compile an operand's value: a column's, or a parameter or literal decoded as the other side's column holds it. */
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

    // decode a parameter each time, as the other side's column holds it
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

/** Decode a scalar as a table's column holds it, and as it is for a computed value. */
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
