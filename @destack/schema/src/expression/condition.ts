import * as schema from "../validate/index.ts";
import { defineSchema } from "../inspect/schema.ts";

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
            /** Match rows with a related row meeting a condition. */
            kind: schema.literal("exists"),
            /** The relation name. */
            via: schema.string().min(1),
            /** The related row's condition. */
            where: conditionSchema.optional(),
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

/** Match rows with a related row meeting a condition. */
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

/** Rename a condition's column operands. */
function rename(condition: Condition, name: (column: string) => string): Condition {
    const operand = (value: Operand): Operand =>
        value.kind === "column" ? { kind: "column", name: name(value.name) } : value;

    // rename the operands
    if (condition.kind === "compare") {
        return { ...condition, left: operand(condition.left), right: operand(condition.right) };
    } else if (condition.kind === "in" || condition.kind === "null") {
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
    if (condition.kind === "compare" || condition.kind === "in" || condition.kind === "null") {
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
