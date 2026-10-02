import { SQL, type SQLWrapper } from "../sql/index.ts";
import { assertNever } from "../error/error.ts";
import { defineSchema, Digest, schema, type Identifier, type JsonValue } from "@destack/schema";
import * as identifiers from "@destack/schema";
import type { Dialect } from "../dialect/dialect.ts";

/** The logical value types of columns. */
export const COLUMN_KINDS = [
    "text",
    "integer",
    "real",
    "boolean",
    "json",
    "binary",
    "blob",
    "bigint",
    "numeric",
] as const;

/** What a foreign key does to referencing rows when their referenced row changes. */
export const REFERENCE_ACTIONS = [
    "cascade",
    "restrict",
    "no action",
    "set null",
    "set default",
] as const;

/** Driver text, such as SQLite's JSON. */
const TEXT = schema.string();

/** A SQLite boolean, stored as zero or one, read as a number or an exact integer. */
const BIT = schema.union([
    schema.literal(0),
    schema.literal(1),
    schema.literal(0n),
    schema.literal(1n),
]);

/** A signed integer in its exact decimal JSON form. */
const INTEGER_TEXT = schema.string().regex(/^-?\d+$(?![\s\S])/u);

/** A logical value type of columns. */
export type ColumnKind = (typeof COLUMN_KINDS)[number];

/** A value a column keeps: JSON, bytes or an exact 64-bit integer. */
export type ColumnValue = JsonValue | Uint8Array | bigint;

/** A value drivers bind and return: text, numbers, booleans, exact integers, bytes or null. */
export type DriverValue = string | number | bigint | boolean | Uint8Array | null;
/** A value drivers bind and return: text, numbers, booleans, exact integers, bytes or null. */
export const DriverValue: schema.Schema<DriverValue> = schema.union([
    schema.string(),
    schema.number(),
    schema.bigint(),
    schema.boolean(),
    schema.instanceof(Uint8Array),
    schema.null(),
]);

/** A logical SQL column. */
export class Column<
    Value extends ColumnValue = ColumnValue,
    Required extends boolean = boolean,
    Default extends boolean = boolean,
    TableName extends string = string,
    Generated extends boolean = boolean,
    Key extends boolean = boolean,
> implements SQLWrapper {
    /** The inference types. */
    declare readonly _: {
        value: Value;
        required: Required;
        default: Default;
        generated: Generated;
        key: Key;
    };
    /** The column declaration. */
    readonly definition: ColumnDefinition<Value>;
    /** The SQL table name. */
    readonly table: TableName;

    /** Create the column. */
    constructor(table: TableName, definition: ColumnDefinition<Value>) {
        this.table = table;
        this.definition = definition;
    }

    /** Report whether a value is a column. */
    static is(value: unknown): value is Column {
        return value instanceof Column;
    }

    /** Embed the column, qualified by its table. */
    getSQL(): SQL {
        return new SQL([this]);
    }
}

/** A column declaration before its table. */
export class ColumnBuilder<
    Value extends ColumnValue = ColumnValue,
    Required extends boolean = false,
    Default extends boolean = false,
    Generated extends boolean = false,
    Key extends boolean = false,
> {
    /** The inference types. */
    declare readonly _: {
        value: Value;
        required: Required;
        default: Default;
        generated: Generated;
        key: Key;
    };
    /** The column's type, validation, defaults and constraints. */
    readonly definition: ColumnDefinition<Value>;

    /** Create the builder. */
    constructor(definition: ColumnDefinition<Value>) {
        this.definition = definition;
    }

    /** Require a value on every persisted row. */
    notNull(): ColumnBuilder<Value, true, Default, Generated, Key> {
        return new ColumnBuilder({ ...this.definition, nullable: false });
    }

    /** Supply a database default when an insert omits the column. */
    default(value: Value | SQL): ColumnBuilder<Value, Required, true, Generated, Key> {
        return new ColumnBuilder({ ...this.definition, default: value });
    }

    /** Make the column part of the table's primary key, in declaration order. */
    primaryKey(): ColumnBuilder<Value, true, Default, Generated, true> {
        return new ColumnBuilder({ ...this.definition, nullable: false, primaryKey: true });
    }

    /** Keep the value out of logs, audit details, sync and request fingerprints. */
    sensitive(): ColumnBuilder<Value, Required, Default, Generated, Key> {
        return new ColumnBuilder({ ...this.definition, classification: "sensitive" });
    }

    /** Mark the value as personal data, exported and erased with its subject. */
    personal(): ColumnBuilder<Value, Required, Default, Generated, Key> {
        return new ColumnBuilder({ ...this.definition, classification: "personal" });
    }

    /** Require distinct non-null values. */
    unique(name?: string): ColumnBuilder<Value, Required, Default, Generated, Key> {
        return new ColumnBuilder({
            ...this.definition,
            unique: name === undefined ? {} : { name },
        });
    }

    /** Reference a column in another table. */
    references(
        column: () => Column<Value>,
        actions: ReferenceAction = {},
    ): ColumnBuilder<Value, Required, Default, Generated, Key> {
        return new ColumnBuilder({ ...this.definition, reference: { column, ...actions } });
    }

    /** Validate values with a narrower schema. */
    validate(
        validator: schema.Schema<Value>,
    ): ColumnBuilder<Value, Required, Default, Generated, Key> {
        const definition = this.definition;

        return new ColumnBuilder({
            ...definition,
            schema: validator,
            encode: (value, dialect) => definition.encode(validator.parse(value), dialect),
            fromJson: (value) => validator.parse(definition.fromJson(value)),
        });
    }

    /** Compute a value in SQL. */
    generatedAlwaysAs(
        expression: SQL | (() => SQL),
        options: { readonly mode: "stored" | "virtual" } = { mode: "stored" },
    ): ColumnBuilder<Value, Required, true, true, Key> {
        return new ColumnBuilder({
            ...this.definition,
            generated: { expression, mode: options.mode },
        });
    }
}

/** A logical column's SQL representation and validation. */
export interface ColumnDefinition<Value extends ColumnValue = ColumnValue> {
    /** The SQL column name. */
    readonly name: string;
    /** The logical value type. */
    readonly kind: ColumnKind;
    /** The SQL type per dialect. */
    readonly types: Readonly<Record<Dialect, string>>;
    /** The application value validator. */
    readonly schema: schema.Schema;
    /** The validator of the JSON form, absent where it is the application value. */
    readonly json?: schema.Schema;
    /** The enum values of text. */
    readonly enumValues?: readonly string[];
    /** Whether the database permits NULL. */
    readonly nullable: boolean;
    /** Whether the column is part of the primary key. */
    readonly primaryKey?: boolean;
    /** The column's unique constraint. */
    readonly unique?: { readonly name?: string };
    /** The database default. */
    readonly default?: Value | SQL;
    /** The generated expression. */
    readonly generated?: {
        /** The SQL calculation. */
        readonly expression: SQL | (() => SQL);
        /** Whether SQL stores or recomputes the value. */
        readonly mode: "stored" | "virtual";
    };
    /** The referenced column and referential actions. */
    readonly reference?: ReferenceAction & { readonly column: () => Column<Value> };
    /** The protection: sensitive values stay in the row, personal values are exportable and erasable. */
    readonly classification?: "sensitive" | "personal";
    /** Encode an application value as a driver parameter. */
    encode(value: unknown, dialect: Dialect): DriverValue;
    /** Decode a driver value as an application value. */
    decode(value: unknown, dialect: Dialect): Value;
    /** Write a value in JSON form: exact numbers as text, instants as epoch milliseconds, bytes as base64. */
    toJson(value: Value): JsonValue;
    /** Read a value from its JSON form. */
    fromJson(value: unknown): Value;
}

/** Referential actions. */
export interface ReferenceAction {
    /** The action when the referenced row is deleted. */
    readonly onDelete?: (typeof REFERENCE_ACTIONS)[number];
    /** The action when the referenced key changes. */
    readonly onUpdate?: (typeof REFERENCE_ACTIONS)[number];
}

/** Define text. */
export function text(name: string): ColumnBuilder<string>;
/** Define text from a set of strings. */
export function text<const Values extends readonly [string, ...string[]]>(
    name: string,
    options: { readonly enum: Values },
): ColumnBuilder<Values[number]>;
/** Define text, whose signatures above type it by its enum. */
export function text(
    name: string,
    options?: { readonly enum: readonly [string, ...string[]] },
): ColumnBuilder<string> {
    // keep the enum values beside the text column
    const types = { sqlite: "text", postgresql: "text" };
    if (options === undefined) {
        return scalarColumn(name, "text", types, schema.string());
    }
    const column = scalarColumn(name, "text", types, schema.enum(options.enum));

    return new ColumnBuilder({ ...column.definition, enumValues: options.enum });
}

/** Define an exact integer. */
export function integer(name: string): ColumnBuilder<number> {
    const validator = schema
        .number()
        .int()
        .min(Number.MIN_SAFE_INTEGER)
        .max(Number.MAX_SAFE_INTEGER);
    const column = scalarColumn(
        name,
        "integer",
        { sqlite: "integer", postgresql: "bigint" },
        validator,
    );

    // read PostgreSQL bigints, which drivers return as text or bigint
    return new ColumnBuilder({
        ...column.definition,
        decode: (value) =>
            validator.parse(
                typeof value === "string" || typeof value === "bigint" ? Number(value) : value,
            ),
    });
}

/** Define a double-precision number. */
export function real(name: string): ColumnBuilder<number> {
    return scalarColumn(
        name,
        "real",
        { sqlite: "real", postgresql: "double precision" },
        schema.number(),
    );
}

/** Define a boolean. */
export function boolean(name: string): ColumnBuilder<boolean> {
    const validator = schema.boolean();
    const column = scalarColumn(
        name,
        "boolean",
        { sqlite: "integer", postgresql: "boolean" },
        validator,
    );

    return new ColumnBuilder({
        ...column.definition,
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
                return Number(BIT.parse(value)) === 1;
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

/** Define validated JSON. */
export function json<Validator extends schema.Schema<JsonValue>>(
    name: string,
    validator: Validator,
): ColumnBuilder<schema.Output<Validator>> {
    // require a declarative schema
    defineSchema(validator);

    return new ColumnBuilder({
        name,
        kind: "json",
        types: { sqlite: "text", postgresql: "jsonb" },
        schema: validator,
        nullable: true,
        toJson: (value) => value,
        fromJson: (value) => validator.parse(value),
        encode: (value) => JSON.stringify(validator.parse(value)),
        decode(value, dialect) {
            // parse SQLite JSON text
            if (dialect === "sqlite") {
                return validator.parse(JSON.parse(TEXT.parse(value)));
            }
            // take parsed PostgreSQL JSON
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

/** Define a prefixed UUIDv7 identifier. */
export function identifier<const Prefix extends string>(
    name: string,
    prefix: Prefix | (() => Prefix),
): ColumnBuilder<Identifier<Prefix>> {
    const validator =
        typeof prefix === "function"
            ? schema.lazy(() => identifiers.identifier(prefix()))
            : identifiers.identifier(prefix);

    return scalarColumn(name, "text", { sqlite: "text", postgresql: "text" }, validator);
}

/** Define bytes. */
export function binary(name: string): ColumnBuilder<Uint8Array> {
    const validator = schema.instanceof(Uint8Array);

    return new ColumnBuilder({
        name,
        kind: "binary",
        types: { sqlite: "blob", postgresql: "bytea" },
        schema: validator,
        json: schema.base64(),
        nullable: true,
        toJson: (value) => value.toBase64(),
        fromJson: (value) => Uint8Array.fromBase64(TEXT.parse(value)),
        encode: (value) => validator.parse(value),
        decode(value) {
            // view driver buffers as plain bytes
            const bytes = validator.parse(value);

            return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        },
    });
}

/** Define a reference to content a blob store keeps: the SHA-256 digest of its bytes, as hexadecimal. */
export function blob(name: string): ColumnBuilder<string> {
    const column = scalarColumn(name, "blob", { sqlite: "text", postgresql: "text" }, Digest);

    return new ColumnBuilder({ ...column.definition, json: Digest });
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
        json: INTEGER_TEXT,
        nullable: true,
        toJson: (value) => value.toString(),
        fromJson: (value) => validator.parse(BigInt(INTEGER_TEXT.parse(value))),
        encode: (value) => validator.parse(value),
        decode(value) {
            // reject integers that already lost precision
            if (typeof value === "number" && !Number.isSafeInteger(value)) {
                throw new RangeError("the database driver returned an inexact integer");
            }

            return validator.parse(
                typeof value === "string" || typeof value === "number" ? BigInt(value) : value,
            );
        },
    });
}

/** Define an exact decimal string. */
export function numeric(name: string): ColumnBuilder<string> {
    return scalarColumn(
        name,
        "numeric",
        { sqlite: "text", postgresql: "numeric" },
        schema.string().regex(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$(?![\s\S])/u),
    );
}

/** Define a column whose JSON form, parameter and driver value are its validated value. */
function scalarColumn<Validator extends schema.Schema<string | number | boolean | null>>(
    name: string,
    kind: ColumnKind,
    types: Readonly<Record<Dialect, string>>,
    validator: Validator,
): ColumnBuilder<schema.Output<Validator>> {
    return new ColumnBuilder({
        name,
        kind,
        types,
        schema: validator,
        nullable: true,
        toJson: (value) => value,
        fromJson: (value) => validator.parse(value),
        encode: (value) => validator.parse(value),
        decode: (value) => validator.parse(value),
    });
}
