import type { Dialect } from "../dialect/dialect.ts";
import { Column, type DriverValue } from "../table/column.ts";
import { TABLE, Table } from "../table/table.ts";
import { DatabaseError } from "../error/error.ts";
import { literal, quote } from "../dialect/quote.ts";
import type { Scalar } from "../query/condition.ts";
import {
    Aliased,
    DialectSQL,
    isSQLWrapper,
    Name,
    Parameter,
    Placeholder,
    SQL,
    type Chunk,
    type SQLWrapper,
} from "./sql.ts";

/** A statement's text and positional parameters, some of them filled by name when it runs. */
export interface Rendered {
    /** The statement text with positional parameter markers. */
    readonly text: string;
    /** The driver values and placeholders in marker order. */
    readonly parameters: readonly (DriverValue | Placeholder)[];
}

/** A statement's text and the driver values of its parameters. */
export interface Bound {
    /** The statement text with positional parameter markers. */
    readonly text: string;
    /** The driver values in marker order. */
    readonly parameters: readonly DriverValue[];
}

/** Render a fragment for a dialect: identifiers quoted, values bound in order. */
export function render(fragment: SQLWrapper, dialect: Dialect): Rendered {
    const parameters: (DriverValue | Placeholder)[] = [];
    const text = renderChunk(fragment, { dialect, parameters });

    return { text, parameters };
}

/** Render a fragment or value as standalone SQL for declarations: values as literals, columns without their table. */
export function inline(value: SQLWrapper | DriverValue, dialect: Dialect): string {
    return renderChunk(isSQLWrapper(value) ? value : new Parameter(value), {
        dialect,
        parameters: undefined,
    });
}

/** Write the right side of a declaration's match with a value: equal to it, or null for null. */
export function match(value: Scalar, dialect: Dialect): string {
    return value === null ? "IS NULL" : `= ${inline(value, dialect)}`;
}

/** Bind a rendered statement's placeholders to named driver values. */
export function fill(
    rendered: Rendered,
    values: Readonly<Record<string, DriverValue>> = {},
): Bound {
    const parameters = rendered.parameters.map((parameter) => {
        // pass bound values through
        if (!(parameter instanceof Placeholder)) {
            return parameter;
        }

        // require each named value
        const value = values[parameter.name];
        if (value === undefined) {
            throw new DatabaseError("INVALID_QUERY", `no value for placeholder ${parameter.name}`);
        }

        return value;
    });

    return { text: rendered.text, parameters };
}

/** Where a render writes values: bound parameters, or literals for a declaration when absent. */
interface Target {
    /** The SQL dialect. */
    readonly dialect: Dialect;
    /** The bound values so far, absent to write literals. */
    readonly parameters: (DriverValue | Placeholder)[] | undefined;
}

/** Render one chunk, binding or writing its values. */
function renderChunk(chunk: Chunk, target: Target): string {
    const dialect = target.dialect;
    // keep raw text
    if (typeof chunk === "string") {
        return chunk;
    }
    // render each listed chunk
    else if (isChunkList(chunk)) {
        return chunk.map((entry) => renderChunk(entry, target)).join("");
    }
    // bind an encoded value, null as it is, or a placeholder
    else if (chunk instanceof Parameter || chunk instanceof Placeholder) {
        const value =
            chunk instanceof Placeholder
                ? chunk
                : chunk.encoder === undefined || chunk.value === null
                  ? driverValue(chunk.value)
                  : chunk.encoder.definition.encode(chunk.value, dialect);

        // write a literal into a declaration, or bind a parameter
        if (target.parameters === undefined) {
            if (value instanceof Placeholder) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `declarations cannot bind placeholder ${value.name}`,
                );
            }

            return literalOf(value, dialect);
        }
        target.parameters.push(value);

        return dialect === "postgresql" ? `$${target.parameters.length}` : "?";
    }
    // quote an identifier
    else if (chunk instanceof Name) {
        return quote(chunk.value);
    }
    // select the dialect's fragment
    else if (chunk instanceof DialectSQL) {
        return renderChunk(chunk.fragments[dialect], target);
    }
    // qualify a column by its table or alias
    else if (Column.is(chunk)) {
        return target.parameters === undefined
            ? quote(chunk.definition.name)
            : `${quote(chunk.table)}.${quote(chunk.definition.name)}`;
    }
    // render a table, or an alias, by its own name
    else if (chunk instanceof Table) {
        return quote(chunk[TABLE].sqlName);
    }
    // render a fragment's parts
    else if (chunk instanceof SQL) {
        return renderChunk(chunk.chunks, target);
    }
    // render a named fragment as its expression
    else if (chunk instanceof Aliased) {
        return renderChunk(chunk.sql, target);
    }
    // render any other wrapper, such as a query, as its fragment
    else {
        return renderChunk(chunk.getSQL(), target);
    }
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

    throw new DatabaseError(
        "INVALID_QUERY",
        `bind a ${typeof value} through its column, since drivers take text, numbers, booleans, bytes and null`,
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
