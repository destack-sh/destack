import { SQL, sql, type SQLWrapper } from "../sql/index.ts";
import type { Statement } from "../query/statement.ts";
import {
    bigint,
    binary,
    blob,
    boolean,
    Column,
    integer,
    json,
    numeric,
    real,
    text,
    type ColumnBuilder,
    type ColumnDefinition,
    type ColumnValue,
    type ValueOf,
} from "./column.ts";
import { check, ForeignKey, type TableConstraint } from "./constraint.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { ModuleMetadata, PACKAGE, type Package } from "@destack/package";
import { schema, Version, type JsonObject, type JsonValue } from "@destack/schema";
import { Expression } from "../expression/expression.ts";
import type { Scalar } from "../query/condition.ts";
import { qualify } from "./namespace.ts";
import type { ChangeRetention } from "../log/description.ts";
import { Tree } from "../tree/tree.ts";
import type { ColumnDescription } from "./description.ts";
import type { TableState } from "../migration/state.ts";
import type { Row } from "./row.ts";
import { recordSchema, type JsonShape, type Shape } from "./schema.ts";

/** The key of a table's declaration, shared by every copy of this module. */
export const TABLE = Symbol.for("destack.table");

/** The column builder of each described kind. */
const COLUMNS: Readonly<Record<ColumnDescription["kind"], (name: string) => ColumnBuilder>> = {
    text: (name) => text(name),
    integer,
    real,
    boolean,
    json: (name) => json(name, schema.json()),
    binary,
    blob,
    bigint,
    numeric,
};

/** One logical SQL table: its columns as properties, and its definition under {@link TABLE}. */
export class Table<
    Name extends string = string,
    Columns extends ColumnMap = ColumnMap,
> implements SQLWrapper {
    /** The table's identity, columns and declaration. */
    readonly [TABLE]: TableDefinition<Name, Columns>;
    /** The selected application record type. */
    declare readonly $inferSelect: Select<Table<Name, Columns>>;
    /** The inserted application record type. */
    declare readonly $inferInsert: Insert<Table<Name, Columns>>;

    /** Create the table. */
    constructor(
        identity: { readonly package: Package; readonly name: Name; readonly sqlName: string },
        columns: Columns,
        declaration: TableDeclaration,
        options: { readonly tree?: TreeColumns; readonly source?: Table } = {},
    ) {
        this[TABLE] = new TableDefinition(this, identity, columns, declaration, options);
    }

    /** Build the table a state describes, such as a database another host declared, marking its unlogged columns sensitive. */
    static describe(state: TableState): Table {
        // read the logged columns
        const description = state.table;
        const logged = new Set(state.log?.columns ?? []);

        // read the key, which a table declares in column order
        const key = description.constraints.flatMap((constraint) =>
            constraint.kind === "primaryKey" ? constraint.columns : [],
        );
        const keyed = new Set(key);
        const ordered = description.columns.filter((column) => keyed.has(column.name));
        if (ordered.map((column) => column.name).join() !== key.join()) {
            throw new TypeError(`${description.name} keys its columns out of declaration order`);
        }

        // build each column by kind
        const columns = Object.fromEntries(
            description.columns.map((column) => {
                const isUnlogged = state.log !== undefined && !logged.has(column.name);
                const definition = {
                    ...COLUMNS[column.kind](column.name).definition,
                    nullable: column.nullable,
                    ...(keyed.has(column.name) ? { primaryKey: true } : {}),
                    ...(isUnlogged ? { classification: "sensitive" as const } : {}),
                    ...(column.generated === undefined
                        ? {}
                        : {
                              generated: {
                                  expression: sql.raw(column.generated.expression),
                                  mode: column.generated.mode ?? "virtual",
                              },
                          }),
                };

                return [column.name, new Column(description.name, definition)];
            }),
        );

        // log the table
        return new Table(
            { package: state.package, name: description.name, sqlName: description.name },
            columns,
            {
                constraints: () => [],
                retention: state.log?.retention ?? "none",
                moved: {},
                convert: {},
                aggregates: [],
                dependents: [],
            },
        );
    }

    /** The declaring package, under the key every declaration kind shares. */
    get [PACKAGE](): Package {
        return this[TABLE].package;
    }

    /** Embed the table's name. */
    getSQL(): SQL {
        return new SQL([this]);
    }
}

/** A table's identity, columns and declaration, with what derives from them. */
export class TableDefinition<Name extends string = string, Columns extends ColumnMap = ColumnMap> {
    /** The declaring package. */
    readonly package: Package;
    /** The table name within its package. */
    readonly name: Name;
    /** The SQL identifier. */
    readonly sqlName: string;
    /** The columns by property. */
    readonly columns: Columns;
    /** How long the log keeps the table's changes. */
    readonly retention: ChangeRetention;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by the release introducing them. */
    readonly convert: Readonly<Record<Version, RowConversion>>;
    /** The aggregates this table's rows feed or keep. */
    readonly aggregates: readonly Summary[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
    /** The source table of a query alias. */
    readonly source: Table | undefined;
    /** The table this defines. */
    readonly #table: Table;
    /** Evaluate the declared constraints once every table exists. */
    readonly #declared: () => readonly TableConstraint[];
    /** The tree columns, absent without a tree. */
    readonly #treeColumns: TreeColumns | undefined;
    /** The tree, built on first read. */
    #tree: Tree | undefined;
    /** The primary key properties, read on first use. */
    #key: readonly string[] | undefined;
    /** The columns in declaration order, read on first use. */
    #entries: readonly (readonly [string, Column])[] | undefined;
    /** The logged columns, read on first use. */
    #logged: Readonly<Record<string, Column>> | undefined;
    /** The built statements by name. */
    readonly #statements = new Map<string, Statement>();

    /** Define a table. */
    constructor(
        table: Table,
        identity: { readonly package: Package; readonly name: Name; readonly sqlName: string },
        columns: Columns,
        declaration: TableDeclaration,
        options: { readonly tree?: TreeColumns; readonly source?: Table },
    ) {
        // keep the identity, columns and declaration
        this.package = identity.package;
        this.name = identity.name;
        this.sqlName = identity.sqlName;
        this.columns = columns;
        this.retention = declaration.retention;
        this.moved = declaration.moved;
        this.convert = declaration.convert;
        this.aggregates = declaration.aggregates;
        this.dependents = declaration.dependents;
        this.source = options.source;
        this.#table = table;
        this.#declared = declaration.constraints;
        this.#treeColumns = options.tree;
    }

    /** The ancestor index over the parent column, absent without a tree. */
    get tree(): Tree | undefined {
        if (this.#treeColumns !== undefined) {
            this.#tree ??= new Tree({ name: "tree", table: this.#table, ...this.#treeColumns });
        }

        return this.#tree;
    }

    /** The primary key properties in key order. */
    get key(): readonly string[] {
        this.#key ??= this.source?.[TABLE].key ?? keyOf(this.#table);

        return this.#key;
    }

    /** The columns in declaration order. */
    get entries(): readonly (readonly [string, Column])[] {
        this.#entries ??= Object.entries(this.columns);

        return this.#entries;
    }

    /** The logged columns: every column but sensitive ones. */
    get logged(): Readonly<Record<string, Column>> {
        this.#logged ??= Object.fromEntries(
            this.entries.filter(([, column]) => column.definition.classification !== "sensitive"),
        );

        return this.#logged;
    }

    /** Read a column by property, failing for a property the table does not declare. */
    column(property: string): Column {
        const column = this.columns[property];
        if (column === undefined) {
            throw new TypeError(`${this.name} has no column ${property}`);
        }

        return column;
    }

    /** Collect the constraints for a dialect: the declared ones, SQLite's key checks and column references. */
    constraints(dialect: Dialect): readonly TableConstraint[] {
        const constraints = [...this.#declared()];
        for (const column of Object.values(this.columns)) {
            // keep primary key checks
            const definition = column.definition;
            if (
                dialect === "sqlite" &&
                definition.primaryKey === true &&
                definition.types.sqlite !== "integer"
            ) {
                constraints.push(
                    check(`${this.name}_${definition.name}_not_null`, sql`${column} IS NOT NULL`),
                );
            }

            // expand references once every table is declared
            const reference = definition.reference;
            if (reference !== undefined) {
                constraints.push(
                    new ForeignKey(
                        { columns: [column], foreignColumns: [reference.column()] },
                        reference,
                    ),
                );
            }
        }

        return constraints;
    }

    /** Write the columns a row has in JSON form, leaving out concealed ones. */
    encode(row: Row, concealed: readonly string[] = []): Record<string, JsonValue> {
        return encodeRow(this.entries, row, concealed);
    }

    /** Write the logged columns a row has in JSON form, leaving out concealed ones, as readers of the log see it. */
    encodeLogged(row: Row, concealed: readonly string[]): Record<string, JsonValue> {
        return encodeRow(Object.entries(this.logged), row, concealed);
    }

    /** Take a row of this table's column values, as code choosing columns at runtime builds it, failing for other properties. */
    values(row: Row): Partial<Select<Table<Name, Columns>>> & Row;
    /**
     * Return the row once each of its properties is one of the table's columns, holding a value of the column's type.
     *
     * @construct every property names a column and its value parses by that column's schema, or is null in a nullable column.
     */
    values(row: Row): Row {
        // require every property to be a column holding a value of its type
        for (const [property, value] of Object.entries(row)) {
            // refuse null in a column without nulls
            const { definition } = this.column(property);
            if (value === null && !definition.nullable) {
                throw new TypeError(`column ${property} of ${this.name} is not nullable`);
            }
            // parse a present value by the column's schema
            else if (value !== null) {
                definition.schema.parse(value);
            }
        }

        return row;
    }

    /** Read the columns a row has from JSON form. */
    decode(row: JsonObject): Partial<Select<Table<Name, Columns>>> & Row;
    /**
     * Decode each column the row has by its column definition.
     *
     * @construct each property the row has is a column, decoded by that column's definition into its type.
     */
    decode(row: JsonObject): Row {
        const decoded: Record<string, ColumnValue> = {};
        for (const [property, column] of this.entries) {
            // read the columns the row has, an undefined value being absent as in JSON
            const value = Object.hasOwn(row, property) ? row[property] : undefined;
            if (value !== undefined) {
                decoded[property] = value === null ? null : column.definition.fromJson(value);
            }
        }

        return decoded;
    }

    /** Validate a selected record in JSON form. */
    selectSchema(form: "json"): schema.Object<JsonShape<Select<Table<Name, Columns>>>>;
    /** Validate a selected record in application form. */
    selectSchema(form?: "application"): schema.Object<Shape<Select<Table<Name, Columns>>>>;
    /**
     * Validate a selected record, whose signatures above type it by the table.
     *
     * @construct the record schema has one entry per column, each the column's schema in the form, and a JSON schema reads JsonOf the column value.
     */
    selectSchema(
        form: "application" | "json" = "application",
    ): schema.Object<Record<string, schema.Schema>> {
        return recordSchema(this, "select", form);
    }

    /** Validate an inserted record in JSON form. */
    insertSchema(form: "json"): schema.Object<JsonShape<Insert<Table<Name, Columns>>>>;
    /** Validate an inserted record in application form. */
    insertSchema(form?: "application"): schema.Object<Shape<Insert<Table<Name, Columns>>>>;
    /**
     * Validate an inserted record, whose signatures above type it by the table.
     *
     * @construct the record schema has one entry per column, each the column's insert schema in the form, and a JSON schema reads JsonOf the column value.
     */
    insertSchema(
        form: "application" | "json" = "application",
    ): schema.Object<Record<string, schema.Schema>> {
        return recordSchema(this, "insert", form);
    }

    /** Validate a partial update. */
    updateSchema(): schema.Object<Shape<Partial<Insert<Table<Name, Columns>>>>>;
    /**
     * Validate a partial update, whose signature above types it by the table.
     *
     * @construct the record schema has one optional entry per column, each the column's schema.
     */
    updateSchema(): schema.Object<Record<string, schema.Schema>> {
        return recordSchema(this, "update");
    }

    /** Read a statement over the table, building it once per name. */
    statement(name: string, build: () => Statement): Statement {
        // reuse the built statement
        const known = this.#statements.get(name);
        if (known !== undefined) {
            return known;
        }
        const built = build();
        this.#statements.set(name, built);

        return built;
    }
}

/** New column values computed from a row's previous values, by property. */
export type RowConversion<Columns = ColumnMap> = {
    readonly [Property in keyof Columns]?: Expression;
};

/** The previous names of a table and its columns. */
export interface TableMove {
    /** The table's previous name within its package. */
    readonly table?: string;
    /** The previous SQL column names by property. */
    readonly columns?: Readonly<Partial<Record<string, string>>>;
}

/** The properties of a single-parent tree per scope. */
interface TreeColumns<Property extends string = string> {
    /** The property with the node identity. */
    readonly id: Property;
    /** The property with the tree scope. */
    readonly scope: Property;
    /** The property with the nullable parent identity. */
    readonly parent: Property;
}

/** A table's declaration with defaults applied. */
interface TableDeclaration {
    /** Evaluate the constraints. */
    readonly constraints: () => readonly TableConstraint[];
    /** How long the log keeps the table's changes. */
    readonly retention: ChangeRetention;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by the release introducing them. */
    readonly convert: Readonly<Record<Version, RowConversion>>;
    /** The aggregates this table's rows feed or keep. */
    readonly aggregates: readonly Summary[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
}

/** A count, sum or extreme of one table's rows kept on the rows they reference. */
export type Summary = SummaryOptions &
    (
        | {
              /** The target table. */
              readonly into: () => Table;
          }
        | {
              /** The aggregated table. */
              readonly from: () => Table;
          }
    );

/** The options of a summary: a count, or a sum or extreme of a property. */
type SummaryOptions = {
    /** The target property. */
    readonly column: string;
    /** The aggregated table's property referencing the target rows. */
    readonly key: string;
    /** The values of the aggregated rows. */
    readonly where?: Readonly<Record<string, Scalar>>;
} & (
    | {
          /** Count the rows. */
          readonly function: "count";
      }
    | {
          /** Sum the values, or take their least or greatest. */
          readonly function: "sum" | "min" | "max";
          /** The aggregated property. */
          readonly value: string;
      }
);

/** Rows of another table referencing this table's rows, as a polymorphic reference does. */
export interface Dependent {
    /** The table with the dependent rows. */
    readonly from: () => Table;
    /** The dependent table's property referencing this table's rows. */
    readonly key: string;
    /** The values of the dependent rows, such as the referenced type. */
    readonly where?: Readonly<Record<string, Scalar>>;
    /** Cascade or restrict the deletion. */
    readonly onDelete: "cascade" | "restrict";
}

/** The options of a table. */
export interface TableOptions<Columns> {
    /** The constraints and indexes. */
    readonly constraints?: (columns: Columns) => readonly TableConstraint[];
    /** Log committed changes under the scope column, or the database's scope for a table without one. */
    readonly log?: {
        /** How long the log keeps the changes, the window by default. */
        readonly retention?: Exclude<ChangeRetention, "none">;
    };
    /** The properties of a single-parent tree per scope. */
    readonly tree?: TreeColumns<keyof Columns & string>;
    /** The previous names of the table and its columns. */
    readonly moved?: {
        /** The table's previous name within its package. */
        readonly table?: string;
        /** The previous SQL column names by property. */
        readonly columns?: { readonly [Property in keyof Columns]?: string };
    };
    /** The row conversions by the release introducing them, converting rows of earlier releases. */
    readonly convert?: Readonly<Record<Version, RowConversion<Columns>>>;
    /** The aggregates this table's rows feed or keep. */
    readonly aggregates?: readonly Summary[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents?: readonly Dependent[];
}

/** The columns by property. */
export type ColumnMap = Record<string, Column>;

/** The column builders by property. */
export type ColumnBuilderMap = Record<string, ColumnBuilder>;

/** Attach each column's definition and table. */
export type TableColumnMap<Builders extends ColumnBuilderMap, Name extends string = string> = {
    [Property in keyof Builders]: Column<Builders[Property]["definition"], Name>;
};

/** A table's column definitions by property. */
type Definitions<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["definition"];
};

/** The value a column reads, null where the column is nullable. */
type ValueIn<Definition extends ColumnDefinition> = Definition["nullable"] extends false
    ? ValueOf<Definition>
    : ValueOf<Definition> | null;

/** The selected application record. */
export type Select<Definition extends Table> = {
    [Property in keyof Definitions<Definition>]: ValueIn<Definitions<Definition>[Property]>;
};

/** The selected record of the columns the log records: every column except sensitive ones. */
export type RowImage<Definition extends Table> = Definition extends Table
    ? {
          [
              Property in keyof Definitions<Definition> as Definitions<Definition>[Property] extends {
                  readonly classification: "sensitive";
              }
                  ? never
                  : Property
          ]: ValueIn<Definitions<Definition>[Property]>;
      }
    : never;

/** The properties an insert must supply. */
type RequiredColumns<Definition extends Table> = {
    [Property in keyof Definitions<Definition>]: Definitions<Definition>[Property] extends
        | { readonly default: unknown }
        | { readonly generated: unknown }
        ? never
        : Definitions<Definition>[Property]["nullable"] extends false
          ? Property
          : never;
}[keyof Definitions<Definition>];

/** The generated properties. */
type GeneratedColumns<Definition extends Table> = {
    [Property in keyof Definitions<Definition>]: Definitions<Definition>[Property] extends {
        readonly generated: unknown;
    }
        ? Property
        : never;
}[keyof Definitions<Definition>];

/** The values of an insert. */
export type Insert<Definition extends Table> = Pick<
    Select<Definition>,
    RequiredColumns<Definition>
> &
    Partial<Omit<Select<Definition>, RequiredColumns<Definition> | GeneratedColumns<Definition>>>;

/** Declare a table. */
export function defineTable<Name extends string, Builders extends ColumnBuilderMap>(
    name: Name,
    builders: Builders & { [Property in Extract<keyof Table, string>]?: never },
    options: TableOptions<TableColumnMap<Builders, Name>> = {},
    module?: ModuleMetadata,
): Table<Name, TableColumnMap<Builders, Name>> & TableColumnMap<Builders, Name> {
    // qualify the table by its package
    const owner = ModuleMetadata.require(module, "defineTable").package;
    const sqlName = qualify(owner, name);

    // reject generated defaults and duplicate names
    const names = new Set<string>();
    for (const builder of Object.values(builders)) {
        const definition = builder.definition;
        if (definition.generated !== undefined && definition.default !== undefined) {
            throw new TypeError(`generated SQL column cannot define defaults: ${definition.name}`);
        }
        if (names.has(definition.name)) {
            throw new TypeError(`duplicate SQL column: ${definition.name}`);
        }
        names.add(definition.name);
    }

    // require conversions keyed by releases up to the declaring one
    Version.requireUpTo(options.convert ?? {}, owner.version, name);

    // attach the columns in order
    const columns = attachColumns<Builders, Name>(sqlName, builders);
    const definition = new Table(
        { package: owner, name, sqlName },
        columns,
        {
            constraints: () => options.constraints?.(columns) ?? [],
            retention: options.log === undefined ? "none" : (options.log.retention ?? "window"),
            moved: options.moved ?? {},
            convert: options.convert ?? {},
            aggregates: options.aggregates ?? [],
            dependents: options.dependents ?? [],
        },
        options.tree === undefined ? {} : { tree: options.tree },
    );

    return Object.assign(definition, columns);
}

/** The columns of a query alias. */
export type AliasColumnMap<Definition extends Table, Name extends string> = {
    [Property in keyof Definitions<Definition>]: Column<Definitions<Definition>[Property], Name>;
};

/** Reference a table under another query name. */
export function alias<Definition extends Table, Name extends string>(
    source: Definition,
    name: Name,
): Table<Name, AliasColumnMap<Definition, Name>> & AliasColumnMap<Definition, Name> {
    // qualify each column by the alias
    const columns = aliasColumns(source, name);
    const definition = new Table(
        { package: source[TABLE].package, name, sqlName: name },
        columns,
        {
            constraints: () => [],
            retention: source[TABLE].retention,
            moved: source[TABLE].moved,
            convert: source[TABLE].convert,
            aggregates: source[TABLE].aggregates,
            dependents: source[TABLE].dependents,
        },
        { source },
    );

    return Object.assign(definition, columns);
}

/** Read a table's primary key properties: its key columns in declaration order. */
function keyOf(table: Table): readonly string[] {
    return Object.entries(table[TABLE].columns)
        .filter(([, column]) => column.definition.primaryKey === true)
        .map(([property]) => property);
}

/** Attach builders' columns to a table's SQL name, whose signature types them by their builders. */
function attachColumns<Builders extends ColumnBuilderMap, Name extends string>(
    sqlName: string,
    builders: Builders,
): TableColumnMap<Builders, Name>;
/**
 * Attach builders' columns to a table's SQL name, in property order.
 *
 * @construct each builder becomes the column of its property, as the mapped type above describes.
 */
function attachColumns(sqlName: string, builders: ColumnBuilderMap): ColumnMap {
    return Object.fromEntries(
        Object.entries(builders).map(([property, builder]) => [
            property,
            new Column(sqlName, builder.definition),
        ]),
    );
}

/** Qualify a table's columns by an alias, whose signature types them by the table. */
function aliasColumns<Definition extends Table, Name extends string>(
    source: Definition,
    name: Name,
): AliasColumnMap<Definition, Name>;
/**
 * Qualify a table's columns by an alias, in property order.
 *
 * @construct each column keeps its definition under the alias, as the mapped type above describes.
 */
function aliasColumns(source: Table, name: string): ColumnMap {
    return Object.fromEntries(
        Object.entries(source[TABLE].columns).map(([property, column]) => [
            property,
            new Column(name, column.definition, true),
        ]),
    );
}

/** Write the values a row has of some columns in JSON form, leaving out the concealed ones. */
function encodeRow(
    columns: readonly (readonly [string, Column])[],
    row: Row,
    concealed: readonly string[],
): Record<string, JsonValue> {
    const encoded: Record<string, JsonValue> = {};
    for (const [property, column] of columns) {
        // write the unconcealed columns the row has
        const value = row[property];
        if (value !== undefined && !concealed.includes(property)) {
            encoded[property] = value === null ? null : column.definition.toJson(value);
        }
    }

    return encoded;
}
