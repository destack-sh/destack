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

/** Another field of the row a comparison reads. */
export const FieldReference = schema.object({
    /** The field's name. */
    field: schema.string().min(1),
});
/** Another field of the row a comparison reads. */
export type FieldReference = schema.Infer<typeof FieldReference>;

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

/** The value a field compares with, never for a field no condition compares. */
type ComparableOf<Value> =
    Exclude<JsonOf<Value>, null> extends Comparable ? Exclude<JsonOf<Value>, null> : never;

/** A condition on a known row's fields and relations. */
type KnownCondition<Row, Related> = {
    readonly [Name in keyof Row as [ComparableOf<Row[Name]>] extends [never] ? never : Name]?:
        | ComparableOf<Row[Name]>
        | Placeholder
        | FieldCondition<ComparableOf<Row[Name]>>;
} & { readonly [Name in keyof Related]?: true | Related[Name] } & {
    /** Require the row's searched text to contain each word, as a word's prefix. */
    readonly SEARCH?: string;
    /** Require every condition. */
    readonly AND?: readonly KnownCondition<Row, Related>[];
    /** Require any condition. */
    readonly OR?: readonly KnownCondition<Row, Related>[];
    /** Negate a condition. */
    readonly NOT?: KnownCondition<Row, Related>;
};

/** An entry of a condition in its JSON form. */
type JsonEntry = Comparable | readonly Comparable[] | JsonCondition | readonly JsonCondition[];

/** A condition in JSON form, as declarations, rows and the wire store it with `{ placeholder }`. */
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
    Related = Readonly<Record<never, never>>,
> = string extends keyof Row ? OpenCondition : KnownCondition<Row, Related>;

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
        throw DatabaseError.of(
            "QUERY_INVALID",
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

/** Resolve a condition's entries, which must all match. */
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
            throw DatabaseError.of("QUERY_INVALID", "a search needs text");
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
    const operand = operandOf(entry);
    if (operand !== undefined) {
        return { kind: "compare", operator: "eq", name, value: operand };
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
        throw DatabaseError.of("QUERY_INVALID", `condition has no field ${name}`);
    }
    // follow a relation to the related rows meeting a condition
    else {
        return { kind: "exists", via: name, where: conditionOf(name, entry) };
    }
}

/** Resolve a field's comparisons, which must all match. */
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
    const comparisons = readComparisons(entry);
    if (comparisons === undefined) {
        throw DatabaseError.of(
            "QUERY_INVALID",
            `condition field ${name} needs a value or comparisons`,
        );
    }

    return comparisons;
}

/** Read a value as a placeholder or another field, absent for anything else. */
function operandOf(value: unknown): Placeholder | FieldReference | undefined {
    // take a placeholder as it is
    if (value instanceof Placeholder) {
        return value;
    }
    // refuse anything but a JSON form with exactly one key
    else if (!isRecord(value) || Object.keys(value).length !== 1) {
        return undefined;
    }

    // read a placeholder's JSON form
    const { placeholder, field } = value;
    if (isText(placeholder)) {
        return new Placeholder(placeholder);
    }
    // read another field
    else if (isText(field)) {
        return { field };
    }
    // refuse any other key
    else {
        return undefined;
    }
}

/** Read a value as a field's comparisons, absent for anything else. */
function readComparisons(value: unknown): FieldCondition | undefined {
    // require an object
    if (!isRecord(value)) {
        return undefined;
    }

    // read each comparison by its operator, refusing an unknown or invalid one
    const comparisons: { -readonly [Key in keyof FieldCondition]: FieldCondition[Key] } = {};
    for (const [operator, operand] of Object.entries(value)) {
        switch (operator) {
            case "eq":
            case "ne":
            case "lt":
            case "lte":
            case "gt":
            case "gte": {
                const read = isComparable(operand) ? operand : operandOf(operand);
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            case "in":
            case "notIn": {
                const read = readEach(operand, (each) => (isComparable(each) ? each : undefined));
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            case "like":
            case "ilike":
            case "notLike":
            case "notIlike":
            case "startsWith":
            case "endsWith": {
                if (typeof operand !== "string") {
                    return undefined;
                }
                comparisons[operator] = operand;
                break;
            }
            case "characters": {
                if (!isText(operand)) {
                    return undefined;
                }
                comparisons[operator] = operand;
                break;
            }
            case "length": {
                const read = isCount(operand) ? operand : readLength(operand);
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            case "isNull":
            case "isNotNull": {
                if (operand !== true) {
                    return undefined;
                }
                comparisons[operator] = operand;
                break;
            }
            case "AND":
            case "OR": {
                const read = readEach(operand, readComparisons);
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            case "NOT": {
                const read = readComparisons(operand);
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            default:
                return undefined;
        }
    }

    return comparisons;
}

/** Read a value as the comparisons of a text's length, absent for anything else. */
function readLength(value: unknown): LengthCondition | undefined {
    // require an object
    if (!isRecord(value)) {
        return undefined;
    }

    // read each comparison by its operator, refusing an unknown or invalid one
    const comparisons: {
        -readonly [Key in keyof LengthCondition]: LengthCondition[Key];
    } = {};
    for (const [operator, operand] of Object.entries(value)) {
        switch (operator) {
            case "eq":
            case "ne":
            case "lt":
            case "lte":
            case "gt":
            case "gte": {
                if (!isCount(operand)) {
                    return undefined;
                }
                comparisons[operator] = operand;
                break;
            }
            case "in":
            case "notIn": {
                const read = readEach(operand, (each) => (isCount(each) ? each : undefined));
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            case "AND":
            case "OR": {
                const read = readEach(operand, readLength);
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            case "NOT": {
                const read = readLength(operand);
                if (read === undefined) {
                    return undefined;
                }
                comparisons[operator] = read;
                break;
            }
            default:
                return undefined;
        }
    }

    return comparisons;
}

/** Read every element of a list, absent when it is no list or an element is invalid. */
function readEach<Read>(
    value: unknown,
    read: (element: unknown) => Read | undefined,
): Read[] | undefined {
    // require a list
    if (!Array.isArray(value)) {
        return undefined;
    }

    // read each element, refusing the list at an invalid one
    const elements: Read[] = [];
    for (const element of value) {
        const each = read(element);
        if (each === undefined) {
            return undefined;
        }
        elements.push(each);
    }

    return elements;
}

/** Report whether a value is an object, neither a list nor a placeholder. */
function isRecord(value: unknown): value is Record<string, unknown> {
    return (
        typeof value === "object" &&
        value !== null &&
        !Array.isArray(value) &&
        !(value instanceof Placeholder)
    );
}

/** Report whether a value is text, a finite number or a boolean, which a field compares with. */
function isComparable(value: unknown): value is Comparable {
    return (
        typeof value === "string" ||
        typeof value === "boolean" ||
        (typeof value === "number" && Number.isFinite(value))
    );
}

/** Report whether a value is non-empty text. */
function isText(value: unknown): value is string {
    return typeof value === "string" && value.length > 0;
}

/** Report whether a value is a non-negative whole number. */
function isCount(value: unknown): value is number {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

/** Read an entry as one condition. */
function conditionOf(name: string, entry: OpenEntry): OpenCondition {
    if (typeof entry !== "object" || isList(entry) || entry instanceof Placeholder) {
        throw DatabaseError.of("QUERY_INVALID", `condition ${name} needs a condition`);
    }

    return entry;
}

/** Read an entry as a list of conditions. */
function conditionsOf(name: string, entry: OpenEntry): readonly OpenCondition[] {
    if (typeof entry !== "object" || !isList(entry)) {
        throw DatabaseError.of("QUERY_INVALID", `condition ${name} needs a list of conditions`);
    }

    // require each element to be a condition
    const conditions: OpenCondition[] = [];
    for (const element of entry) {
        if (typeof element !== "object" || element instanceof Placeholder) {
            throw DatabaseError.of(
                "QUERY_INVALID",
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
