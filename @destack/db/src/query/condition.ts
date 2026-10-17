import { schema } from "@destack/schema";
import type { JsonOf } from "../table/column.ts";
import { DatabaseError } from "../error/error.ts";
import { SearchText } from "../search/text.ts";
import { Predicate } from "./predicate.ts";
import { Namespace } from "./namespace.ts";
import { Placeholder, type SQL } from "../sql/index.ts";
import type { Table } from "../table/table.ts";

/** The most terms one condition has. */
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
export const Scalar = schema.union([
    schema.string(),
    schema.number(),
    schema.boolean(),
    schema.null(),
]);
/** A scalar a condition compares, in its JSON form. */
export type Scalar = schema.Infer<typeof Scalar>;

/** A scalar a field compares with: any scalar but null. */
export const Comparable = schema.union([schema.string(), schema.number(), schema.boolean()]);
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

/** Another field of the row a comparison reads. */
export const FieldReference = schema.object({
    /** The field's name. */
    field: schema.string().min(1),
});
/** Another field of the row a comparison reads. */
export type FieldReference = schema.Infer<typeof FieldReference>;

/** A placeholder read when the condition runs, or another field of the row. */
const Operand = schema.union([PlaceholderValue, FieldReference]);

/** A field's comparand: a value, a placeholder read when the condition runs, or another field. */
const Comparand = schema.union([Comparable, PlaceholderValue, FieldReference]);

/** The comparisons of one field. */
export type FieldCondition<Value extends Comparable = Comparable> = {
    /** Match an equal value. */
    readonly eq?: Value | Placeholder | FieldReference;
    /** Match a different value. */
    readonly ne?: Value | Placeholder | FieldReference;
    /** Match a lower value. */
    readonly lt?: Value | Placeholder | FieldReference;
    /** Match a value at most this one. */
    readonly lte?: Value | Placeholder | FieldReference;
    /** Match a higher value. */
    readonly gt?: Value | Placeholder | FieldReference;
    /** Match a value at least this one. */
    readonly gte?: Value | Placeholder | FieldReference;
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
    /** Match text starting with a prefix. */
    readonly startsWith?: Value & string;
    /** Match text ending with a suffix. */
    readonly endsWith?: Value & string;
    /** Match text of only these characters: letters, digits, ranges of them, and `._+/=:@*-`. */
    readonly characters?: string;
    /** Match text whose length in characters is this one or meets a comparison. */
    readonly length?: number | LengthCondition;
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

/** The comparisons of a text's length in characters. */
export type LengthCondition = {
    /** Match an equal length. */
    readonly eq?: number;
    /** Match a different length. */
    readonly ne?: number;
    /** Match a shorter length. */
    readonly lt?: number;
    /** Match a length at most this one. */
    readonly lte?: number;
    /** Match a longer length. */
    readonly gt?: number;
    /** Match a length at least this one. */
    readonly gte?: number;
    /** Match one of some lengths. */
    readonly in?: readonly number[];
    /** Match none of some lengths. */
    readonly notIn?: readonly number[];
    /** Require every comparison. */
    readonly AND?: readonly LengthCondition[];
    /** Require any comparison. */
    readonly OR?: readonly LengthCondition[];
    /** Negate a comparison. */
    readonly NOT?: LengthCondition;
};

/** The comparisons of a text's length, as resolving reads them. */
const LengthCondition: schema.Schema<LengthCondition> = schema.lazy(() =>
    schema.object({
        eq: schema.int().nonnegative().exactOptional(),
        ne: schema.int().nonnegative().exactOptional(),
        lt: schema.int().nonnegative().exactOptional(),
        lte: schema.int().nonnegative().exactOptional(),
        gt: schema.int().nonnegative().exactOptional(),
        gte: schema.int().nonnegative().exactOptional(),
        in: schema.array(schema.int().nonnegative()).exactOptional(),
        notIn: schema.array(schema.int().nonnegative()).exactOptional(),
        AND: schema.array(LengthCondition).exactOptional(),
        OR: schema.array(LengthCondition).exactOptional(),
        NOT: LengthCondition.exactOptional(),
    }),
);

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
        startsWith: schema.string().exactOptional(),
        endsWith: schema.string().exactOptional(),
        characters: schema.string().min(1).exactOptional(),
        length: schema.union([schema.int().nonnegative(), LengthCondition]).exactOptional(),
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
    /** Require the row's searched text to contain each word, as a word's prefix. */
    readonly SEARCH?: string;
    /** Require every condition. */
    readonly AND?: readonly KnownCondition<Row, Relations>[];
    /** Require any condition. */
    readonly OR?: readonly KnownCondition<Row, Relations>[];
    /** Negate a condition. */
    readonly NOT?: KnownCondition<Row, Relations>;
};

/** An entry of a condition in its JSON form. */
type JsonEntry = Comparable | readonly Comparable[] | JsonCondition | readonly JsonCondition[];

/** A condition in JSON form, as declarations, rows and the wire keep it with `{ placeholder }`. */
export type JsonCondition = { readonly [name: string]: JsonEntry };

/** An entry of a condition whose row is unknown. */
type OpenEntry =
    | Comparable
    | Placeholder
    | readonly Comparable[]
    | OpenCondition
    | readonly OpenCondition[];

/** A condition on a row of unknown fields, its placeholders in memory or in JSON form. */
export type OpenCondition = { readonly [name: string]: OpenEntry };

/** A predicate over a row. */
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
    any,
    resolve,
    render,
    is,
    conjuncts,
};

/** Report whether a condition's entry is itself a condition, as in combinations and negations. */
function is(entry: unknown): entry is JsonCondition {
    return typeof entry === "object" && entry !== null && !Array.isArray(entry);
}

/** List the conditions a condition requires at its top: itself and its conjunction's members. */
function conjuncts(where: JsonCondition | undefined): readonly JsonCondition[] {
    const conjunction = where?.["AND"];

    return [where ?? {}, ...(Array.isArray(conjunction) ? conjunction.filter(is) : [])];
}

/** Match rows whose fields equal some values, a null value matching a missing one. */
function equal(values: Readonly<Record<string, Scalar>>): OpenCondition {
    return Object.fromEntries(
        Object.entries(values).map(([name, value]) => [
            name,
            value === null ? { isNull: true } : value,
        ]),
    );
}

/** Resolve conditions the engine builds, such as reads of many keys, unbounded, matching any. */
function any(conditions: readonly Condition[], table: Table): Predicate {
    const fields = Namespace.fields(table);

    return {
        kind: "any",
        predicates: conditions.map((condition) => resolveCondition(condition, fields)),
    };
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
    Predicate.require(predicate, table, namespace);

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
    // search the row's text for every word of a query
    else if (name === "SEARCH") {
        if (typeof entry !== "string") {
            throw new DatabaseError("INVALID_QUERY", "a search needs text");
        }

        return { kind: "search", words: SearchText.query(entry) };
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

    // compare with a placeholder or another field, or apply the field's comparisons
    const operand = Operand.safeParse(entry);
    if (operand.success) {
        return { kind: "compare", operator: "eq", name, value: operand.data };
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
    // refuse a value for a name that is not a field
    else if (typeof entry !== "object" || isList(entry) || entry instanceof Placeholder) {
        throw new DatabaseError("INVALID_QUERY", `condition has no field ${name}`);
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
        ...textPredicates(name, field),
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

/** Resolve a field's matches of text by its ends, its characters and its length. */
function textPredicates(name: string, field: FieldCondition): Predicate[] {
    // match a prefix and a suffix as literal patterns
    const predicates: Predicate[] = [];
    if (field.startsWith !== undefined) {
        const pattern = `${Predicate.literal(field.startsWith)}%`;
        predicates.push({ kind: "like", name, pattern, isInsensitive: false });
    }
    if (field.endsWith !== undefined) {
        const pattern = `%${Predicate.literal(field.endsWith)}`;
        predicates.push({ kind: "like", name, pattern, isInsensitive: false });
    }

    // match the characters and the length
    if (field.characters !== undefined) {
        predicates.push({ kind: "characters", name, set: Predicate.characters(field.characters) });
    }
    if (field.length !== undefined) {
        const length = typeof field.length === "number" ? { eq: field.length } : field.length;
        predicates.push({ kind: "length", name, predicate: fieldPredicate(name, length) });
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
        const alternatives = field.OR.map((nested) => fieldPredicate(name, nested));
        predicates.push({ kind: "any", predicates: alternatives });
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
            `condition field ${name} needs a value or comparisons`,
        );
    }

    return parsed.data;
}

/** Read an entry as one condition. */
function conditionOf(name: string, entry: OpenEntry): OpenCondition {
    if (typeof entry !== "object" || isList(entry) || entry instanceof Placeholder) {
        throw new DatabaseError("INVALID_QUERY", `condition ${name} needs a condition`);
    }

    return entry;
}

/** Read an entry as a list of conditions. */
function conditionsOf(name: string, entry: OpenEntry): readonly OpenCondition[] {
    if (typeof entry !== "object" || !isList(entry)) {
        throw new DatabaseError("INVALID_QUERY", `condition ${name} needs a list of conditions`);
    }

    // require each element to be a condition
    const conditions: OpenCondition[] = [];
    for (const element of entry) {
        if (typeof element !== "object" || element instanceof Placeholder) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `condition ${name} lists a value, not a condition`,
            );
        }
        conditions.push(element);
    }

    return conditions;
}

/** Report whether an entry is a list. */
function isList(entry: OpenEntry): entry is readonly Comparable[] | readonly OpenCondition[] {
    return Array.isArray(entry);
}
