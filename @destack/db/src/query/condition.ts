import { Parameter, combine, inArray, sql, type SQL, type SQLWrapper } from "../sql/index.ts";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Order } from "./order.ts";
import type { Namespace } from "./namespace.ts";
import { DatabaseError } from "../error/error.ts";
import { defineSchema, schema } from "@destack/schema";

/**
 * The most terms one condition has.
 *
 * At about 40 bytes a term, a condition renders at most about 40 KB of SQL.
 */
const CONDITION_TERMS = 1000;

/** The SQL each comparison operator renders. */
const OPERATORS = { eq: "=", ne: "<>", lt: "<", lte: "<=", gt: ">", gte: ">=" } as const;

/** How a condition's names resolve: columns to values, and related rows to decisions. */
export interface Binding<Value, Decision = Value> {
    /** Resolve a column. */
    column(name: string): Value;
    /** Resolve a parameter's value. */
    parameter(name: string): Scalar;
    /** Resolve whether a related row meets a condition. */
    exists(via: string, where: Condition | undefined): Decision;
}

/** A scalar a condition compares, in its JSON form. */
export const Scalar = defineSchema(
    schema.union([schema.string(), schema.number(), schema.boolean(), schema.null()]),
);
/** A scalar a condition compares, in its JSON form. */
export type Scalar = schema.Infer<typeof Scalar>;

/** A comparison operator. */
export const Operator = defineSchema(schema.enum(["eq", "ne", "lt", "lte", "gt", "gte"]));
/** A comparison operator. */
export type Operator = schema.Infer<typeof Operator>;

/** One side of a comparison: a column, a literal or a parameter. */
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
            /** The value, in its column's JSON form. */
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
/** One side of a comparison: a column, a literal or a parameter. */
export type Operand = schema.Infer<typeof Operand>;

/** A predicate over a row, decided alike in SQL and memory. */
export type Condition =
    | {
          readonly kind: "compare";
          readonly operator: Operator;
          readonly left: Operand;
          readonly right: Operand;
      }
    | {
          readonly kind: "oneOf";
          readonly operand: Operand;
          readonly values: readonly Exclude<Scalar, null>[];
      }
    | { readonly kind: "missing"; readonly operand: Operand }
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
            kind: schema.literal("oneOf"),
            /** The operand. */
            operand: Operand,
            /** The values, none of them missing. */
            values: schema.array(
                schema.union([schema.string(), schema.number(), schema.boolean()]),
            ),
        }),
        schema.object({
            /** Match a missing value. */
            kind: schema.literal("missing"),
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
            /** Match rows with a related row meeting a condition. */
            kind: schema.literal("exists"),
            /** The relation name. */
            via: schema.string().min(1),
            /** The related row's condition. */
            where: conditionSchema.exactOptional(),
        }),
    ]),
);

/** A predicate over a row, decided alike in SQL and memory. */
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
    rename,
    relations,
    terms,
    require: requireComparable,
    bind,
    render: renderCondition,
    compile,
    matches,
};

/** Compare a column with an operand. */
function compare(operator: Operator, left: Operand | string, right: Operand | Scalar): Condition {
    return {
        kind: "compare",
        operator,
        left: typeof left === "string" ? column(left) : left,
        right: typeof right === "object" && right !== null ? right : literal(right),
    };
}

/** Match rows whose column has one of some values. */
function oneOf(operand: Operand | string, values: readonly Exclude<Scalar, null>[]): Condition {
    return {
        kind: "oneOf",
        operand: typeof operand === "string" ? column(operand) : operand,
        values,
    };
}

/** Match rows whose column has no value. */
function missing(operand: Operand | string): Condition {
    return {
        kind: "missing",
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

/** Match rows with a related row meeting a condition. */
function exists(via: string, where?: Condition): Condition {
    return where === undefined ? { kind: "exists", via } : { kind: "exists", via, where };
}

/** Build a column operand. */
function column(name: string): Operand {
    return { kind: "column", name };
}

/** Embed a literal operand. */
function literal(value: Scalar): Operand {
    return { kind: "literal", value };
}

/** Build a parameter operand. */
function parameter(name: string): Operand {
    return { kind: "parameter", name };
}

/** Rename a condition's column operands. */
function rename(condition: Condition, name: (column: string) => string): Condition {
    const operand = (value: Operand): Operand =>
        value.kind === "column" ? { kind: "column", name: name(value.name) } : value;

    // rename the operands
    if (condition.kind === "compare") {
        return { ...condition, left: operand(condition.left), right: operand(condition.right) };
    } else if (condition.kind === "oneOf" || condition.kind === "missing") {
        return { ...condition, operand: operand(condition.operand) };
    }
    // rename through combinations
    else if (condition.kind === "not") {
        return { ...condition, condition: rename(condition.condition, name) };
    } else if (condition.kind === "all" || condition.kind === "any") {
        return {
            ...condition,
            conditions: condition.conditions.map((entry) => rename(entry, name)),
        };
    }
    // leave relations alone
    else {
        return condition;
    }
}

/** List the columns a condition reads, by property. */
function columns(condition: Condition, names = new Set<string>()): Set<string> {
    // collect the column operands
    if (
        condition.kind === "compare" ||
        condition.kind === "oneOf" ||
        condition.kind === "missing"
    ) {
        const operands =
            condition.kind === "compare" ? [condition.left, condition.right] : [condition.operand];
        for (const operand of operands) {
            if (operand.kind === "column") {
                names.add(operand.name);
            }
        }
    }
    // collect through combinations
    else if (condition.kind !== "exists") {
        const nested = condition.kind === "not" ? [condition.condition] : condition.conditions;
        for (const entry of nested) {
            columns(entry, names);
        }
    }

    return names;
}

/** List the relations a condition follows, with their conditions. */
function relations(
    condition: Condition,
    found: { readonly via: string; readonly where: Condition | undefined }[] = [],
): { readonly via: string; readonly where: Condition | undefined }[] {
    // collect relations through combinations
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

/** Count a condition's terms. */
function terms(condition: Condition): number {
    switch (condition.kind) {
        case "compare":
        case "missing":
            return 1;
        case "oneOf":
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

/** Decide a compiled condition on a row with relations unknown. */
function matches(match: Match, row: Readonly<Record<string, unknown>>): boolean {
    return (
        match({ column: (name) => row[name], parameter: () => null, exists: () => undefined }) ===
        true
    );
}

/** A compiled condition, deciding a row in three-valued logic. */
export type Match = (binding: Binding<unknown, boolean | undefined>) => boolean | undefined;

/** Require a bounded condition over comparable columns and computed values. */
function requireComparable(
    condition: Condition,
    table: Table,
    namespace: Namespace = { computed: {} },
): void {
    // bound the size
    const size = Condition.terms(condition);
    if (size > CONDITION_TERMS) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `condition has ${size} terms, more than ${CONDITION_TERMS}`,
        );
    }

    // require comparable columns or computed values
    for (const name of Condition.columns(condition)) {
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
            // require a bound value
            const value = parameters[name];
            if (value === undefined) {
                throw new DatabaseError("INVALID_QUERY", `condition parameter is unbound: ${name}`);
            }

            return value;
        },
    };
}

/** Render a condition as SQL. */
function renderCondition(condition: Condition, binding: Binding<SQLWrapper>): SQL {
    switch (condition.kind) {
        case "compare": {
            const isOrdered = condition.operator !== "eq" && condition.operator !== "ne";
            const left = renderOperand(condition.left, condition.right, binding, isOrdered);
            const right = renderOperand(condition.right, condition.left, binding, isOrdered);

            return sql`(${left} ${sql.raw(OPERATORS[condition.operator])} ${right})`;
        }
        case "oneOf": {
            // match the operand in the listed values
            const operand = renderOperand(condition.operand, undefined, binding, false);
            const values = condition.values.map((value) =>
                renderOperand(Condition.literal(value), condition.operand, binding, false),
            );

            return inArray(operand, values);
        }
        case "missing":
            return sql`(${renderOperand(condition.operand, undefined, binding, false)} IS NULL)`;
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
            const left = compileOperand(condition.left, condition.right, table);
            const right = compileOperand(condition.right, condition.left, table);
            const operator = condition.operator;

            return (binding) => {
                const order = Order.values(left(binding), right(binding));

                return order === undefined ? undefined : satisfies(operator, order);
            };
        }
        case "oneOf": {
            // match nothing for an empty list and unknown for a missing operand
            const operand = compileOperand(condition.operand, undefined, table);
            const name = condition.operand.kind === "column" ? condition.operand.name : undefined;
            const values =
                name === undefined
                    ? [...condition.values]
                    : condition.values.map((entry) => decode(table, name, entry));

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
        case "missing": {
            const operand = compileOperand(condition.operand, undefined, table);

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

            return (binding) => binding.exists(via, where);
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
function renderOperand(
    operand: Operand,
    other: Operand | undefined,
    binding: Binding<SQLWrapper>,
    isOrdered: boolean,
): SQLWrapper {
    // read a column, ordering text by byte
    if (operand.kind === "column") {
        const resolved = binding.column(operand.name);

        return isOrdered && Column.is(resolved) ? Order.text(resolved) : resolved;
    }

    // bind a value through the other side's column
    const scalar = operand.kind === "parameter" ? binding.parameter(operand.name) : operand.value;
    const target = other?.kind === "column" ? binding.column(other.name) : undefined;

    return target instanceof Column && scalar !== null
        ? sql`${new Parameter(target.definition.fromJson(scalar), target)}`
        : sql`${scalar}`;
}

/** Compile an operand's value. */
function compileOperand(
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
    const property = other?.kind === "column" ? other.name : undefined;
    if (operand.kind === "parameter") {
        const name = operand.name;

        return (binding) => {
            const scalar = binding.parameter(name);

            return property === undefined || scalar === null
                ? scalar
                : decode(table, property, scalar);
        };
    }

    // decode a literal once
    const value =
        property === undefined || operand.value === null
            ? operand.value
            : decode(table, property, operand.value);

    return () => value;
}

/** Decode a scalar as a column stores it, or keep it for a computed value. */
function decode(table: Table, property: string, scalar: Scalar): unknown {
    // compare a computed value, which no column stores, as its JSON scalar
    const stored = table[TABLE].columns[property];

    return stored === undefined ? scalar : stored.definition.fromJson(scalar);
}

/** Decide whether an order satisfies an operator. */
function satisfies(operator: Operator, order: number): boolean {
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
