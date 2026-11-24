import { Condition, type JsonCondition, Now } from "@destack/db";
import { type Localization, type Message, t } from "@destack/locale";
import type { JsonValue } from "@destack/schema";

/** The comparisons of one field, as a condition has them. */
export type FilterComparison = Exclude<JsonCondition[string], readonly JsonCondition[]>;

/** What a person compares a field with: nothing, one value, several, or two ends. */
export type FilterArity = "none" | "one" | "many" | "range" | "span";

/** A span of time relative to now. */
export interface FilterSpan {
    /** The name a filter keeps it by. */
    readonly name: string;
    /** What it is called. */
    readonly title: Message;
    /** The unit it starts from. */
    readonly unit: "day" | "week";
    /** The days from there to its start. */
    readonly start: number;
    /** The days from there to its end. */
    readonly end: number;
}

/** A day, in milliseconds. */
const DAY = 86_400_000;

/** The spans of time a person filters by. */
export const FILTER_SPANS: readonly FilterSpan[] = [
    { name: "today", title: t`today`, unit: "day", start: 0, end: 1 },
    { name: "yesterday", title: t`yesterday`, unit: "day", start: -1, end: 0 },
    { name: "tomorrow", title: t`tomorrow`, unit: "day", start: 1, end: 2 },
    { name: "this-week", title: t`this week`, unit: "week", start: 0, end: 7 },
    { name: "last-week", title: t`last week`, unit: "week", start: -7, end: 0 },
    { name: "next-week", title: t`next week`, unit: "week", start: 7, end: 14 },
    { name: "last-7-days", title: t`the last 7 days`, unit: "day", start: -6, end: 1 },
    { name: "next-7-days", title: t`the next 7 days`, unit: "day", start: 0, end: 8 },
    { name: "last-30-days", title: t`the last 30 days`, unit: "day", start: -29, end: 1 },
    { name: "next-30-days", title: t`the next 30 days`, unit: "day", start: 0, end: 31 },
];

/** How a field compares with what a person gives. */
export interface FilterOperatorDefinition {
    /** The name a filter keeps it by. */
    readonly name: string;
    /** What it compares with. */
    readonly arity: FilterArity;
    /** Say how it compares with some values, such as "is" with one and "is any of" with more. */
    readonly title: (locale: Localization, count: number) => string;
    /** Write the comparison with some values, undefined while they are incomplete. */
    readonly write: (values: readonly JsonValue[]) => FilterComparison | undefined;
    /** Read the values of a comparison it writes, undefined for another's. */
    readonly read: (comparison: FilterComparison) => readonly JsonValue[] | undefined;
}

/** A scalar a condition compares with. */
type Scalar = string | number | boolean;

/** Report whether a value is one a condition compares with. */
function isScalar(value: unknown): value is Scalar {
    return typeof value === "string" || typeof value === "number" || typeof value === "boolean";
}

/** Read a comparison's entries, none for a plain value. */
function entriesOf(comparison: FilterComparison): Readonly<Record<string, unknown>> {
    return Condition.is(comparison) ? comparison : {};
}

/** Read the only entry of a comparison, undefined where it has others. */
function only(comparison: FilterComparison, name: string): unknown {
    const entries = entriesOf(comparison);

    return Object.keys(entries).length === 1 ? entries[name] : undefined;
}

/** Compare with one value, or with several at once. */
function matching(
    name: string,
    title: FilterOperatorDefinition["title"],
    single: "eq" | "ne",
    several: "in" | "notIn",
): FilterOperatorDefinition {
    return {
        name,
        arity: "many",
        title,
        write: (values) => {
            // compare with the value itself, or with a list of several
            const scalars = values.filter(isScalar);
            const [first] = scalars;
            if (first === undefined) {
                return undefined;
            }

            return scalars.length > 1
                ? { [several]: scalars }
                : single === "eq"
                  ? first
                  : { [single]: first };
        },
        read: (comparison) => {
            // take a plain value as an equal one
            if (isScalar(comparison)) {
                return single === "eq" ? [comparison] : undefined;
            }
            const one = only(comparison, single);
            const list = only(comparison, several);

            // leave a list with a placeholder to what reads it
            if (isScalar(one)) {
                return [one];
            }

            return Array.isArray(list) && list.every(isScalar) ? list : undefined;
        },
    };
}

/** Compare with one value by an operator of the engine. */
function comparing(
    name: string,
    title: FilterOperatorDefinition["title"],
    operator: string,
    wrap: (value: Scalar) => Scalar = (value) => value,
    unwrap: (value: Scalar) => Scalar | undefined = (value) => value,
): FilterOperatorDefinition {
    return {
        name,
        arity: "one",
        title,
        write: ([value]) =>
            isScalar(value)
                ? operator === "eq"
                    ? wrap(value)
                    : { [operator]: wrap(value) }
                : undefined,
        read: (comparison) => {
            // take a plain value as an equal one
            const value =
                operator === "eq" && isScalar(comparison) ? comparison : only(comparison, operator);
            const read = isScalar(value) ? unwrap(value) : undefined;

            return read === undefined ? undefined : [read];
        },
    };
}

/** Compare with nothing: whether there is a value. */
function present(
    name: string,
    title: FilterOperatorDefinition["title"],
    operator: string,
): FilterOperatorDefinition {
    return {
        name,
        arity: "none",
        title,
        write: () => ({ [operator]: true }),
        read: (comparison) => (only(comparison, operator) === true ? [] : undefined),
    };
}

/** Match a missing value too, as "is not Ada" says, where the engine's comparison leaves it out. */
function negating(operator: FilterOperatorDefinition): FilterOperatorDefinition {
    return {
        ...operator,
        write: (values) => {
            const comparison = operator.write(values);

            return Condition.is(comparison) ? { OR: [comparison, { isNull: true }] } : undefined;
        },
        read: (comparison) => {
            // read a program's comparison alone as what it says, too
            const either = only(comparison, "OR");
            const [first, second, ...others] = Array.isArray(either) ? either.filter(isEntry) : [];
            const isEither =
                first !== undefined &&
                second !== undefined &&
                others.length === 0 &&
                only(second, "isNull") === true;

            return operator.read(isEither ? first : comparison);
        },
    };
}

/** Report whether a value is the comparisons of a field. */
function isEntry(value: unknown): value is JsonCondition {
    return Condition.is(value);
}

/** Write text as a pattern that contains it, its own wildcards escaped. */
function contains(value: Scalar): Scalar {
    return `%${String(value).replaceAll(/[\\%_]/gu, (character) => `\\${character}`)}%`;
}

/** Read the text a pattern contains, undefined for another pattern. */
function contained(value: Scalar): Scalar | undefined {
    // take escaped wildcards as text, and no others
    const pattern = typeof value === "string" ? value : "";

    return /^%(?:[^\\%_]|\\[\\%_])*%$/u.test(pattern)
        ? pattern.slice(1, -1).replaceAll(/\\([\\%_])/gu, "$1")
        : undefined;
}

/** Compare with a value, or with any of several. */
const IS = matching(
    "is",
    (locale, count) => (count > 1 ? locale.render(t`is any of`) : locale.render(t`is`)),
    "eq",
    "in",
);

/** Compare with no value of several. */
const IS_NOT = negating(
    matching(
        "is-not",
        (locale, count) => (count > 1 ? locale.render(t`is none of`) : locale.render(t`is not`)),
        "ne",
        "notIn",
    ),
);

/** Find a missing value. */
const IS_EMPTY = present("is-empty", (locale) => locale.render(t`is empty`), "isNull");

/** Find a value that is there. */
const IS_PRESENT = present("is-present", (locale) => locale.render(t`is not empty`), "isNotNull");

/** The comparison of numbers and times with two ends, both included. */
const BETWEEN: FilterOperatorDefinition = {
    name: "between",
    arity: "range",
    title: (locale) => locale.render(t`is between`),
    write: ([start, end]) =>
        isScalar(start) && isScalar(end) ? { gte: start, lte: end } : undefined,
    read: (comparison) => {
        const { gte, lte, ...others } = entriesOf(comparison);

        return isScalar(gte) && isScalar(lte) && Object.keys(others).length === 0
            ? [gte, lte]
            : undefined;
    },
};

/** The comparison of times with a span relative to now. */
const WITHIN: FilterOperatorDefinition = {
    name: "within",
    arity: "span",
    title: (locale) => locale.render(t`is`),
    write: ([name]) => {
        const span = FILTER_SPANS.find((each) => each.name === name);

        return span === undefined ? undefined : writeSpan(span);
    },
    read: (comparison) => {
        const written = JSON.stringify(comparison);
        const span = FILTER_SPANS.find((each) => JSON.stringify(writeSpan(each)) === written);

        return span === undefined ? undefined : [span.name];
    },
};

/** Write a span as the times a field is between. */
function writeSpan({ start, end, unit }: FilterSpan): JsonCondition {
    return {
        gte: { placeholder: new Now(start * DAY, 1, unit).placeholder },
        lt: { placeholder: new Now(end * DAY, 1, unit).placeholder },
    };
}

/** How a field of each kind compares, in the order a menu lists them, the first the default. */
export const FILTER_OPERATORS: Readonly<Record<string, readonly FilterOperatorDefinition[]>> = {
    text: [
        comparing("contains", (locale) => locale.render(t`contains`), "ilike", contains, contained),
        negating(
            comparing(
                "contains-not",
                (locale) => locale.render(t`does not contain`),
                "notIlike",
                contains,
                contained,
            ),
        ),
        comparing("is", (locale) => locale.render(t`is`), "eq"),
        negating(comparing("is-not", (locale) => locale.render(t`is not`), "ne")),
        comparing("starts", (locale) => locale.render(t`starts with`), "startsWith"),
        comparing("ends", (locale) => locale.render(t`ends with`), "endsWith"),
        IS_EMPTY,
        IS_PRESENT,
    ],
    number: [
        comparing("is", (locale) => locale.render(t`is`), "eq"),
        negating(comparing("is-not", (locale) => locale.render(t`is not`), "ne")),
        comparing("above", (locale) => locale.render(t`is more than`), "gt"),
        comparing("least", (locale) => locale.render(t`is at least`), "gte"),
        comparing("below", (locale) => locale.render(t`is less than`), "lt"),
        comparing("most", (locale) => locale.render(t`is at most`), "lte"),
        BETWEEN,
        IS_EMPTY,
        IS_PRESENT,
    ],
    time: [
        WITHIN,
        comparing("before", (locale) => locale.render(t`is before`), "lt"),
        comparing("after", (locale) => locale.render(t`is after`), "gt"),
        comparing("since", (locale) => locale.render(t`is on or after`), "gte"),
        comparing("until", (locale) => locale.render(t`is on or before`), "lte"),
        BETWEEN,
        IS_EMPTY,
        IS_PRESENT,
    ],
    boolean: [comparing("is", (locale) => locale.render(t`is`), "eq")],
    choice: [IS, IS_NOT, IS_EMPTY, IS_PRESENT],
    person: [IS, IS_NOT, IS_EMPTY, IS_PRESENT],
    object: [IS, IS_NOT, IS_EMPTY, IS_PRESENT],
    json: [IS_EMPTY, IS_PRESENT],
};
