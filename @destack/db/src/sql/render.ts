import { nth } from "@destack/schema";
import type { Dialect } from "../dialect/dialect.ts";
import type { ColumnCipher } from "../database/encryption.ts";
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
    /** The statement's leading verb, as OpenTelemetry's `db.operation.name` reports it. */
    readonly operation: string;
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
    /** The statement's leading verb, as OpenTelemetry's `db.operation.name` reports it. */
    readonly operation: string;
}

/** The prefix of the slots a kept rendering binds values to, outside any placeholder name. */
const SLOT = "\u0000";

/** The renderings of statement shapes a connection keeps, each rendered once. */
export interface Renderings {
    /** Take a shape's rendering, rendering it the first time. */
    readonly take: (shape: string, render: () => StatementTemplate) => StatementTemplate;
}

/** How to render a statement: its relations' place, its values' cipher, its kept renderings. */
export interface RenderOptions {
    /** The database's namespace in a shared SQLite store, absent for a separate database. */
    readonly namespace?: string | undefined;
    /** The PostgreSQL schema the statement names its relations in, absent for SQLite. */
    readonly schema?: string | undefined;
    /** The cipher encrypting sensitive columns' values, absent for a database without them. */
    readonly cipher?: ColumnCipher | undefined;
    /** The renderings kept by shape, absent to render every statement anew. */
    readonly renderings?: Renderings | undefined;
}

/** Render a fragment for a dialect: quoted identifiers, placed relations, bound values. */
export function render(
    fragment: SQLWrapper,
    dialect: Dialect,
    place: RenderOptions = {},
): StatementTemplate {
    // render anew without kept renderings
    const { renderings } = place;
    if (renderings === undefined) {
        return renderAnew(fragment, dialect, place, false);
    }

    // take the shape's rendering, its values collected in render order
    const values: Parameter[] = [];
    const shape = shapeOf(fragment, dialect, values, new Set());
    const key = `${dialect}\u0000${place.namespace ?? ""}\u0000${place.schema ?? ""}\u0000${shape}`;
    const template = renderings.take(key, () => renderAnew(fragment, dialect, place, true));
    if (values.length === 0) {
        return template;
    }

    // bind each value, encoded through its column, to its slot
    const encoded = values.map((value) => value.encode(dialect, place.cipher));
    const parameters = template.parameters.map((parameter) =>
        parameter instanceof Placeholder && parameter.placeholder.startsWith(SLOT)
            ? (encoded[Number(parameter.placeholder.slice(SLOT.length))] ?? null)
            : parameter,
    );

    return { ...template, parameters };
}

/** Render a fragment, its values bound or as slots for a kept rendering. */
function renderAnew(
    fragment: SQLWrapper,
    dialect: Dialect,
    place: RenderOptions,
    isSlotted: boolean,
): StatementTemplate {
    // render the statement, then define the common tables it reads at its start
    const parameters: (DriverValue | Placeholder)[] = [];
    const commons = new Map<string, string>();
    const named = new Set<string>();
    const text = renderChunk(fragment, {
        dialect,
        namespace: place.namespace,
        schema: place.schema,
        cipher: place.cipher,
        parameters,
        bound: new Map(),
        commons,
        tables: named,
        slots: isSlotted ? { next: 0 } : undefined,
    });
    const tables = [...named];
    const isRead = isReading(text);
    const operation = nth(text.trimStart().split(/\s/u, 1), 0).toUpperCase();
    if (commons.size === 0) {
        return { text, parameters, tables, isRead, operation };
    }
    const defined = [...commons].map(
        ([name, query]) => `${quote(name)} AS NOT MATERIALIZED (${query})`,
    );

    return { text: `WITH ${defined.join(", ")} ${text}`, parameters, tables, isRead, operation };
}

/** Render a fragment or value as standalone SQL: literal values, unqualified columns. */
export function inline(value: SQLWrapper | DriverValue, dialect: Dialect): string {
    return renderChunk(isSQLWrapper(value) ? value : new Parameter(value), {
        dialect,
        namespace: undefined,
        schema: undefined,
        cipher: undefined,
        parameters: undefined,
        bound: new Map(),
        commons: undefined,
        tables: undefined,
        slots: undefined,
    });
}

/** Write a fragment's shape as its rendering reads it, collecting its values in order. */
function shapeOf(
    chunk: Chunk,
    dialect: Dialect,
    values: Parameter[],
    commons: Set<string>,
): string {
    // write text as it is, and each listed chunk
    if (typeof chunk === "string") {
        return chunk;
    } else if (isChunkList(chunk)) {
        return chunk.map((entry) => shapeOf(entry, dialect, values, commons)).join("");
    }
    // collect a value, and write a placeholder by its name
    else if (chunk instanceof Parameter) {
        values.push(chunk);

        return "\u0001";
    } else if (chunk instanceof Placeholder) {
        return `\u0002${chunk.placeholder}`;
    }
    // write names, relations, columns and tables by what they render
    else if (chunk instanceof Name) {
        return `\u0003${chunk.value}`;
    } else if (chunk instanceof RelationName) {
        return `\u0004${chunk.value}`;
    } else if (Column.is(chunk)) {
        return `\u0005${chunk.table}.${chunk.definition.name}${chunk.isAliased ? "@" : ""}`;
    } else if (chunk instanceof Table) {
        const { source, sqlName } = chunk[TABLE];

        return `\u0006${sqlName}${source === undefined ? "" : `@${source[TABLE].sqlName}`}`;
    }
    // write a common table's query at its first read, as the rendering defines it once
    else if (chunk instanceof CommonTable) {
        const isFirst = !commons.has(chunk.name);
        commons.add(chunk.name);

        return `\u0007${chunk.name}${isFirst ? `(${shapeOf(chunk.query, dialect, values, commons)})` : ""}`;
    }
    // write the dialect's fragment, a fragment's chunks, a named fragment and any other value
    else if (chunk instanceof DialectSQL) {
        return shapeOf(chunk.fragments[dialect], dialect, values, commons);
    } else if (chunk instanceof SQL) {
        return shapeOf(chunk.chunks, dialect, values, commons);
    } else if (chunk instanceof Alias) {
        return shapeOf(chunk.sql, dialect, values, commons);
    } else {
        return shapeOf(chunk.getSQL(), dialect, values, commons);
    }
}

/** Report whether a fragment binds no value, so it renders alike on every run. */
export function isConstant(chunk: Chunk): boolean {
    // take text, names, relations, columns and tables as constant
    if (typeof chunk === "string") {
        return true;
    }
    // check each listed chunk
    else if (isChunkList(chunk)) {
        return chunk.every((entry) => isConstant(entry));
    }
    // refuse values and placeholders
    else if (chunk instanceof Parameter || chunk instanceof Placeholder) {
        return false;
    }
    // take identifiers, relation names, columns and tables
    else if (
        chunk instanceof Name ||
        chunk instanceof RelationName ||
        Column.is(chunk) ||
        chunk instanceof Table
    ) {
        return true;
    }
    // check every dialect's fragment
    else if (chunk instanceof DialectSQL) {
        return Object.values(chunk.fragments).every((fragment) => isConstant(fragment));
    }
    // check a common table's query
    else if (chunk instanceof CommonTable) {
        return isConstant(chunk.query);
    }
    // check a fragment's chunks
    else if (chunk instanceof SQL) {
        return isConstant(chunk.chunks);
    }
    // check a named fragment's expression
    else if (chunk instanceof Alias) {
        return isConstant(chunk.sql);
    }
    // check any other SQL value's fragment
    else {
        return isConstant(chunk.getSQL());
    }
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
                `placeholder '${parameter.placeholder}' is invalid: it has no value`,
            );
        }

        return value;
    });

    return {
        text: rendered.text,
        parameters,
        tables: rendered.tables,
        isRead: rendered.isRead,
        operation: rendered.operation,
    };
}

/** Where a render writes values: bound parameters, or literals for a declaration when absent. */
interface Target {
    /** The SQL dialect. */
    readonly dialect: Dialect;
    /** The database's namespace in a shared store, absent for a separate database. */
    readonly namespace: string | undefined;
    /** The PostgreSQL schema relations are named in, absent for SQLite. */
    readonly schema: string | undefined;
    /** The cipher encryption sensitive columns' values, absent for a database without them. */
    readonly cipher: ColumnCipher | undefined;
    /** The common tables the statement reads, by name, absent for a declaration. */
    readonly commons: Map<string, string> | undefined;
    /** The SQLite marker number of each bound value by key, for later occurrences. */
    readonly bound: Map<string, number>;
    /** The bound values so far, absent to write literals. */
    readonly parameters: (DriverValue | Placeholder)[] | undefined;
    /** The SQL names of the tables referenced so far, absent for a declaration. */
    readonly tables: Set<string> | undefined;
    /** The next slot a value binds to in a kept rendering, absent to bind values themselves. */
    readonly slots: { next: number } | undefined;
}

/** Render a column qualified by its table within the database's place, or by its alias. */
function renderColumn(column: Column, target: Target): string {
    if (!column.isAliased) {
        target.tables?.add(column.table);
    }
    const table = column.isAliased ? quote(column.table) : placed(column.table, target);

    return target.parameters === undefined
        ? quote(column.definition.name)
        : `${table}.${quote(column.definition.name)}`;
}

/** Quote a relation in its PostgreSQL schema, or under its SQLite namespace. */
export function placed(name: string, target: RenderOptions): string {
    return target.schema === undefined
        ? quote(relation(name, target.namespace))
        : `${quote(target.schema)}.${quote(name)}`;
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

        return placed(chunk.value, target);
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

        return source === undefined ? placed(sqlName, target) : quote(sqlName);
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
    // write a literal into a declaration, or bind a parameter, or its slot in a kept rendering
    const { dialect, slots } = target;
    const slot =
        slots !== undefined && chunk instanceof Parameter
            ? new Placeholder(`${SLOT}${String(slots.next++)}`)
            : undefined;
    const value =
        slot ?? (chunk instanceof Placeholder ? chunk : chunk.encode(dialect, target.cipher));
    if (target.parameters === undefined) {
        if (value instanceof Placeholder) {
            throw DatabaseError.of(
                "QUERY_INVALID",
                `placeholder '${value.placeholder}' is invalid: declarations cannot bind it`,
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
            `common table '${table.name}' is invalid: declarations cannot read it`,
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
    // refuse another verb, and take a read naming no writing verb, quoted or not
    if (!READ.test(text)) {
        return false;
    } else if (!WRITING.test(text)) {
        return true;
    }

    // look for the writing verb outside the quotes
    return !WRITING.test(text.replace(QUOTED, "''"));
}
