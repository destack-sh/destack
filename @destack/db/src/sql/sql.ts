import type { Dialect } from "../dialect/dialect.ts";
import type { Column, ColumnDefinition, ColumnValue } from "../table/column.ts";

/** Something that renders as SQL, such as a column, a table, a query or a fragment. */
export interface SQLWrapper {
    /** Read the fragment this renders as. */
    getSQL(): SQL;
}

/** One part of a fragment: raw text or a value the renderer resolves. */
export type Chunk =
    | string
    | Parameter
    | Placeholder
    | Name
    | DialectSQL
    | SQLWrapper
    | readonly Chunk[];

/** Decode a selected driver value into its application value. */
export type Decoder<Value> = Column | ((value: unknown) => Value);

/** A SQL fragment with the type of the value it computes. */
export class SQL<Value = unknown> implements SQLWrapper {
    /** The computed value type. */
    declare readonly _: { readonly value: Value };
    /** The parts in order. */
    readonly chunks: readonly Chunk[];
    /** How a selection decodes the value, when it is not the driver value. */
    readonly decoder: Decoder<Value> | undefined;

    /** Create the fragment. */
    constructor(chunks: readonly Chunk[], decoder?: Decoder<Value>) {
        this.chunks = chunks;
        this.decoder = decoder;
    }

    /** Read this fragment. */
    getSQL(): SQL {
        return this;
    }

    /** Decode the selected value by a column or a function. */
    mapWith<Next extends ColumnValue>(decoder: Column<ColumnDefinition<Next>>): SQL<Next>;
    /** Decode the selected value by a function. */
    mapWith<Next>(decoder: (value: unknown) => Next): SQL<Next>;
    /** Decode the selected value, whose signatures above type it by the decoder. */
    mapWith(decoder: Decoder<unknown>): SQL {
        return new SQL(this.chunks, decoder);
    }

    /** Alias the value in a selection. */
    as<Alias extends string>(alias: Alias): Aliased<Value, Alias> {
        return new Aliased(this, alias);
    }
}

/** A fragment named in a selection, as `expression AS "alias"`. */
export class Aliased<Value = unknown, Alias extends string = string> implements SQLWrapper {
    /** The named fragment. */
    readonly sql: SQL<Value>;
    /** The name. */
    readonly alias: Alias;

    /** Create the named fragment. */
    constructor(fragment: SQL<Value>, alias: Alias) {
        this.sql = fragment;
        this.alias = alias;
    }

    /** Read the named fragment. */
    getSQL(): SQL {
        return this.sql;
    }
}

/** A value bound as a parameter, encoded by its column when it has one. */
export class Parameter {
    /** The application value. */
    readonly value: unknown;
    /** The column encoding the value. */
    readonly encoder: Column | undefined;

    /** Create the parameter. */
    constructor(value: unknown, encoder?: Column) {
        this.value = value;
        this.encoder = encoder;
    }
}

/** A parameter filled by name when a prepared statement runs. */
export class Placeholder<Name extends string = string> {
    /** The value's name. */
    readonly name: Name;

    /** Create the placeholder. */
    constructor(name: Name) {
        this.name = name;
    }
}

/** A quoted SQL identifier. */
export class Name {
    /** The unquoted identifier. */
    readonly value: string;

    /** Create the identifier. */
    constructor(value: string) {
        this.value = value;
    }
}

/** A fragment with SQL for each dialect. */
export class DialectSQL {
    /** The fragment for each dialect. */
    readonly fragments: Readonly<Record<Dialect, SQL>>;

    /** Create the fragment. */
    constructor(fragments: Readonly<Record<Dialect, SQL>>) {
        this.fragments = fragments;
    }
}

/** Write SQL with values bound as parameters and fragments embedded. */
export const sql = Object.assign(
    function sql<Value = unknown>(
        strings: TemplateStringsArray,
        ...values: readonly unknown[]
    ): SQL<Value> {
        // interleave the text with each value's chunk
        const chunks: Chunk[] = [];
        for (const [index, text] of strings.entries()) {
            chunks.push(text);
            if (index < values.length) {
                chunks.push(chunkOf(values[index]));
            }
        }

        return new SQL<Value>(chunks);
    },
    {
        /** Embed text without binding it. */
        raw<Value = unknown>(text: string): SQL<Value> {
            return new SQL<Value>([text]);
        },
        /** Join fragments and values with a separator. */
        join(values: readonly unknown[], separator?: SQLWrapper): SQL {
            const chunks: Chunk[] = [];
            for (const [index, value] of values.entries()) {
                if (index > 0 && separator !== undefined) {
                    chunks.push(separator);
                }
                chunks.push(chunkOf(value));
            }

            return new SQL(chunks);
        },
        /** Quote an identifier. */
        identifier(value: string): SQL {
            return new SQL([new Name(value)]);
        },
        /** Bind a value, encoded by a column. */
        param<Value>(value: Value, encoder?: Column): SQL<Value> {
            return new SQL<Value>([new Parameter(value, encoder)]);
        },
        /** Bind a value by name when a prepared statement runs. */
        placeholder(name: string): SQL {
            return new SQL([new Placeholder(name)]);
        },
        /** An empty fragment. */
        empty(): SQL {
            return new SQL([]);
        },
    },
);

/** Declare a fragment with SQL for each dialect. */
export function dialectSQL<Value = unknown>(fragments: Readonly<Record<Dialect, SQL>>): SQL<Value> {
    return new SQL<Value>([new DialectSQL(fragments)]);
}

/** Report whether a value renders as SQL. */
export function isSQLWrapper(value: unknown): value is SQLWrapper {
    return (
        typeof value === "object" &&
        value !== null &&
        "getSQL" in value &&
        typeof value.getSQL === "function"
    );
}

/** Read the chunk a template value renders as: fragments embedded, lists in parentheses, others bound. */
function chunkOf(value: unknown): Chunk {
    // embed fragments, columns, tables and queries
    if (isSQLWrapper(value) || value instanceof Placeholder || value instanceof Parameter) {
        return value;
    }
    // list values in parentheses
    else if (Array.isArray(value)) {
        return ["(", sql.join(value, sql.raw(", ")), ")"];
    }
    // bind everything else
    else {
        return new Parameter(value);
    }
}
