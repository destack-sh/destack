import type { Dialect } from "../dialect/dialect.ts";
import { ColumnCipher, SEALED_DIALECT } from "../database/cipher.ts";
import { Column, type DriverValue } from "../table/column.ts";
import { TABLE, Table } from "../table/table.ts";
import { DatabaseError } from "../error/error.ts";
import { literal, quote } from "../dialect/quote.ts";
import { relation } from "../table/namespace.ts";
import type { Scalar } from "../query/condition.ts";
import {
    Alias,
    DialectSQL,
    isSQLWrapper,
    Name,
    Parameter,
    CommonTable,
    Placeholder,
    RelationName,
    SQL,
    type Chunk,
    type SQLWrapper,
} from "./sql.ts";

/** The start of a statement that may only read: SELECT, or common tables ahead of one. */
const READ = /^\s*(?:SELECT|WITH)\b/iu;

/** A verb that writes, as a main statement, a common table's body or a row lock. */
const WRITING = /\b(?:INSERT|UPDATE|DELETE)\b|\bREPLACE\s+INTO\b/iu;

/** A statement's quoted spans: string literals and quoted identifiers. */
const QUOTED = /'(?:[^']|'')*'|"(?:[^"]|"")*"/gu;

/** A statement's text and positional parameters, some of them filled by name when it runs. */
export interface StatementTemplate {
    /** The statement text with positional parameter markers. */
    readonly text: string;
    /** The driver values and placeholders in marker order. */
    readonly parameters: readonly (DriverValue | Placeholder)[];
    /** The SQL tables the statement reads through tables, columns and relations, unqualified. */
    readonly tables: readonly string[];
    /** Whether the statement only reads: it selects, with no writing verb. */
    readonly isRead: boolean;
}

/** A statement's text and the driver values of its parameters. */
export interface DriverStatement {
    /** The statement text with positional parameter markers. */
    readonly text: string;
    /** The driver values in marker order. */
    readonly parameters: readonly DriverValue[];
    /** The SQL tables the statement reads through tables, columns and relations, unqualified. */
    readonly tables: readonly string[];
    /** Whether the statement only reads: it selects, with no writing verb. */
    readonly isRead: boolean;
}

/** Render a fragment for a dialect: quoted identifiers, namespaced relations, bound values. */
export function render(
    fragment: SQLWrapper,
    dialect: Dialect,
    namespace?: string,
    cipher?: ColumnCipher,
): StatementTemplate {
    // render the statement, then define the common tables it reads at its start
    const parameters: (DriverValue | Placeholder)[] = [];
    const commons = new Map<string, string>();
    const named = new Set<string>();
    const text = renderChunk(fragment, {
        dialect,
        namespace,
        cipher,
        parameters,
        bound: new Map(),
        commons,
        tables: named,
    });
    const tables = [...named];
    const isRead = isReading(text);
    if (commons.size === 0) {
        return { text, parameters, tables, isRead };
    }
    const defined = [...commons].map(
        ([name, query]) => `${quote(name)} AS NOT MATERIALIZED (${query})`,
    );

    return { text: `WITH ${defined.join(", ")} ${text}`, parameters, tables, isRead };
}

/** Render a fragment or value as standalone SQL: literal values, unqualified columns. */
export function inline(value: SQLWrapper | DriverValue, dialect: Dialect): string {
    return renderChunk(isSQLWrapper(value) ? value : new Parameter(value), {
        dialect,
        namespace: undefined,
        cipher: undefined,
        parameters: undefined,
        bound: new Map(),
        commons: undefined,
        tables: undefined,
    });
}

/** Write the right side of a declaration's match with a value: equal to it, or null for null. */
export function match(value: Scalar, dialect: Dialect): string {
    return value === null ? "IS NULL" : `= ${inline(value, dialect)}`;
}

/** Bind a rendered statement's placeholders to named driver values. */
export function fill(
    rendered: StatementTemplate,
    values: Readonly<Record<string, DriverValue>> = {},
): DriverStatement {
    const parameters = rendered.parameters.map((parameter) => {
        // pass bound values through
        if (!(parameter instanceof Placeholder)) {
            return parameter;
        }

        // require each named value
        const value = values[parameter.placeholder];
        if (value === undefined) {
            throw DatabaseError.of(
                "QUERY_INVALID",
                `no value for placeholder ${parameter.placeholder}`,
            );
        }

        return value;
    });

    return { text: rendered.text, parameters, tables: rendered.tables, isRead: rendered.isRead };
}

/** Where a render writes values: bound parameters, or literals for a declaration when absent. */
interface Target {
    /** The SQL dialect. */
    readonly dialect: Dialect;
    /** The database's namespace in a shared store, absent for a separate database. */
    readonly namespace: string | undefined;
    /** The cipher sealing sensitive columns' values, absent for a database without them. */
    readonly cipher: ColumnCipher | undefined;
    /** The common tables the statement reads, by name, absent for a declaration. */
    readonly commons: Map<string, string> | undefined;
    /** The SQLite marker number of each bound value by key, for later occurrences. */
    readonly bound: Map<string, number>;
    /** The bound values so far, absent to write literals. */
    readonly parameters: (DriverValue | Placeholder)[] | undefined;
    /** The SQL names of the tables referenced so far, absent for a declaration. */
    readonly tables: Set<string> | undefined;
}

/** Render a column qualified by its table within the database's namespace, or by its alias. */
function renderColumn(column: Column, target: Target): string {
    if (!column.isAliased) {
        target.tables?.add(column.table);
    }
    const table = column.isAliased ? column.table : relation(column.table, target.namespace);

    return target.parameters === undefined
        ? quote(column.definition.name)
        : `${quote(table)}.${quote(column.definition.name)}`;
}

/** Render one chunk, binding or writing its values. */
function renderChunk(chunk: Chunk, target: Target): string {
    const dialect = target.dialect;
    // write raw text as it is
    if (typeof chunk === "string") {
        return chunk;
    }
    // render each listed chunk
    else if (isChunkList(chunk)) {
        return chunk.map((entry) => renderChunk(entry, target)).join("");
    }
    // bind an encoded value, null as it is, or a placeholder
    else if (chunk instanceof Parameter || chunk instanceof Placeholder) {
        return bind(chunk, target);
    }
    // quote an identifier
    else if (chunk instanceof Name) {
        return quote(chunk.value);
    }
    // read a common table by name, defining it once for the statement
    else if (chunk instanceof CommonTable) {
        return quote(define(chunk, target));
    }
    // quote a relation within the database's namespace
    else if (chunk instanceof RelationName) {
        target.tables?.add(chunk.value);

        return quote(relation(chunk.value, target.namespace));
    }
    // select the dialect's fragment
    else if (chunk instanceof DialectSQL) {
        return renderChunk(chunk.fragments[dialect], target);
    }
    // qualify a column by its table or alias
    else if (Column.is(chunk)) {
        return renderColumn(chunk, target);
    }
    // render an alias by its name, and a table within the database's namespace
    else if (chunk instanceof Table) {
        const { source, sqlName } = chunk[TABLE];
        target.tables?.add((source ?? chunk)[TABLE].sqlName);

        return quote(source === undefined ? relation(sqlName, target.namespace) : sqlName);
    }
    // render a fragment's chunks
    else if (chunk instanceof SQL) {
        return renderChunk(chunk.chunks, target);
    }
    // render a named fragment as its expression
    else if (chunk instanceof Alias) {
        return renderChunk(chunk.sql, target);
    }
    // render any other SQL value, such as a query, as its fragment
    else {
        return renderChunk(chunk.getSQL(), target);
    }
}

/** Bind a value or a placeholder as a marker, or write it as a literal into a declaration. */
function bind(chunk: Parameter | Placeholder, target: Target): string {
    // write a literal into a declaration, or bind a parameter
    const { dialect } = target;
    const value = chunk instanceof Placeholder ? chunk : encoded(chunk, target);
    if (target.parameters === undefined) {
        if (value instanceof Placeholder) {
            throw DatabaseError.of(
                "QUERY_INVALID",
                `declarations cannot bind placeholder ${value.placeholder}`,
            );
        }

        return literalOf(value, dialect);
    }
    // number each PostgreSQL value apart, since it types each marker by one use
    if (dialect === "postgresql") {
        target.parameters.push(value);

        return `$${target.parameters.length}`;
    }

    // bind each distinct SQLite value once, every later occurrence reusing its marker
    const key = keyOf(value);
    const bound = target.bound.get(key);
    if (bound !== undefined) {
        return `?${bound}`;
    }
    target.parameters.push(value);
    target.bound.set(key, target.parameters.length);

    return `?${target.parameters.length}`;
}

/** Encode a parameter's value through its column, sealing a sensitive column's value. */
function encoded(parameter: Parameter, target: Target): DriverValue {
    // take null and a value without a column as they are
    const { encoder } = parameter;
    if (encoder === undefined || parameter.value === null) {
        return driverValue(parameter.value);
    }

    // encode through the column, or seal a sensitive value's text-stored form
    const { classification, sealing, name } = encoder.definition;
    if (classification !== "sensitive" || sealing === undefined) {
        return encoder.definition.encode(parameter.value, target.dialect);
    }
    const value = encoder.definition.encode(parameter.value, SEALED_DIALECT);

    return ColumnCipher.require(target.cipher, encoder).seal(value, name, sealing);
}

/** Require a value bound without a column to be one the drivers take as it is. */
function driverValue(value: unknown): DriverValue {
    if (
        value === null ||
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "boolean" ||
        typeof value === "bigint" ||
        value instanceof Uint8Array
    ) {
        return value;
    }

    throw DatabaseError.of(
        "QUERY_INVALID",
        `cannot bind a ${typeof value} without its column: drivers accept text, numbers, booleans, bytes and null`,
    );
}

/** Report whether a chunk lists chunks. */
function isChunkList(chunk: Chunk): chunk is readonly Chunk[] {
    return Array.isArray(chunk);
}

/** Write a driver value as a SQL literal. */
function literalOf(value: DriverValue, dialect: Dialect): string {
    // write scalars
    if (value === null) {
        return "NULL";
    } else if (typeof value === "string") {
        return literal(value);
    } else if (typeof value === "number" || typeof value === "bigint") {
        return value.toString();
    } else if (typeof value === "boolean") {
        return dialect === "sqlite" ? (value ? "1" : "0") : value ? "TRUE" : "FALSE";
    }
    // write bytes as a blob literal, or decoded hexadecimal in PostgreSQL
    else {
        return dialect === "sqlite" ? `X'${value.toHex()}'` : `decode('${value.toHex()}', 'hex')`;
    }
}

/** Key a bound value or placeholder, telling apart values of different kinds. */
function keyOf(value: DriverValue | Placeholder): string {
    // key a placeholder by its name, and bytes by their hexadecimal
    if (value instanceof Placeholder) {
        return `placeholder:${value.placeholder}`;
    } else if (value instanceof Uint8Array) {
        return `bytes:${value.toHex()}`;
    }

    return `${value === null ? "null" : typeof value}:${String(value)}`;
}

/** Define a common table for the statement at its first read, its name standing for its query. */
function define(table: CommonTable, target: Target): string {
    // refuse a common table in a declaration, which defines none
    const { commons } = target;
    if (commons === undefined) {
        throw DatabaseError.of(
            "QUERY_INVALID",
            `declarations cannot read common table ${table.name}`,
        );
    }

    // render its query at its first read
    if (!commons.has(table.name)) {
        commons.set(table.name, renderChunk(table.query, target));
    }

    return table.name;
}

/** Report whether a statement only reads: it selects, with no writing verb outside its quotes. */
function isReading(text: string): boolean {
    const bare = text.replace(QUOTED, "''");

    return READ.test(bare) && !WRITING.test(bare);
}
