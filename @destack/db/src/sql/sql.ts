import type { Dialect } from "../dialect/dialect.ts";
import type { Column, ColumnDefinition, ColumnValue } from "../table/column.ts";

/** Something that renders as SQL, such as a column, a table, a query or a fragment. */
export interface SQLWrapper {
    /** Read the fragment this renders as. */
    readonly getSQL: () => SQL;
}

/** One chunk of a fragment: raw text or a value the renderer resolves. */
export type Chunk =
    | string
    | Parameter
    | Placeholder
    | Name
    | RelationName
    | CommonTable
    | DialectSQL
    | SQLWrapper
    | readonly Chunk[];

/** Decode a selected driver value into its application value: by column kind or function. */
export type Decoder<Value> =
    | ((value: unknown) => Value)
    | ([Value] extends [ColumnValue] ? ColumnDefinition<Value> : never);

/** A SQL fragment with the type of the value it computes. */
export class SQL<Value = unknown> implements SQLWrapper {
    /** The computed value type. */
    declare readonly _: { readonly value: Value };
    /** The chunks in order. */
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

    /** Decode the selected value by a column definition or a function. */
    mapWith<Next>(decoder: Decoder<Next>): SQL<Next> {
        return new SQL(this.chunks, decoder);
    }

    /** Decode a selected driver value, returning it as the driver returns it without a decoder. */
    decode(value: unknown, dialect: Dialect): unknown {
        // return the driver value
        const decoder = this.decoder;
        if (decoder === undefined) {
            return value;
        }
        // decode by a function
        else if (typeof decoder === "function") {
            return decoder(value);
        }
        // decode as a column definition's values
        else {
            return decoder.decode(value, dialect);
        }
    }

    /** Alias the value in a selection. */
    as<Name extends string>(alias: Name): Alias<Value, Name> {
        return new Alias(this, alias);
    }
}

/** A fragment named in a selection, as `expression AS "alias"`. */
export class Alias<Value = unknown, Name extends string = string> implements SQLWrapper {
    /** The named fragment. */
    readonly sql: SQL<Value>;
    /** The name. */
    readonly alias: Name;

    /** Create the named fragment. */
    constructor(fragment: SQL<Value>, alias: Name) {
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

/** A value filled by name when a statement or condition runs. */
export class Placeholder<Name extends string = string> implements SQLWrapper {
    /** The value's name. */
    readonly placeholder: Name;

    /** Create the placeholder. */
    constructor(name: Name) {
        this.placeholder = name;
    }

    /** Embed the placeholder in a statement. */
    getSQL(): SQL {
        return new SQL([this]);
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

/** A quoted relation name, such as a log table's, within its database's namespace. */
export class RelationName {
    /** The relation's SQL name outside a namespace. */
    readonly value: string;

    /** Create the relation name. */
    constructor(value: string) {
        this.value = value;
    }
}

/** A named query a statement defines once at its start, read by name everywhere in it. */
export class CommonTable implements SQLWrapper {
    /** The name the statement reads it by. */
    readonly name: string;
    /** The query. */
    readonly query: SQL;

    /** Name a query. */
    constructor(name: string, query: SQL) {
        this.name = name;
        this.query = query;
    }

    /** Read the query by its name. */
    getSQL(): SQL {
        return new SQL([this]);
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
        // interleave the template's text with each value's chunk
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
        /** Read a query its statement defines once under a name standing for that query. */
        common(name: string, query: SQL): CommonTable {
            return new CommonTable(name, query);
        },
        /** Quote a relation's name within its database's namespace. */
        relation(value: string): SQL {
            return new SQL([new RelationName(value)]);
        },
        /** Bind a value, encoded by a column. */
        param<Value>(value: Value, encoder?: Column): SQL<Value> {
            return new SQL<Value>([new Parameter(value, encoder)]);
        },
        /** Bind a value by name when a statement or condition runs. */
        placeholder<Name extends string>(name: Name): Placeholder<Name> {
            return new Placeholder(name);
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

/** Read a template value's chunk: fragments embedded, lists in parentheses, others bound. */
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
