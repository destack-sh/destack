import {
    Parameter,
    Placeholder,
    combine,
    dialectSQL,
    inArray,
    sql,
    type SQL,
    type SQLWrapper,
} from "../sql/index.ts";
import { Column } from "../table/column.ts";
import type { Row } from "../table/row.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Order } from "./order.ts";
import type { Namespace } from "./namespace.ts";
import { DatabaseError } from "../error/error.ts";
import type { Comparable, Condition, FieldReference, Operator, Scalar } from "./condition.ts";

/** The ASCII capitals, which a case-insensitive pattern lowers. */
const CAPITALS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/** The ASCII small letters the capitals lower to. */
const SMALL = "abcdefghijklmnopqrstuvwxyz";

/** The characters a glob reads as wildcards or sets, which a translated pattern quotes. */
const GLOB_SPECIAL = /[*?[]/gu;

/** The characters a LIKE pattern escapes to match them literally. */
const LIKE_SPECIAL: ReadonlySet<string> = new Set(["%", "_", "\\"]);

/** The characters a regular expression reads as special, which a translated pattern escapes. */
const REGEXP_SPECIAL = /[\\^$.*+?()[\]{}|/]/gu;

/** A character class a `characters` match takes: letters, digits, ranges of either, and `._+/=:@*-`. */
const CHARACTER_SET = /^(?:[a-z]-[a-z]|[A-Z]-[A-Z]|[0-9]-[0-9]|[A-Za-z0-9._+/=:@*-])+$/u;

/** One piece of a character set: a range of letters or digits, or one character. */
const CHARACTER_PIECE = /[a-z]-[a-z]|[A-Z]-[A-Z]|[0-9]-[0-9]|./gu;

/** The SQL each comparison operator renders. */
const OPERATORS = {
    eq: "=",
    ne: "<>",
    lt: "<",
    lte: "<=",
    gt: ">",
    gte: ">=",
} as const satisfies Record<Operator, string>;

/** How a predicate's names resolve: fields to values, placeholders to scalars, relations to decisions. */
export interface Binding<Value, Decision = Value> {
    /** Resolve a field. */
    field(name: string): Value;
    /** Resolve a placeholder's value. */
    placeholder(name: string): Scalar;
    /** Resolve whether a related row meets a condition. */
    exists(via: string, where: Condition): Decision;
}

/** A condition resolved against its row's fields, which rendering and matching walk. */
export type Predicate =
    | {
          readonly kind: "compare";
          readonly operator: Operator;
          readonly name: string;
          readonly value: Comparable | Placeholder | FieldReference;
      }
    | { readonly kind: "oneOf"; readonly name: string; readonly values: readonly Comparable[] }
    | { readonly kind: "missing"; readonly name: string }
    | {
          readonly kind: "like";
          readonly name: string;
          readonly pattern: string;
          readonly isInsensitive: boolean;
      }
    | { readonly kind: "characters"; readonly name: string; readonly set: string }
    | { readonly kind: "length"; readonly name: string; readonly predicate: Predicate }
    | { readonly kind: "all" | "any"; readonly predicates: readonly Predicate[] }
    | { readonly kind: "not"; readonly predicate: Predicate }
    | { readonly kind: "exists"; readonly via: string; readonly where: Condition };

/** A compiled predicate, deciding a row in three-valued logic. */
export type Match = (binding: Binding<unknown, boolean | undefined>) => boolean | undefined;

/** A condition resolved against its row's fields, decided alike in SQL and memory. */
export const Predicate = {
    all,
    fields,
    relations,
    rename,
    size,
    require: requireComparable,
    bind,
    render,
    compile,
    matches,
    unbound,
    literal,
    characters,
};

/** Write a text as a LIKE pattern matching it literally. */
function literal(text: string): string {
    return text.replaceAll(/[%_\\]/gu, (special) => `\\${special}`);
}

/** Read a character set of letters, digits, their ranges and `._+/=:@*-`, a hyphen last. */
function characters(set: string): string {
    if (!CHARACTER_SET.test(set)) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `character set ${set} takes letters, digits, their ranges and ._+/=:@*- alone`,
        );
    }

    // split the set as it was read, refusing a descending range
    const ranges = [...set.matchAll(CHARACTER_PIECE)].map(([piece]) => piece);
    const descending = ranges.find(
        (piece) => piece.length === 3 && piece.charAt(0) > piece.charAt(2),
    );
    if (descending !== undefined) {
        throw new DatabaseError("INVALID_QUERY", `character range ${descending} descends`);
    }

    // move a literal hyphen last, where every dialect's class reads it literally
    return [...ranges.filter((each) => each !== "-"), ...(ranges.includes("-") ? ["-"] : [])].join(
        "",
    );
}

/** Require every predicate, keeping a single one as it is. */
function all(...predicates: Predicate[]): Predicate {
    const [only, ...rest] = predicates;

    return only !== undefined && rest.length === 0 ? only : { kind: "all", predicates };
}

/** List the fields a predicate compares. */
function fields(predicate: Predicate): Set<string> {
    const names = new Set<string>();
    visit(predicate, (visited) => {
        if (
            visited.kind === "compare" ||
            visited.kind === "oneOf" ||
            visited.kind === "missing" ||
            visited.kind === "like" ||
            visited.kind === "characters" ||
            visited.kind === "length"
        ) {
            names.add(visited.name);
        }
        if (visited.kind === "compare" && isReference(visited.value)) {
            names.add(visited.value.field);
        }
    });

    return names;
}

/** List the relations a predicate follows, with their conditions. */
function relations(predicate: Predicate): { readonly via: string; readonly where: Condition }[] {
    const found: { readonly via: string; readonly where: Condition }[] = [];
    visit(predicate, (visited) => {
        if (visited.kind === "exists") {
            found.push({ via: visited.via, where: visited.where });
        }
    });

    return found;
}

/** Rename a predicate's fields, leaving its relations' conditions to the related rows. */
function rename(predicate: Predicate, name: (field: string) => string): Predicate {
    switch (predicate.kind) {
        case "compare":
            return {
                ...predicate,
                name: name(predicate.name),
                value: isReference(predicate.value)
                    ? { field: name(predicate.value.field) }
                    : predicate.value,
            };
        case "oneOf":
        case "missing":
        case "like":
        case "characters":
            return { ...predicate, name: name(predicate.name) };
        case "length":
            return {
                ...predicate,
                name: name(predicate.name),
                predicate: rename(predicate.predicate, name),
            };
        case "all":
        case "any":
            return {
                ...predicate,
                predicates: predicate.predicates.map((nested) => rename(nested, name)),
            };
        case "not":
            return { ...predicate, predicate: rename(predicate.predicate, name) };
        case "exists":
            return predicate;
    }
}

/** Count a predicate's terms: each comparison, listed value, combination and relation. */
function size(predicate: Predicate): number {
    switch (predicate.kind) {
        case "compare":
        case "missing":
        case "like":
        case "characters":
        case "exists":
            return 1;
        case "length":
            return 1 + size(predicate.predicate);
        case "oneOf":
            return Math.max(1, predicate.values.length);
        case "all":
        case "any":
            return predicate.predicates.reduce((total, nested) => total + size(nested), 1);
        case "not":
            return 1 + size(predicate.predicate);
    }
}

/** Visit each predicate of a tree, outside in. */
function visit(predicate: Predicate, visitor: (visited: Predicate) => void): void {
    visitor(predicate);
    if (predicate.kind === "all" || predicate.kind === "any") {
        for (const nested of predicate.predicates) {
            visit(nested, visitor);
        }
    } else if (predicate.kind === "not" || predicate.kind === "length") {
        visit(predicate.predicate, visitor);
    }
}

/** Decide whether a comparison's value is another field of the row. */
function isReference(value: Comparable | Placeholder | FieldReference): value is FieldReference {
    return typeof value === "object" && !(value instanceof Placeholder);
}

/** Require a predicate to compare only comparable columns and computed values. */
function requireComparable(
    predicate: Predicate,
    table: Table,
    namespace: Namespace = { extras: {} },
): void {
    // require a comparable column for each compared field outside the computed values, and text for patterns
    visit(predicate, (visited) => {
        const isCompared = visited.kind === "compare" || visited.kind === "oneOf";
        if (visited.kind === "compare" && isReference(visited.value)) {
            requireSameOrdering(table, visited.name, visited.value.field);
        } else if (isCompared && !Object.hasOwn(namespace.extras, visited.name)) {
            Order.column(table, visited.name);
        } else if (
            visited.kind === "like" ||
            visited.kind === "characters" ||
            visited.kind === "length"
        ) {
            requireText(table, visited.name);
        }
    });
}

/** Require two columns that order as the same values, which a comparison of fields reads. */
function requireSameOrdering(table: Table, left: string, right: string): void {
    const leftOrdering = Order.ordering(table, left);
    const rightOrdering = Order.ordering(table, right);
    if (leftOrdering !== rightOrdering) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `${table[TABLE].name}.${left} orders as ${leftOrdering}, compared with ${right} as ${rightOrdering}`,
        );
    }
}

/** Read a table's column of any kind, refusing a name it has no column of. */
function columnOf(table: Table, name: string): Column {
    const column = table[TABLE].columns[name];
    if (column === undefined) {
        throw new DatabaseError("INVALID_QUERY", `${table[TABLE].name} has no column ${name}`);
    }

    return column;
}

/** Require a pattern's field to be a text column or a computed value. */
function requireText(table: Table, name: string): void {
    const column = table[TABLE].columns[name];
    if (column !== undefined && column.definition.kind !== "text") {
        throw new DatabaseError(
            "INVALID_QUERY",
            `field ${name} matches patterns, not ${column.definition.kind}`,
        );
    }
}

/** Bind a predicate to a table, its namespace and placeholders' values. */
function bind(
    table: Table,
    placeholders: Readonly<Record<string, Scalar>> = {},
    namespace: Namespace = { extras: {} },
): Binding<SQLWrapper> {
    return {
        field: (name) =>
            Object.hasOwn(namespace.extras, name)
                ? Order.expression(table, name, namespace)
                : columnOf(table, name),
        placeholder: (name) => {
            // require a bound value
            return placeholders[name] ?? unbound(name);
        },
        exists: (via, where) => {
            if (namespace.exists === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `condition follows relation ${via}, which ${table[TABLE].name} does not declare`,
                );
            }

            return namespace.exists(via, where);
        },
    };
}

/** Render a predicate as SQL. */
function render(predicate: Predicate, binding: Binding<SQLWrapper>): SQL {
    switch (predicate.kind) {
        case "compare": {
            // bind the value through the field's column
            const field = binding.field(predicate.name);
            const right = renderValue(predicate.value, field, binding);

            return sql`(${field} ${sql.raw(OPERATORS[predicate.operator])} ${right})`;
        }
        case "oneOf": {
            const field = binding.field(predicate.name);
            const values = predicate.values.map((value) => renderValue(value, field, binding));

            return inArray(field, values);
        }
        case "missing":
            return sql`(${binding.field(predicate.name)} IS NULL)`;
        case "like":
            return renderLike(predicate, binding.field(predicate.name));
        case "characters": {
            // match text with no character outside the set
            const field = binding.field(predicate.name);

            return dialectSQL({
                postgresql: sql`(${field} !~ ${`[^${predicate.set}]`})`,
                sqlite: sql`(${field} NOT GLOB ${`*[^${predicate.set}]*`})`,
            });
        }
        case "length": {
            // compare the field's length in characters
            const field = binding.field(predicate.name);
            const length = sql`length(${field})`;

            return render(predicate.predicate, {
                ...binding,
                field: (name) => (name === predicate.name ? length : binding.field(name)),
            });
        }
        case "all":
        case "any": {
            // render empty conjunctions true and disjunctions false
            if (predicate.predicates.length === 0) {
                return predicate.kind === "all" ? sql`true` : sql`false`;
            }
            const joined = predicate.predicates.map((nested) => render(nested, binding));

            return combine(joined, predicate.kind === "all" ? "AND" : "OR");
        }
        case "not":
            return sql`(NOT ${render(predicate.predicate, binding)})`;
        case "exists":
            return sql`${binding.exists(predicate.via, predicate.where)}`;
    }
}

/** Render a pattern match, alike on every dialect: case-sensitive, ASCII-insensitive for ilike, `\\` escaping the next character. */
function renderLike(
    predicate: Extract<Predicate, { readonly kind: "like" }>,
    field: SQLWrapper,
): SQL {
    // lower ASCII capitals for a case-insensitive match
    const pattern = predicate.isInsensitive ? lowerAscii(predicate.pattern) : predicate.pattern;
    const postgres = predicate.isInsensitive
        ? sql`translate(${field}, ${CAPITALS}, ${SMALL})`
        : sql`${field}`;
    const sqlite = predicate.isInsensitive ? sql`lower(${field})` : sql`${field}`;

    // match with LIKE on PostgreSQL and with a case-sensitive glob on SQLite
    return dialectSQL({
        postgresql: sql`(${postgres} LIKE ${escaped(pattern)} ESCAPE '\\')`,
        sqlite: sql`(${sqlite} GLOB ${globOf(pattern)})`,
    });
}

/** One token of a LIKE pattern: any run, any one character, or a literal character. */
type LikeToken = "%" | "_" | { readonly literal: string };

/** Read a LIKE pattern's tokens: `%` any run, `_` any one character, `\\` making the next character literal. */
function likeTokens(pattern: string): LikeToken[] {
    // walk the pattern's code units, which keep each wildcard and escape whole
    const tokens: LikeToken[] = [];
    for (let index = 0; index < pattern.length; index += 1) {
        // read an escaped character literally, refusing an escape ending the pattern
        const character = pattern[index];
        if (character === "\\") {
            const next = pattern[index + 1];
            if (next === undefined) {
                throw new DatabaseError("INVALID_QUERY", "a like pattern ends with its escape");
            }
            tokens.push({ literal: next });
            index += 1;
        }
        // read a wildcard or a literal character
        else if (character === "%" || character === "_") {
            tokens.push(character);
        } else if (character !== undefined) {
            tokens.push({ literal: character });
        }
    }

    return tokens;
}

/** Write a LIKE pattern for PostgreSQL's backslash escape, validating its escapes. */
function escaped(pattern: string): string {
    return likeTokens(pattern).map(escapedToken).join("");
}

/** Write one LIKE token for PostgreSQL's backslash escape. */
function escapedToken(token: LikeToken): string {
    // keep a wildcard
    if (typeof token === "string") {
        return token;
    }
    // escape a literal wildcard or backslash
    else if (LIKE_SPECIAL.has(token.literal)) {
        return `\\${token.literal}`;
    }
    // keep any other literal character
    else {
        return token.literal;
    }
}

/** Translate a LIKE pattern into a glob: `%` any run, `_` any one character, other characters literal. */
function globOf(pattern: string): string {
    return likeTokens(pattern).map(globToken).join("");
}

/** Translate one LIKE token into a glob. */
function globToken(token: LikeToken): string {
    // match any run
    if (token === "%") {
        return "*";
    }
    // match any one character
    else if (token === "_") {
        return "?";
    }
    // match a literal character, quoting a glob's wildcards and sets
    else {
        return token.literal.replaceAll(GLOB_SPECIAL, (special) => `[${special}]`);
    }
}

/** Lower the ASCII capitals of a text, leaving every other character. */
function lowerAscii(text: string): string {
    return text.replaceAll(/[A-Z]/gu, (capital) => capital.toLowerCase());
}

/** Build the expression matching a LIKE pattern: `%` any run, `_` any one character, other characters literal. */
function likeExpression(pattern: string): RegExp {
    const source = likeTokens(pattern).map(expressionToken).join("");

    return new RegExp(`^${source}$`, "su");
}

/** Translate one LIKE token into a regular expression. */
function expressionToken(token: LikeToken): string {
    // match any run
    if (token === "%") {
        return ".*";
    }
    // match any one character
    else if (token === "_") {
        return ".";
    }
    // match a literal character, escaping a regular expression's special characters
    else {
        return token.literal.replaceAll(REGEXP_SPECIAL, "\\$&");
    }
}

/** Render a value, bound through the field's column when its kind encodes values. */
function renderValue(
    value: Comparable | Placeholder | FieldReference,
    field: SQLWrapper,
    binding: Binding<SQLWrapper>,
): SQLWrapper {
    // read another field of the row
    if (isReference(value)) {
        return binding.field(value.field);
    }

    // bind a literal or a placeholder's value
    const scalar = value instanceof Placeholder ? binding.placeholder(value.placeholder) : value;

    return Column.is(field) && scalar !== null && !Column.isPlain(field)
        ? sql`${new Parameter(field.definition.fromJson(scalar), field)}`
        : sql`${scalar}`;
}

/** Compile a predicate, decoding its values as the table's columns keep them, or as scalars without one. */
function compile(predicate: Predicate, table?: Table): Match {
    switch (predicate.kind) {
        case "compare":
            return compileCompare(predicate, table);
        case "oneOf":
            return compileOneOf(predicate, table);
        case "missing":
            return compileMissing(predicate);
        case "like":
            return compileLike(predicate);
        case "characters":
            return compileCharacters(predicate);
        case "length":
            return compileLength(predicate);
        case "all":
        case "any":
            return compileJunction(predicate, table);
        case "not":
            return compileNot(predicate, table);
        case "exists":
            return compileExists(predicate);
    }
}

/** Compile a comparison with a literal, a placeholder or another field of the row. */
function compileCompare(
    predicate: Extract<Predicate, { readonly kind: "compare" }>,
    table?: Table,
): Match {
    const { name, operator, value } = predicate;

    // compare with another field of the row
    if (isReference(value)) {
        const other = value.field;

        return (binding) => {
            const order = Order.values(binding.field(name), binding.field(other));

            return order === undefined ? undefined : satisfies(operator, order);
        };
    }
    // decode a placeholder on each evaluation
    else if (value instanceof Placeholder) {
        const placeholder = value.placeholder;

        return (binding) => {
            // read the value and compare it as the field's column keeps it
            const scalar = binding.placeholder(placeholder);
            const decoded = scalar === null ? null : decode(table, name, scalar);
            const order = Order.values(binding.field(name), decoded);

            return order === undefined ? undefined : satisfies(operator, order);
        };
    }
    // decode a literal once
    else {
        const decoded = decode(table, name, value);

        return (binding) => {
            const order = Order.values(binding.field(name), decoded);

            return order === undefined ? undefined : satisfies(operator, order);
        };
    }
}

/** Compile a match of listed values: nothing for an empty list, unknown for a missing value. */
function compileOneOf(
    predicate: Extract<Predicate, { readonly kind: "oneOf" }>,
    table?: Table,
): Match {
    const name = predicate.name;
    const values = predicate.values.map((value) => decode(table, name, value));

    return (binding) => {
        // match nothing for an empty list and unknown for a missing value
        const value = binding.field(name);
        if (values.length === 0) {
            return false;
        } else if (value === null || value === undefined) {
            return undefined;
        }

        return values.some((entry) => Order.values(value, entry) === 0);
    };
}

/** Compile a match of a missing value. */
function compileMissing(predicate: Extract<Predicate, { readonly kind: "missing" }>): Match {
    const name = predicate.name;

    return (binding) => {
        const value = binding.field(name);

        return value === null || value === undefined;
    };
}

/** Compile a pattern match of text, unknown for a missing value. */
function compileLike(predicate: Extract<Predicate, { readonly kind: "like" }>): Match {
    const { name, isInsensitive } = predicate;
    const expression = likeExpression(
        isInsensitive ? lowerAscii(predicate.pattern) : predicate.pattern,
    );

    return (binding) => {
        // match the text, unknown for a missing value
        const value = binding.field(name);
        if (value === null || value === undefined) {
            return undefined;
        } else if (typeof value !== "string") {
            throw new DatabaseError(
                "INVALID_QUERY",
                `field ${name} matches patterns, not ${typeof value}`,
            );
        }

        return expression.test(isInsensitive ? lowerAscii(value) : value);
    };
}

/** Compile a match of text by its characters, unknown for a missing value. */
function compileCharacters(predicate: Extract<Predicate, { readonly kind: "characters" }>): Match {
    const { name } = predicate;
    const expression = new RegExp(`^[${predicate.set}]*$`, "u");

    return (binding) => {
        const text = textOf(binding.field(name), name);

        return text === undefined ? undefined : expression.test(text);
    };
}

/** Compile a comparison of text's length in characters, unknown for a missing value. */
function compileLength(predicate: Extract<Predicate, { readonly kind: "length" }>): Match {
    // compare lengths as numbers, which no column decodes
    const { name } = predicate;
    const match = compile(predicate.predicate);

    return (binding) => {
        const text = textOf(binding.field(name), name);
        const length = text === undefined ? null : Array.from(text).length;

        return match({
            ...binding,
            field: (field) => (field === name ? length : binding.field(field)),
        });
    };
}

/** Read a field's text, absent for a missing value, refusing a value of another kind. */
function textOf(value: unknown, name: string): string | undefined {
    if (value === null || value === undefined) {
        return undefined;
    } else if (typeof value !== "string") {
        throw new DatabaseError("INVALID_QUERY", `field ${name} matches text, not ${typeof value}`);
    }

    return value;
}

/** Compile a conjunction or disjunction, settling on its deciding value. */
function compileJunction(
    predicate: Extract<Predicate, { readonly kind: "all" | "any" }>,
    table?: Table,
): Match {
    const isDecidedBy = predicate.kind === "any";
    const compiled = predicate.predicates.map((nested) => compile(nested, table));

    return (binding) => {
        // stop at the deciding value, and remember an unknown one
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

/** Compile a negation, unknown staying unknown. */
function compileNot(predicate: Extract<Predicate, { readonly kind: "not" }>, table?: Table): Match {
    const match = compile(predicate.predicate, table);

    return (binding) => {
        const matched = match(binding);

        return matched === undefined ? undefined : !matched;
    };
}

/** Compile a relation test, which the binding decides. */
function compileExists(predicate: Extract<Predicate, { readonly kind: "exists" }>): Match {
    const { via, where } = predicate;

    return (binding) => binding.exists(via, where);
}

/** Refuse a placeholder no value binds, as rendering refuses it. */
function unbound(name: string): never {
    throw new DatabaseError("INVALID_QUERY", `no value for placeholder ${name}`);
}

/** Decide a compiled predicate on a row with relations unknown. */
function matches(match: Match, row: Row): boolean {
    return (
        match({ field: (name) => row[name], placeholder: unbound, exists: () => undefined }) ===
        true
    );
}

/** Decode a scalar as its column keeps it, or keep it for a plain kind, a computed value or a row without a table. */
function decode(table: Table | undefined, name: string, scalar: Scalar): unknown {
    const column = table?.[TABLE].columns[name];

    return column === undefined || Column.isPlain(column)
        ? scalar
        : column.definition.fromJson(scalar);
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
