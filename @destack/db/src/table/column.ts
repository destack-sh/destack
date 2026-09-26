import { assertNever } from "../error/error.ts";
import { type SQL, sql, type SQLWrapper } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import * as identifiers from "@destack/schema/identifier";
import type { Dialect } from "../dialect/dialect.ts";

/** A logical SQL column and its application value. */
export class Column<
    Value = unknown,
    Required extends boolean = boolean,
    Default extends boolean = boolean,
    TableName extends string = string,
    Generated extends boolean = boolean,
> implements SQLWrapper<Value> {
    /** The value and insertion flags used by query inference. */
    declare readonly _: {
        value: Value;
        required: Required;
        default: Default;
        generated: Generated;
    };
    /** The column declaration. */
    readonly definition: ColumnDefinition<Value>;
    /** The SQL table name. */
    readonly table: TableName;

    /** Attach a column declaration to its table. */
    constructor(table: TableName, definition: ColumnDefinition<Value>) {
        this.table = table;
        this.definition = definition;
    }

    /** Return the qualified column expression. */
    getSQL(): SQL<Value> {
        return sql<Value>`${sql.identifier(this.table)}.${sql.identifier(this.definition.name)}`;
    }

    /** Emit column references without parentheses. */
    shouldOmitSQLParens(): boolean {
        return true;
    }

    /** Retain query parameters until a physical dialect supplies their encoding. */
    mapToDriverValue(value: Value): Value {
        return value;
    }

    /** Require a concrete database dialect before decoding a driver value. */
    mapFromDriverValue(_value: unknown): Value {
        throw new TypeError("bind the logical column to a database before decoding values");
    }
}

/** A column declaration before attachment to a table. */
export class ColumnBuilder<
    Value = unknown,
    Required extends boolean = false,
    Default extends boolean = false,
    Generated extends boolean = false,
> {
    /** The value and insertion flags used by table inference. */
    declare readonly _: {
        value: Value;
        required: Required;
        default: Default;
        generated: Generated;
    };
    /** The logical type, validation, defaults, and constraints. */
    readonly definition: ColumnDefinition<Value>;

    /** Retain a logical column declaration. */
    constructor(definition: ColumnDefinition<Value>) {
        this.definition = definition;
    }

    /** Require a value on every persisted row. */
    notNull(): ColumnBuilder<Value, true, Default, Generated> {
        return new ColumnBuilder({ ...this.definition, nullable: false });
    }

    /** Supply a database default when an insert omits the column. */
    default(value: Value | SQL): ColumnBuilder<Value, Required, true, Generated> {
        return new ColumnBuilder({ ...this.definition, default: value });
    }

    /** Supply an application default when an insert omits the column. */
    /* oxlint-disable-next-line destack/prevent-abbreviations -- mirrors the Drizzle column builder method */
    $defaultFn(value: () => Value | SQL): ColumnBuilder<Value, Required, true, Generated> {
        return new ColumnBuilder({ ...this.definition, runtimeDefault: value });
    }

    /** Supply an application value when an update omits the column. */
    /* oxlint-disable-next-line destack/prevent-abbreviations -- mirrors the Drizzle column builder method */
    $onUpdateFn(value: () => Value | SQL): ColumnBuilder<Value, Required, true, Generated> {
        return new ColumnBuilder({ ...this.definition, runtimeUpdate: value });
    }

    /** Declare the table's primary key. */
    primaryKey(): ColumnBuilder<Value, true, Default, Generated> {
        return new ColumnBuilder({ ...this.definition, nullable: false, primaryKey: true });
    }

    /** Keep the value out of logs, audit details, sync and request fingerprints. */
    sensitive(): ColumnBuilder<Value, Required, Default, Generated> {
        return new ColumnBuilder({ ...this.definition, classification: "sensitive" });
    }

    /** Mark the value as personal data, exported and erased with its subject. */
    personal(): ColumnBuilder<Value, Required, Default, Generated> {
        return new ColumnBuilder({ ...this.definition, classification: "personal" });
    }

    /** Require distinct non-null values. */
    unique(name?: string): ColumnBuilder<Value, Required, Default, Generated> {
        return new ColumnBuilder({ ...this.definition, unique: { name } });
    }

    /** Reference a column in another table. */
    references(
        column: () => Column<Value>,
        actions: ReferenceAction = {},
    ): ColumnBuilder<Value, Required, Default, Generated> {
        return new ColumnBuilder({ ...this.definition, reference: { column, ...actions } });
    }

    /** Validate application values with a narrower schema of the same type. */
    validate(validator: schema.Schema<Value>): ColumnBuilder<Value, Required, Default, Generated> {
        const { encode, decode, toJson, fromJson } = this.definition;

        return new ColumnBuilder({
            ...this.definition,
            schema: validator,
            encode: (value, dialect) => encode(validator.parse(value), dialect),
            decode: (value, dialect) => validator.parse(decode(value, dialect)),
            toJson: (value) => toJson(validator.parse(value)),
            fromJson: (value) => validator.parse(fromJson(value)),
        });
    }

    /** Refine the application's static value type. */
    $type<Type extends Value>(): ColumnBuilder<Type, Required, Default, Generated> {
        return new ColumnBuilder(this.definition as unknown as ColumnDefinition<Type>);
    }

    /** Compute a value in SQL with an explicit storage mode. */
    generatedAlwaysAs(
        expression: SQL | (() => SQL),
        options: { readonly mode: "stored" | "virtual" } = { mode: "stored" },
    ): ColumnBuilder<Value, Required, true, true> {
        return new ColumnBuilder({
            ...this.definition,
            generated: { expression, mode: options.mode },
        });
    }
}

/** A logical column's SQL representation and application validation. */
export interface ColumnDefinition<Value = unknown> {
    /** The SQL column name. */
    readonly name: string;
    /** The logical value type. */
    readonly kind:
        | "text"
        | "integer"
        | "real"
        | "boolean"
        | "json"
        | "binary"
        | "bigint"
        | "numeric"
        | "timestamp";
    /** The physical SQL type for each supported dialect. */
    readonly types: Readonly<Record<Dialect, string>>;
    /** The application value validator. */
    readonly schema: schema.Schema;
    /** The declared text values, when restricted to an enum. */
    readonly enumValues?: readonly string[];
    /** Whether the database permits NULL. */
    readonly nullable: boolean;
    /** Whether this column is the primary key. */
    readonly primaryKey?: boolean;
    /** The column's unique constraint. */
    readonly unique?: { readonly name?: string };
    /** The database default. */
    readonly default?: Value | SQL;
    /** The default evaluated by an insert. */
    readonly runtimeDefault?: () => Value | SQL;
    /** The default evaluated by an update. */
    readonly runtimeUpdate?: () => Value | SQL;
    /** The stored generated expression, evaluated after table declaration. */
    readonly generated?: {
        /** The SQL calculation, evaluated after table declaration. */
        readonly expression: SQL | (() => SQL);
        /** Whether SQL stores or recomputes the value. */
        readonly mode: "stored" | "virtual";
    };
    /** The referenced column and referential actions. */
    readonly reference?: ReferenceAction & { readonly column: () => Column<Value> };
    /** How the value is protected: sensitive values never leave the row, personal ones are exportable and erasable. */
    readonly classification?: "sensitive" | "personal";
    /** Encode an application value as a driver parameter. */
    encode(value: Value, dialect: Dialect): unknown;
    /** Decode a driver value as an application value. */
    decode(value: unknown, dialect: Dialect): Value;
    /** Write an application value in its JSON form, the same in every dialect: exact numbers as text, instants as epoch milliseconds, bytes as base64. */
    toJson(value: Value): JsonValue;
    /** Read an application value from its JSON form. */
    fromJson(value: unknown): Value;
}

/** A value in JSON. */
export type JsonValue = schema.Infer<ReturnType<typeof schema.json>>;

/** Referential actions shared by SQLite and PostgreSQL. */
export interface ReferenceAction {
    /** The action when the referenced row is deleted. */
    readonly onDelete?: "cascade" | "restrict" | "no action" | "set null" | "set default";
    /** The action when the referenced key changes. */
    readonly onUpdate?: "cascade" | "restrict" | "no action" | "set null" | "set default";
}

/** Define text, optionally constrained to a set of strings. */
export function text<const Values extends readonly [string, ...string[]]>(
    name: string,
    options?: { readonly enum: Values },
): ColumnBuilder<Values[number]> {
    const validator = options ? schema.enum(options.enum) : schema.string();

    return new ColumnBuilder({
        name,
        kind: "text",
        enumValues: options?.enum,
        types: { sqlite: "text", postgresql: "text" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode: (value) => validator.parse(value),
        decode: (value) => validator.parse(value),
    });
}

/** Define an integer represented exactly by a JavaScript number. */
export function integer(name: string): ColumnBuilder<number> {
    const validator = schema
        .number()
        .int()
        .min(Number.MIN_SAFE_INTEGER)
        .max(Number.MAX_SAFE_INTEGER);

    return new ColumnBuilder({
        name,
        kind: "integer",
        types: { sqlite: "integer", postgresql: "bigint" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode: (value) => validator.parse(value),
        decode: (value) =>
            validator.parse(
                typeof value === "string" || typeof value === "bigint" ? Number(value) : value,
            ),
    });
}

/** Define a double-precision number. */
export function real(name: string): ColumnBuilder<number> {
    const validator = schema.number();

    return new ColumnBuilder({
        name,
        kind: "real",
        types: { sqlite: "real", postgresql: "double precision" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode: (value) => validator.parse(value),
        decode: (value) => validator.parse(value),
    });
}

/** Define a boolean with native PostgreSQL and integer SQLite storage. */
export function boolean(name: string): ColumnBuilder<boolean> {
    const validator = schema.boolean();
    const integer = schema.union([schema.literal(0), schema.literal(1)]);

    return new ColumnBuilder({
        name,
        kind: "boolean",
        types: { sqlite: "integer", postgresql: "boolean" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode(value, dialect) {
            const checked = validator.parse(value);

            // store SQLite booleans as zero or one
            if (dialect === "sqlite") {
                return Number(checked);
            }
            // store native PostgreSQL booleans
            else if (dialect === "postgresql") {
                return checked;
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }
        },
        decode(value, dialect) {
            // read zero or one from SQLite
            if (dialect === "sqlite") {
                const checked = integer.parse(typeof value === "bigint" ? Number(value) : value);

                return checked === 1;
            }
            // read native PostgreSQL booleans
            else if (dialect === "postgresql") {
                return validator.parse(value);
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }
        },
    });
}

/** Define validated JSON with SQLite text and PostgreSQL JSONB storage. */
export function json<Validator extends schema.Schema>(
    name: string,
    validator: Validator,
): ColumnBuilder<schema.Output<Validator>> {
    // require a declarative JSON schema before accepting values
    defineSchema(validator);

    return new ColumnBuilder<schema.Output<Validator>>({
        name,
        kind: "json",
        types: { sqlite: "text", postgresql: "jsonb" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode(value, dialect) {
            const validated = validator.parse(value);

            // store SQLite JSON as text
            if (dialect === "sqlite") {
                return JSON.stringify(validated);
            }
            // pass PostgreSQL JSON to the driver
            else if (dialect === "postgresql") {
                return validated;
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }
        },
        decode(value, dialect) {
            let decoded;

            // parse SQLite JSON text
            if (dialect === "sqlite") {
                decoded = JSON.parse(schema.string().parse(value));
            }
            // take parsed PostgreSQL JSON
            else if (dialect === "postgresql") {
                decoded = value;
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }

            return validator.parse(decoded) as schema.Output<Validator>;
        },
    });
}

/** Define a prefixed UUIDv7 identifier. */
export function identifier<const Prefix extends string>(name: string, prefix: Prefix) {
    const validator = identifiers.identifier(prefix);

    return new ColumnBuilder({
        name,
        kind: "text",
        types: { sqlite: "text", postgresql: "text" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode: (value: schema.Output<typeof validator>) => validator.parse(value),
        decode: (value) => validator.parse(value),
    });
}

/** Define bytes stored as SQLite BLOB or PostgreSQL BYTEA. */
export function binary(name: string): ColumnBuilder<Uint8Array> {
    const validator = schema.instanceof(Uint8Array);

    return new ColumnBuilder({
        name,
        kind: "binary",
        types: { sqlite: "blob", postgresql: "bytea" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value).toBase64(),
        fromJson: (value) => Uint8Array.fromBase64(schema.string().parse(value)),
        encode: (value) => validator.parse(value),
        decode(value) {
            const bytes = validator.parse(value);

            return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        },
    });
}

/** Define an exact signed 64-bit integer. */
export function bigint(name: string): ColumnBuilder<bigint> {
    const validator = schema
        .bigint()
        .min(-(1n << 63n))
        .max((1n << 63n) - 1n);

    return new ColumnBuilder({
        name,
        kind: "bigint",
        types: { sqlite: "integer", postgresql: "bigint" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value).toString(),
        fromJson: (value) => validator.parse(BigInt(schema.string().parse(value))),
        encode: (value) => validator.parse(value),
        decode(value) {
            // reject a driver configuration that has already lost integer precision
            if (typeof value === "number" && !Number.isSafeInteger(value)) {
                throw new RangeError("the database driver returned an inexact integer");
            }

            return validator.parse(
                typeof value === "string" || typeof value === "number" ? BigInt(value) : value,
            );
        },
    });
}

/** Define an exact decimal string with PostgreSQL NUMERIC and SQLite text storage. */
export function numeric(name: string): ColumnBuilder<string> {
    const validator = schema.string().regex(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/);

    return new ColumnBuilder({
        name,
        kind: "numeric",
        types: { sqlite: "text", postgresql: "numeric" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value) as JsonValue,
        fromJson: (value) => validator.parse(value),
        encode: (value) => validator.parse(value),
        decode: (value) => validator.parse(value),
    });
}

/** Define a UTC instant with millisecond precision. */
export function timestamp(name: string): ColumnBuilder<Date> {
    const validator = schema.date();

    return new ColumnBuilder({
        name,
        kind: "timestamp",
        types: { sqlite: "integer", postgresql: "timestamp(3) with time zone" },
        schema: validator,
        nullable: true,
        toJson: (value) => validator.parse(value).getTime(),
        fromJson: (value) => validator.parse(new Date(schema.number().int().parse(value))),
        encode(value, dialect) {
            const checked = validator.parse(value);

            // store SQLite instants as epoch milliseconds
            if (dialect === "sqlite") {
                return checked.getTime();
            }
            // store PostgreSQL instants as ISO strings
            else if (dialect === "postgresql") {
                return checked.toISOString();
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }
        },
        decode(value, dialect) {
            // read SQLite epoch milliseconds
            if (dialect === "sqlite") {
                return validator.parse(
                    new Date(
                        schema
                            .number()
                            .int()
                            .parse(typeof value === "bigint" ? Number(value) : value),
                    ),
                );
            }
            // read PostgreSQL dates or ISO strings
            else if (dialect === "postgresql") {
                return validator.parse(
                    value instanceof Date ? value : new Date(schema.string().parse(value)),
                );
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }
        },
    });
}
