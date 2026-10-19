import { SQL, type SQLWrapper } from "../sql/index.ts";
import { assertNever } from "../error/error.ts";
import { schema, type Identifier, type JsonValue } from "@destack/schema";
import type { Dialect } from "../dialect/dialect.ts";

/** How a sensitive value is sealed: randomly, or alike for a column looked up by value. */
export type Sealing = "random" | "deterministic";

/** The SQL type of a sensitive column, which keeps its value sealed as text. */
const SEALED_TYPES = { sqlite: "text", postgresql: "text" } as const;

/** The logical value types of columns. */
export const COLUMN_KINDS = [
    "text",
    "integer",
    "real",
    "boolean",
    "json",
    "binary",
    "bigint",
    "numeric",
] as const;

/** What a foreign key does to referencing rows on a change, refusals checked at deferred commit. */
export const REFERENCE_ACTIONS = ["cascade", "no action", "set null", "set default"] as const;

/** The column kinds whose values are their own JSON form and driver parameter. */
const PLAIN_KINDS: ReadonlySet<ColumnKind> = new Set<ColumnKind>(["text", "integer", "real"]);

/** Driver text, such as SQLite's JSON. */
const TEXT = schema.string();

/** A SQLite boolean, stored as zero or one, read as a number or an exact integer. */
const BIT = schema.union([
    schema.literal(0),
    schema.literal(1),
    schema.literal(0n),
    schema.literal(1n),
]);

/** The JavaScript type each scalar column kind stores. */
const SCALAR_TYPES = {
    text: "string",
    numeric: "string",
    integer: "number",
    real: "number",
    boolean: "boolean",
} as const satisfies Partial<Record<ColumnKind, "string" | "number" | "boolean">>;

/** A decimal number as text, in fixed or exponent notation. */
const DECIMAL = schema.string().regex(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/u);

/** A signed integer in its exact decimal JSON form. */
const INTEGER_TEXT = schema.string().regex(/^-?\d+$/u);

/** A logical value type of columns. */
export type ColumnKind = (typeof COLUMN_KINDS)[number];

/** A value a column keeps: JSON, bytes or an exact 64-bit integer. */
export type ColumnValue = JsonValue | Uint8Array | bigint;
/** A value a column keeps: JSON, bytes or an exact 64-bit integer. */
export const ColumnValue: schema.Schema<ColumnValue> = schema.union([
    schema.json(),
    schema.instanceof(Uint8Array),
    schema.bigint(),
]);

/** The JSON form of a column value: exact integers and bytes as text. */
export type JsonOf<Value> = Value extends bigint | Uint8Array ? string : Value;

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
    Definition extends ColumnDefinition = ColumnDefinition,
    TableName extends string = string,
> implements SQLWrapper {
    /** The column declaration. */
    readonly definition: Definition;
    /** The SQL table name. */
    readonly table: TableName;
    /** Whether the table name is a query alias, which no namespace qualifies. */
    readonly isAliased: boolean;

    /** Create the column of a table or of a query alias. */
    constructor(table: TableName, definition: Definition, isAliased = false) {
        this.table = table;
        this.definition = definition;
        this.isAliased = isAliased;
    }

    /** Report whether a value is a column. */
    static is(value: unknown): value is Column {
        return value instanceof Column;
    }

    /** Report whether a column's values are their own JSON form and parameter. */
    static isPlain(column: Column): boolean {
        return PLAIN_KINDS.has(column.definition.kind);
    }

    /** Embed the column, qualified by its table. */
    getSQL(): SQL {
        return new SQL([this]);
    }
}

/** A column declaration before its table. */
export class ColumnBuilder<Definition extends ColumnDefinition = ColumnDefinition> {
    /** The column's type, validation, defaults and constraints. */
    readonly definition: Definition;

    /** Create the builder. */
    constructor(definition: Definition) {
        this.definition = definition;
    }

    /** Require a value on every persisted row. */
    notNull(): ColumnBuilder<Definition & { readonly nullable: false }> {
        return new ColumnBuilder({ ...this.definition, nullable: false as const });
    }

    /** Supply a database default when an insert omits the column. */
    default(
        value: ValueOf<Definition> | SQL,
    ): ColumnBuilder<Definition & { readonly default: ValueOf<Definition> | SQL }> {
        return new ColumnBuilder({ ...this.definition, default: value });
    }

    /** Make the column part of the table's primary key, in declaration order. */
    primaryKey(): ColumnBuilder<
        Definition & { readonly nullable: false; readonly primaryKey: true }
    > {
        return new ColumnBuilder({
            ...this.definition,
            nullable: false as const,
            primaryKey: true as const,
        });
    }

    /** Seal the value at rest and keep it out of logs, audits, sync and fingerprints. */
    sensitive(
        sealing: Sealing = "random",
    ): ColumnBuilder<Definition & { readonly classification: "sensitive" }> {
        return new ColumnBuilder({
            ...this.definition,
            // keep the sealed text, whatever the value's own type
            types: SEALED_TYPES,
            classification: "sensitive" as const,
            sealing,
            schema: schema.sensitive(this.definition.schema.clone()),
            json: schema.sensitive(this.definition.json.clone()),
        });
    }

    /** Mark the value personal in its schemas, masked when concealed. */
    personal(): ColumnBuilder<Definition> {
        return new ColumnBuilder({
            ...this.definition,
            schema: schema.sensitive(this.definition.schema.clone(), "personal"),
            json: schema.sensitive(this.definition.json.clone(), "personal"),
        });
    }

    /** Require distinct non-null values. */
    unique(name?: string): ColumnBuilder<Definition> {
        return new ColumnBuilder({
            ...this.definition,
            unique: name === undefined ? {} : { name },
        });
    }

    /** Reference a column in another table with the same values. */
    references(column: () => Column, actions: ReferenceAction = {}): ColumnBuilder<Definition> {
        return new ColumnBuilder({ ...this.definition, reference: { column, ...actions } });
    }

    /** Validate values that are their own JSON form with a narrower schema, in both forms. */
    validate(
        this: ColumnBuilder<Definition & { readonly json: schema.Schema<ValueOf<Definition>> }>,
        validator: schema.Schema<ValueOf<Definition>>,
    ): ColumnBuilder<Definition> {
        const definition = this.definition;

        return new ColumnBuilder({
            ...definition,
            schema: validator,
            json: validator,
            fromJson: (value: unknown) => validator.parse(definition.fromJson(value)),
        });
    }

    /** Compute a value in SQL. */
    generatedAlwaysAs(
        expression: SQL | (() => SQL),
        options: { readonly mode: "stored" | "virtual" } = { mode: "stored" },
    ): ColumnBuilder<Definition & { readonly generated: ColumnGeneration }> {
        return new ColumnBuilder({
            ...this.definition,
            generated: { expression, mode: options.mode },
        });
    }
}

/** A logical column's SQL representation and validation. */
export interface ColumnDefinition<
    Value extends ColumnValue = ColumnValue,
    Json extends JsonValue = JsonValue,
> {
    /** The SQL column name. */
    readonly name: string;
    /** The logical value type. */
    readonly kind: ColumnKind;
    /** The SQL type per dialect. */
    readonly types: Readonly<Record<Dialect, string>>;
    /** The application value validator. */
    readonly schema: schema.Schema;
    /** The validator of the JSON form. */
    readonly json: schema.Schema<Json>;
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
    readonly generated?: ColumnGeneration;
    /** The referenced column and referential actions. */
    readonly reference?: ReferenceAction & {
        readonly column: () => Column<ColumnDefinition<Value>>;
    };
    /** The protection: sensitive values stay in the row and out of the log. */
    readonly classification?: "sensitive";
    /** How a sensitive value is sealed, absent for an opaque copy of sealed text. */
    readonly sealing?: Sealing;
    /** Convert a valid application value to a driver parameter, which its write validated. */
    readonly encode: (value: unknown, dialect: Dialect) => DriverValue;
    /** Convert a stored driver value to an application value, which its write validated. */
    readonly decode: (value: unknown, dialect: Dialect) => Value;
    /** Write a value as JSON: exact numbers as text, instants as epoch ms, bytes as base64. */
    toJson(value: Value): JsonValue;
    /** Read a value from its JSON form. */
    readonly fromJson: (value: unknown) => Value;
}

/** A column's SQL calculation. */
export interface ColumnGeneration {
    /** The SQL calculation. */
    readonly expression: SQL | (() => SQL);
    /** Whether SQL stores or recomputes the value. */
    readonly mode: "stored" | "virtual";
}

/** The application value a column definition reads. */
export type ValueOf<Definition extends ColumnDefinition> =
    ReturnType<Definition["fromJson"]> extends infer Value extends ColumnValue ? Value : never;

/** Referential actions. */
export interface ReferenceAction {
    /** The action when the referenced row is deleted. */
    readonly onDelete?: (typeof REFERENCE_ACTIONS)[number];
    /** The action when the referenced key changes. */
    readonly onUpdate?: (typeof REFERENCE_ACTIONS)[number];
}

/** A column definition of a value, kind and JSON form, as its constructor declares it. */
type ColumnDefinitionOf<
    Value extends ColumnValue,
    Kind extends ColumnKind,
    Json extends JsonValue = JsonOf<Value>,
> = ColumnDefinition<Value, Json> & { readonly kind: Kind };

/** The SQL types of a text column. */
const TEXT_TYPES = { sqlite: "text", postgresql: "text" };

/** Define text. */
export function text(name: string): ColumnBuilder<ColumnDefinitionOf<string, "text">> {
    return scalarColumn(name, "text", TEXT_TYPES, schema.string());
}

/** Define text taking one of a set of strings, kept beside the column. */
export function textEnum<const Values extends readonly [string, ...string[]]>(
    name: string,
    values: Values,
): ColumnBuilder<ColumnDefinitionOf<Values[number], "text", Values[number]>> {
    const column = scalarColumn(name, "text", TEXT_TYPES, schema.literal(values));

    return new ColumnBuilder<ColumnDefinitionOf<Values[number], "text", Values[number]>>({
        ...column.definition,
        enumValues: values,
    });
}

/** Define an exact integer. */
export function integer(name: string): ColumnBuilder<ColumnDefinitionOf<number, "integer">> {
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
            stored(
                typeof value === "string" || typeof value === "bigint" ? Number(value) : value,
                "number",
            ),
    });
}

/** Define a double-precision number. */
export function real(name: string): ColumnBuilder<ColumnDefinitionOf<number, "real">> {
    return scalarColumn(
        name,
        "real",
        { sqlite: "real", postgresql: "double precision" },
        schema.number(),
    );
}

/** Define a boolean. */
export function boolean(name: string): ColumnBuilder<ColumnDefinitionOf<boolean, "boolean">> {
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
            const checked = stored(value, "boolean");

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
                return stored(value, "boolean");
            }
            // reject other dialects
            else {
                return assertNever(dialect);
            }
        },
    });
}

/** Define JSON validated on write and read back as stored, migrated as its schema changes. */
export function json<Value extends JsonValue>(
    name: string,
    validator: schema.Schema<Value>,
): ColumnBuilder<ColumnDefinitionOf<Value, "json", Value>> {
    return new ColumnBuilder<ColumnDefinitionOf<Value, "json", Value>>({
        name,
        kind: "json",
        types: { sqlite: "text", postgresql: "jsonb" },
        schema: validator,
        json: validator,
        nullable: true,
        toJson: (value) => value,
        fromJson: (value) => validator.parse(value),
        encode: (value) => JSON.stringify(value),
        decode: (value, dialect) => {
            // read the stored JSON as its write validated it
            // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- stored JSON passed its write's validator
            return storedJson(value, dialect) as Value;
        },
    });
}

/** Read a JSON column's stored value as the driver returns it: SQLite's text parsed. */
function storedJson(value: unknown, dialect: Dialect): unknown {
    // parse SQLite JSON text
    if (dialect === "sqlite") {
        const parsed: unknown = JSON.parse(stored(value, "string"));

        return parsed;
    }
    // take parsed PostgreSQL JSON
    else if (dialect === "postgresql") {
        return value;
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Define a prefixed UUIDv7 identifier, whose JSON form is its text. */
export function identifier<const Prefix extends string>(
    name: string,
    prefix: Prefix | (() => Prefix),
): ColumnBuilder<ColumnDefinitionOf<Identifier<Prefix>, "text", string>> {
    const validator =
        typeof prefix === "function"
            ? schema.lazy(() => schema.identifier(prefix()))
            : schema.identifier(prefix);

    return new ColumnBuilder<ColumnDefinitionOf<Identifier<Prefix>, "text", string>>({
        name,
        kind: "text",
        types: { sqlite: "text", postgresql: "text" },
        schema: validator,
        json: validator,
        nullable: true,
        toJson: (value) => value,
        fromJson: (value) => validator.parse(value),
        encode: (value) => stored(value, "string"),
        decode: (value) => validator.parse(value),
    });
}

/** Define bytes. */
export function binary(name: string): ColumnBuilder<ColumnDefinitionOf<Uint8Array, "binary">> {
    const validator = schema.instanceof(Uint8Array);

    return new ColumnBuilder<ColumnDefinitionOf<Uint8Array, "binary">>({
        name,
        kind: "binary",
        types: { sqlite: "blob", postgresql: "bytea" },
        schema: validator,
        json: schema.base64(),
        nullable: true,
        toJson: (value) => value.toBase64(),
        fromJson: (value) => Uint8Array.fromBase64(TEXT.parse(value)),
        encode: (value) => stored(value, "bytes"),
        decode(value) {
            // view driver buffers as plain bytes
            const bytes = stored(value, "bytes");

            return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        },
    });
}

/** Define an exact signed 64-bit integer. */
export function bigint(name: string): ColumnBuilder<ColumnDefinitionOf<bigint, "bigint">> {
    const validator = schema
        .bigint()
        .min(-(1n << 63n))
        .max((1n << 63n) - 1n);

    return new ColumnBuilder<ColumnDefinitionOf<bigint, "bigint">>({
        name,
        kind: "bigint",
        types: { sqlite: "integer", postgresql: "bigint" },
        schema: validator,
        json: INTEGER_TEXT,
        nullable: true,
        toJson: (value) => value.toString(),
        fromJson: (value) => validator.parse(BigInt(INTEGER_TEXT.parse(value))),
        encode: (value) => stored(value, "bigint"),
        decode(value) {
            // reject integers that already lost precision
            if (typeof value === "number" && !Number.isSafeInteger(value)) {
                throw new RangeError("the database driver returned an inexact integer");
            }

            return stored(
                typeof value === "string" || typeof value === "number" ? BigInt(value) : value,
                "bigint",
            );
        },
    });
}

/** Define an exact decimal string. */
export function numeric(name: string): ColumnBuilder<ColumnDefinitionOf<string, "numeric">> {
    // read SQLite's arithmetic results as decimal text
    return scalarColumn(
        name,
        "numeric",
        { sqlite: "text", postgresql: "numeric" },
        DECIMAL,
        decimalText,
    );
}

/** Write a driver's number or exact integer as decimal text, keeping any other value. */
function decimalText(value: unknown): unknown {
    return typeof value === "number" || typeof value === "bigint" ? value.toString() : value;
}

/** Define a column whose JSON form, parameter and driver value are its validated value. */
function scalarColumn<
    Value extends string | number | boolean | null,
    Kind extends keyof typeof SCALAR_TYPES,
>(
    name: string,
    kind: Kind,
    types: Readonly<Record<Dialect, string>>,
    validator: schema.Schema<Value>,
    read: (value: unknown) => unknown = (value) => value,
): ColumnBuilder<ColumnDefinitionOf<Value, Kind, Value>> {
    return new ColumnBuilder<ColumnDefinitionOf<Value, Kind, Value>>({
        name,
        kind,
        types,
        schema: validator,
        json: validator,
        nullable: true,
        toJson: (value) => value,
        fromJson: (value) => validator.parse(value),
        encode: (value) => stored(value, SCALAR_TYPES[kind]),
        decode: (value) => validator.parse(read(value)),
    });
}

/** The JavaScript type of each driver value a column stores, by name. */
interface StoredTypes {
    /** Text. */
    readonly string: string;
    /** A double-precision number. */
    readonly number: number;
    /** A boolean. */
    readonly boolean: boolean;
    /** An exact integer. */
    readonly bigint: bigint;
    /** Bytes. */
    readonly bytes: Uint8Array;
}

/** Narrow a driver value to the type a column stores, refusing a value of another type. */
function stored<Type extends keyof StoredTypes>(value: unknown, type: Type): StoredTypes[Type] {
    // refuse a value of another type than the column stores
    if (!isStored(value, type)) {
        throw new TypeError(`the database driver returned ${typeof value} for a ${type} column`);
    }

    return value;
}

/** Decide whether a driver value has the type a column stores. */
function isStored<Type extends keyof StoredTypes>(
    value: unknown,
    type: Type,
): value is StoredTypes[Type] {
    return type === "bytes" ? value instanceof Uint8Array : typeof value === type;
}
