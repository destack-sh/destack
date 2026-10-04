import { defineSchema, schema } from "@destack/schema";
import type { JsonOf } from "../table/column.ts";
import { DatabaseError } from "../error/error.ts";
import { Predicate } from "./predicate.ts";
import { Namespace } from "./namespace.ts";
import { Placeholder, type SQL } from "../sql/index.ts";
import type { Table } from "../table/table.ts";

/**
 * The most terms one condition has.
 *
 * At about 40 bytes a term, a condition renders at most about 40 KB of SQL.
 */
const CONDITION_TERMS = 1000;

/** The comparison operators, in the order a field's comparisons resolve. */
const COMPARISONS = ["eq", "ne", "lt", "lte", "gt", "gte"] as const;

/** The pattern operators: whether each negates its match and ignores ASCII case. */
const PATTERNS = [
    { operator: "like", isNegated: false, isInsensitive: false },
    { operator: "ilike", isNegated: false, isInsensitive: true },
    { operator: "notLike", isNegated: true, isInsensitive: false },
    { operator: "notIlike", isNegated: true, isInsensitive: true },
] as const;

/** A scalar a condition compares, in its JSON form. */
export const Scalar = defineSchema(
    schema.union([schema.string(), schema.number(), schema.boolean(), schema.null()]),
);
/** A scalar a condition compares, in its JSON form. */
export type Scalar = schema.Infer<typeof Scalar>;

/** A scalar a field compares with: any scalar but null. */
export const Comparable = defineSchema(
    schema.union([schema.string(), schema.number(), schema.boolean()]),
);
/** A scalar a field compares with: any scalar but null. */
export type Comparable = schema.Infer<typeof Comparable>;

/** A comparison operator. */
export type Operator = (typeof COMPARISONS)[number];

/** A placeholder in its JSON form, as declarations and the wire keep it. */
const PlaceholderJson = schema.object({
    /** The placeholder's name. */
    placeholder: schema.string().min(1),
});

/** A placeholder as a condition reads it, in memory or in its JSON form. */
const PlaceholderValue: schema.Schema<Placeholder> = schema.union([
    schema.instanceof(Placeholder),
    PlaceholderJson.transform((value) => new Placeholder(value.placeholder)),
]);

/** A value a field compares with, or a placeholder read when the condition runs. */
const Comparand = schema.union([Comparable, PlaceholderValue]);

/** The comparisons of one field. */
export type FieldCondition<Value extends Comparable = Comparable> = {
    /** Match an equal value. */
    readonly eq?: Value | Placeholder;
    /** Match a different value. */
    readonly ne?: Value | Placeholder;
    /** Match a lower value. */
    readonly lt?: Value | Placeholder;
    /** Match a value at most this one. */
    readonly lte?: Value | Placeholder;
    /** Match a higher value. */
    readonly gt?: Value | Placeholder;
    /** Match a value at least this one. */
    readonly gte?: Value | Placeholder;
    /** Match one of some values. */
    readonly in?: readonly Value[];
    /** Match none of some values. */
    readonly notIn?: readonly Value[];
    /** Match text by a pattern, `%` any run and `_` any one character. */
    readonly like?: Value & string;
    /** Match text by a pattern, ignoring ASCII case. */
    readonly ilike?: Value & string;
    /** Match text a pattern does not. */
    readonly notLike?: Value & string;
    /** Match text a pattern does not, ignoring ASCII case. */
    readonly notIlike?: Value & string;
    /** Match a missing value. */
    readonly isNull?: true;
    /** Match a present value. */
    readonly isNotNull?: true;
    /** Require every comparison. */
    readonly AND?: readonly FieldCondition<Value>[];
    /** Require any comparison. */
    readonly OR?: readonly FieldCondition<Value>[];
    /** Negate a comparison. */
    readonly NOT?: FieldCondition<Value>;
};

/** The comparisons of one field, as resolving reads them. */
const FieldCondition: schema.Schema<FieldCondition> = schema.lazy(() =>
    schema.object({
        eq: Comparand.exactOptional(),
        ne: Comparand.exactOptional(),
        lt: Comparand.exactOptional(),
        lte: Comparand.exactOptional(),
        gt: Comparand.exactOptional(),
        gte: Comparand.exactOptional(),
        in: schema.array(Comparable).exactOptional(),
        notIn: schema.array(Comparable).exactOptional(),
        like: schema.string().exactOptional(),
        ilike: schema.string().exactOptional(),
        notLike: schema.string().exactOptional(),
        notIlike: schema.string().exactOptional(),
        isNull: schema.literal(true).exactOptional(),
        isNotNull: schema.literal(true).exactOptional(),
        AND: schema.array(FieldCondition).exactOptional(),
        OR: schema.array(FieldCondition).exactOptional(),
        NOT: FieldCondition.exactOptional(),
    }),
);

/** The value a field compares with, never for a field no condition compares. */
type ComparableOf<Value> =
    Exclude<JsonOf<Value>, null> extends Comparable ? Exclude<JsonOf<Value>, null> : never;

/** A condition on a known row's fields and relations. */
type KnownCondition<Row, Relations> = {
    readonly [Name in keyof Row as [ComparableOf<Row[Name]>] extends [never] ? never : Name]?:
        | ComparableOf<Row[Name]>
        | Placeholder
        | FieldCondition<ComparableOf<Row[Name]>>;
} & { readonly [Name in keyof Relations]?: true | Relations[Name] } & {
    /** Require every condition. */
    readonly AND?: readonly KnownCondition<Row, Relations>[];
    /** Require any condition. */
    readonly OR?: readonly KnownCondition<Row, Relations>[];
    /** Negate a condition. */
    readonly NOT?: KnownCondition<Row, Relations>;
};

/** An entry of a condition in its JSON form. */
type JsonEntry = Comparable | readonly Comparable[] | JsonCondition | readonly JsonCondition[];

/** A condition in its JSON form, as declarations, rows and the wire keep it: placeholders as `{ placeholder }`. */
export type JsonCondition = { readonly [name: string]: JsonEntry };

/** An entry of a condition whose row is unknown. */
type OpenEntry =
    | Comparable
    | Placeholder
    | readonly Comparable[]
    | OpenCondition
    | readonly OpenCondition[];

/** A condition on a row whose fields are unknown, its placeholders in memory or in their JSON form. */
export type OpenCondition = { readonly [name: string]: OpenEntry };

/**
 * A predicate over a row.
 *
 * A field takes a value or its comparisons, a relation its related row's condition or `true`, and `AND`, `OR` and `NOT` combine conditions.
 * Relations map each relation name to the condition type of its related rows.
 */
export type Condition<
    Row = Readonly<Record<string, Scalar>>,
    Relations = Readonly<Record<never, never>>,
> = string extends keyof Row ? OpenCondition : KnownCondition<Row, Relations>;

/** The schema of a condition as plain JSON, its placeholders in their JSON form. */
const conditionSchema: schema.Schema<JsonCondition> = schema.lazy(() =>
    schema.record(
        schema.string(),
        schema.union([
            Comparable,
            schema.array(Comparable),
            conditionSchema,
            schema.array(conditionSchema),
        ]),
    ),
);

/** A predicate over a row as plain JSON. */
export const Condition = {
    /** The schema of a condition. */
    schema: conditionSchema,
    equal,
    resolve,
    render,
};

/** Match rows whose fields equal some values, a null value matching a missing one. */
function equal(values: Readonly<Record<string, Scalar>>): OpenCondition {
    return Object.fromEntries(
        Object.entries(values).map(([name, value]) => [
            name,
            value === null ? { isNull: true } : value,
        ]),
    );
}

/** Resolve a bounded condition against its row's fields, any other name being a relation. */
function resolve(condition: Condition, fields: ReadonlySet<string>): Predicate {
    // bound the size, each relation's condition being bounded when resolved against its row
    const predicate = resolveCondition(condition, fields);
    const terms = Predicate.size(predicate);
    if (terms > CONDITION_TERMS) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `condition has ${terms} terms, more than ${CONDITION_TERMS}`,
        );
    }

    return predicate;
}

/** Render a condition over a table's rows as SQL. */
function render(condition: Condition, table: Table, namespace?: Namespace): SQL {
    const predicate = resolve(condition, Namespace.fields(table, namespace));

    return Predicate.render(predicate, Predicate.bind(table, {}, namespace));
}

/** Resolve a condition's entries, which all hold. */
function resolveCondition(condition: Condition, fields: ReadonlySet<string>): Predicate {
    return Predicate.all(
        ...Object.entries(condition).map(([name, entry]) => resolveEntry(name, entry, fields)),
    );
}

/** Resolve one entry of a condition. */
function resolveEntry(name: string, entry: OpenEntry, fields: ReadonlySet<string>): Predicate {
    // combine conditions
    if (name === "AND" || name === "OR") {
        return {
            kind: name === "AND" ? "all" : "any",
            predicates: conditionsOf(name, entry).map((nested) => resolveCondition(nested, fields)),
        };
    }
    // negate a condition
    else if (name === "NOT") {
        return { kind: "not", predicate: resolveCondition(conditionOf(name, entry), fields) };
    }
    // compare a field
    else if (fields.has(name)) {
        return resolveField(name, entry);
    }
    // follow a relation
    else {
        return resolveRelation(name, entry);
    }
}

/** Resolve a field's entry: a value, a placeholder, or the field's comparisons. */
function resolveField(name: string, entry: OpenEntry): Predicate {
    // compare with a value
    if (typeof entry !== "object") {
        return { kind: "compare", operator: "eq", name, value: entry };
    }

    // compare with a placeholder, or apply the field's comparisons
    const placeholder = PlaceholderValue.safeParse(entry);
    if (placeholder.success) {
        return { kind: "compare", operator: "eq", name, value: placeholder.data };
    } else {
        return fieldPredicate(name, comparisonsOf(name, entry));
    }
}

/** Resolve a relation's entry: `true` for any related row, or the related rows' condition. */
function resolveRelation(name: string, entry: OpenEntry): Predicate {
    // follow a relation to any related row
    if (entry === true) {
        return { kind: "exists", via: name, where: {} };
    }
    // refuse a value for a name that is no field
    else if (typeof entry !== "object" || isList(entry) || entry instanceof Placeholder) {
        throw new DatabaseError("INVALID_QUERY", `condition names no field ${name}`);
    }
    // follow a relation to the related rows meeting a condition
    else {
        return { kind: "exists", via: name, where: conditionOf(name, entry) };
    }
}

/** Resolve a field's comparisons, which all hold. */
function fieldPredicate(name: string, field: FieldCondition): Predicate {
    return Predicate.all(
        ...comparisonPredicates(name, field),
        ...valuePredicates(name, field),
        ...patternPredicates(name, field),
        ...nestedPredicates(name, field),
    );
}

/** Resolve a field's comparisons with single values. */
function comparisonPredicates(name: string, field: FieldCondition): Predicate[] {
    // compare with each value
    const predicates: Predicate[] = [];
    for (const operator of COMPARISONS) {
        const value = field[operator];
        if (value !== undefined) {
            predicates.push({ kind: "compare", operator, name, value });
        }
    }

    return predicates;
}

/** Resolve a field's matches of listed and missing values. */
function valuePredicates(name: string, field: FieldCondition): Predicate[] {
    // match listed values
    const predicates: Predicate[] = [];
    if (field.in !== undefined) {
        predicates.push({ kind: "oneOf", name, values: field.in });
    }
    if (field.notIn !== undefined) {
        predicates.push({ kind: "not", predicate: { kind: "oneOf", name, values: field.notIn } });
    }

    // match missing and present values
    if (field.isNull === true) {
        predicates.push({ kind: "missing", name });
    }
    if (field.isNotNull === true) {
        predicates.push({ kind: "not", predicate: { kind: "missing", name } });
    }

    return predicates;
}

/** Resolve a field's matches of text by patterns. */
function patternPredicates(name: string, field: FieldCondition): Predicate[] {
    // match each pattern, negated or ignoring ASCII case as its operator says
    const predicates: Predicate[] = [];
    for (const { operator, isNegated, isInsensitive } of PATTERNS) {
        const pattern = field[operator];
        if (pattern !== undefined) {
            const like: Predicate = { kind: "like", name, pattern, isInsensitive };
            predicates.push(isNegated ? { kind: "not", predicate: like } : like);
        }
    }

    return predicates;
}

/** Resolve a field's nested comparisons combined by `AND`, `OR` and `NOT`. */
function nestedPredicates(name: string, field: FieldCondition): Predicate[] {
    // require every, any or no nested comparison
    const predicates: Predicate[] = [];
    if (field.AND !== undefined) {
        predicates.push(Predicate.all(...field.AND.map((nested) => fieldPredicate(name, nested))));
    }
    if (field.OR !== undefined) {
        const any = field.OR.map((nested) => fieldPredicate(name, nested));
        predicates.push({ kind: "any", predicates: any });
    }
    if (field.NOT !== undefined) {
        predicates.push({ kind: "not", predicate: fieldPredicate(name, field.NOT) });
    }

    return predicates;
}

/** Read an entry as a field's comparisons. */
function comparisonsOf(name: string, entry: OpenEntry): FieldCondition {
    const parsed = FieldCondition.safeParse(entry);
    if (!parsed.success) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `condition field ${name} takes a value or comparisons`,
        );
    }

    return parsed.data;
}

/** Read an entry as one condition. */
function conditionOf(name: string, entry: OpenEntry): OpenCondition {
    if (typeof entry !== "object" || isList(entry) || entry instanceof Placeholder) {
        throw new DatabaseError("INVALID_QUERY", `condition ${name} takes a condition`);
    }

    return entry;
}

/** Read an entry as a list of conditions. */
function conditionsOf(name: string, entry: OpenEntry): readonly OpenCondition[] {
    if (typeof entry !== "object" || !isList(entry)) {
        throw new DatabaseError("INVALID_QUERY", `condition ${name} takes a list of conditions`);
    }

    // require each element to be a condition
    const conditions: OpenCondition[] = [];
    for (const element of entry) {
        if (typeof element !== "object" || element instanceof Placeholder) {
            throw new DatabaseError("INVALID_QUERY", `condition ${name} lists a value`);
        }
        conditions.push(element);
    }

    return conditions;
}

/** Report whether an entry is a list. */
function isList(entry: OpenEntry): entry is readonly Comparable[] | readonly OpenCondition[] {
    return Array.isArray(entry);
}
