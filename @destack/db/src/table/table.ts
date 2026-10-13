import { SQL, sql, type SQLWrapper } from "../sql/index.ts";
import type { Statement } from "../query/statement.ts";
import {
    bigint,
    binary,
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
import { check, ForeignKey, Index, Unique, type TableConstraint } from "./constraint.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { ModuleMetadata, PACKAGE, type Package } from "@destack/package";
import { present, schema, Version, type JsonObject, type JsonValue } from "@destack/schema";
import { Expression } from "../expression/expression.ts";
import type { Condition, Scalar } from "../query/condition.ts";
import { qualify } from "./namespace.ts";
import type { ChangeRetention } from "../log/description.ts";
import { Tree } from "../tree/tree.ts";
import { FullTextIndex } from "../search/search.ts";
import { DatabaseError } from "../error/error.ts";
import type { ColumnDescription, TableDescription } from "./description.ts";
import type { TableState } from "../migration/state.ts";
import type { Row } from "./row.ts";
import { recordSchema, type JsonShape, type Shape } from "./schema.ts";

/** The schema of a masked column's JSON value. */
const JSON_VALUE = schema.json();

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

    /** Build the table a state describes, such as another host's. */
    static describe(state: TableState, properties: Readonly<Record<string, string>> = {}): Table {
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

                return [
                    properties[column.name] ?? column.name,
                    new Column(description.name, definition),
                ];
            }),
        );

        // log the table
        return new Table(
            { package: state.package, name: description.name, sqlName: description.name },
            columns,
            {
                constraints: () => [],
                retention: state.log?.retention ?? "none",
                isAppendOnly: state.log?.appendOnly === true,
                moved: {},
                convert: {},
                aggregates: [],
                dependents: [],
                fullText:
                    state.fullText === undefined
                        ? undefined
                        : new FullTextIndex(
                              state.fullText.map((column) => properties[column] ?? column),
                              state.fullText,
                              description.name,
                          ),
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
    /** Whether rows are only ever inserted, deletes going unlogged. */
    readonly isAppendOnly: boolean;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by the release introducing them. */
    readonly convert: Readonly<Record<Version, RowConversion>>;
    /** The aggregates this table's rows feed or keep. */
    readonly aggregates: readonly Summary[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
    /** The table's full-text index, absent for a table nothing searches. */
    readonly fullText: FullTextIndex | undefined;
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
    /** The unique keys, read on first use. */
    #uniqueKeys: readonly UniqueKey[] | undefined;
    /** The columns in declaration order, read on first use. */
    #entries: readonly (readonly [string, Column])[] | undefined;
    /** The logged columns, read on first use. */
    #logged: Readonly<Record<string, Column>> | undefined;
    /** The columns whose schemas mark personal values, read on first use. */
    #personal: ReadonlySet<string> | undefined;
    /** The built statements by name. */
    readonly #statements = new Map<string, Statement>();
    /** The table's descriptions by dialect, described on first use. */
    readonly descriptions = new Map<Dialect, TableDescription>();

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
        this.isAppendOnly = declaration.isAppendOnly;
        this.moved = declaration.moved;
        this.convert = declaration.convert;
        this.aggregates = declaration.aggregates;
        this.dependents = declaration.dependents;
        this.fullText = declaration.fullText;
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

    /** The column sets no two rows share values of, among rows meeting each set's condition. */
    get uniqueKeys(): readonly UniqueKey[] {
        this.#uniqueKeys ??= this.#gatherUniqueKeys();

        return this.#uniqueKeys;
    }

    /** Gather the unique keys, each column by its property. */
    #gatherUniqueKeys(): UniqueKey[] {
        // map each column to its property
        const properties = new Map(this.entries.map(([property, column]) => [column, property]));
        const named = (columns: readonly Column[]) =>
            columns.map((column) =>
                present(properties.get(column), `the property of ${column.definition.name}`),
            );

        // gather the declared keys over plain columns
        const declared = this.#declared().flatMap((constraint): UniqueKey[] => {
            if (constraint instanceof Unique) {
                return [
                    {
                        columns: named(constraint.columns),
                        where: undefined,
                        isNullsNotDistinct: false,
                    },
                ];
            } else if (constraint instanceof Index && constraint.isUnique) {
                const columns = constraint.columns.map((column) => {
                    if (column instanceof SQL) {
                        throw new TypeError(`unique index ${constraint.name} keys an expression`);
                    }

                    return column;
                });

                return [
                    {
                        columns: named(columns),
                        where: constraint.predicate,
                        isNullsNotDistinct: constraint.isNullsNotDistinct,
                    },
                ];
            } else {
                return [];
            }
        });

        // add the primary key and each unique column
        const unique = this.entries.filter(([, column]) => column.definition.unique !== undefined);

        return [
            { columns: this.key, where: undefined, isNullsNotDistinct: false },
            ...unique.map(([property]) => ({
                columns: [property],
                where: undefined,
                isNullsNotDistinct: false,
            })),
            ...declared,
        ];
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

    /** The columns whose schemas mark personal values, which concealment masks. */
    get personal(): ReadonlySet<string> {
        this.#personal ??= new Set(
            this.entries
                .filter(([, column]) => schema.marksPersonal(column.definition.schema))
                .map(([property]) => property),
        );

        return this.#personal;
    }

    /** Show a row as readers lacking its concealed columns see it, personal values masked. */
    conceal(row: Row, concealed: readonly string[]): Row {
        // keep a row nothing conceals as it is
        if (concealed.length === 0) {
            return row;
        }

        return Object.fromEntries(
            Object.entries(row).flatMap(([property, value]) => {
                // keep a shown column, mask a personal one, and leave out the rest
                if (!concealed.includes(property)) {
                    return [[property, value]];
                } else if (this.personal.has(property)) {
                    return [[property, maskOf(this.column(property), value)]];
                }

                return [];
            }),
        );
    }

    /** Read the table's full-text index, refusing a table nothing searches. */
    requireFullText(): FullTextIndex {
        if (this.fullText === undefined) {
            throw new DatabaseError("INVALID_QUERY", `${this.name} keeps no text to search`);
        }

        return this.fullText;
    }

    /** Read a column by property, failing for a property the table does not declare. */
    column(property: string): Column {
        const column = this.columns[property];
        if (column === undefined) {
            throw new TypeError(`${this.name} has no column ${property}`);
        }

        return column;
    }

    /** Collect a dialect's constraints: declared ones, SQLite key checks and column references. */
    constraints(dialect: Dialect): readonly TableConstraint[] {
        const constraints = [...this.#declared()];
        for (const [property, column] of Object.entries(this.columns)) {
            // keep primary key checks
            const definition = column.definition;
            if (
                dialect === "sqlite" &&
                definition.primaryKey === true &&
                definition.types.sqlite !== "integer"
            ) {
                constraints.push(
                    check(`${this.name}_${definition.name}_not_null`, {
                        [property]: { isNotNull: true },
                    }),
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
        return encodeRow(this.entries, this.conceal(row, concealed));
    }

    /** Write a row's logged columns as JSON without concealed ones, as log readers see it. */
    encodeLogged(row: Row, concealed: readonly string[]): Record<string, JsonValue> {
        return encodeRow(Object.entries(this.logged), this.conceal(row, concealed));
    }

    /** Fill a row's omitted columns in memory: each default, else null in a nullable column. */
    defaults(row: Row): Row {
        const filled: [string, ColumnValue][] = [];
        for (const [property, { definition }] of this.entries) {
            // keep a given value and leave out a computed one
            if (
                Object.hasOwn(row, property) ||
                definition.generated !== undefined ||
                definition.default instanceof SQL
            ) {
                continue;
            }
            // take the default
            else if (definition.default !== undefined) {
                filled.push([property, definition.default]);
            }
            // take null in a nullable column
            else if (definition.nullable) {
                filled.push([property, null]);
            }
        }

        return { ...row, ...Object.fromEntries(filled) };
    }

    /** Complete an inserted row in memory with its defaults. */
    complete(row: Row): Select<Table<Name, Columns>> & Row;
    /**
     * Complete a row with its defaults.
     *
     * @construct every column has a value of its type once completed, as the signature above reads.
     */
    complete(row: Row): Row {
        // fill the defaults and require every column
        const completed = this.defaults(row);
        const missing = this.entries.find(([property]) => !Object.hasOwn(completed, property));
        if (missing === undefined) {
            return completed;
        }

        // refuse the first missing column
        const [property, { definition }] = missing;
        const isComputed = definition.generated !== undefined || definition.default instanceof SQL;
        throw new TypeError(
            isComputed
                ? `column ${property} of ${this.name} takes a value only the database computes`
                : `column ${property} of ${this.name} needs a value`,
        );
    }

    /** Take a row of this table's column values, built at runtime, failing for other properties. */
    values(row: Row): Partial<Select<Table<Name, Columns>>> & Row;
    /**
     * Return the row once each property is a table column, null only where nullable.
     *
     * @construct each property is a column, and its encoder parses the value when written.
     */
    values(row: Row): Row {
        // refuse other properties and null in a column without nulls
        for (const [property, value] of Object.entries(row)) {
            if (value === null && !this.column(property).definition.nullable) {
                throw new TypeError(`column ${property} of ${this.name} is not nullable`);
            }
        }

        return row;
    }

    /** Read the columns a row has from JSON form. */
    decode(row: JsonObject): Partial<Select<Table<Name, Columns>>> & Row;
    /**
     * Decode each column the row has by its column definition.
     *
     * @construct each row property is a column, decoded into its type by the column's definition.
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
     * Validate a selected record, typed by the table in the signatures above.
     *
     * @construct the schema has each column's schema in the form, and JSON schemas read JsonOf.
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
     * Validate an inserted record, typed by the table in the signatures above.
     *
     * @construct the schema has each column's insert schema in the form, JSON ones read JsonOf.
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
    /** Whether rows are only ever inserted, deletes going unlogged. */
    readonly isAppendOnly: boolean;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by the release introducing them. */
    readonly convert: Readonly<Record<Version, RowConversion>>;
    /** The aggregates this table's rows feed or keep. */
    readonly aggregates: readonly Summary[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
    /** The table's full-text index, absent for a table nothing searches. */
    readonly fullText: FullTextIndex | undefined;
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
    /** Log committed changes under the scope column, or the database's scope without one. */
    readonly log?: {
        /** How long the log keeps the changes, the window by default. */
        readonly retention?: Exclude<ChangeRetention, "none">;
        /** Whether rows are only inserted, as events are. */
        readonly appendOnly?: boolean;
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
    /** The text properties a full-text index keeps, in a logged table. */
    readonly fullText?: readonly (keyof Columns & string)[];
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
            isAppendOnly: options.log?.appendOnly === true,
            moved: options.moved ?? {},
            convert: options.convert ?? {},
            aggregates: options.aggregates ?? [],
            dependents: options.dependents ?? [],
            fullText: fullTextOf(name, sqlName, columns, options),
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
            isAppendOnly: source[TABLE].isAppendOnly,
            moved: source[TABLE].moved,
            convert: source[TABLE].convert,
            aggregates: source[TABLE].aggregates,
            dependents: source[TABLE].dependents,
            fullText: source[TABLE].fullText,
        },
        { source },
    );

    return Object.assign(definition, columns);
}

/** Columns whose values no two rows share, among the rows meeting a condition. */
export interface UniqueKey {
    /** The key columns' properties. */
    readonly columns: readonly string[];
    /** The condition selecting the rows the key covers, all rows when absent. */
    readonly where: Condition | undefined;
    /** Whether rows with nulls in the same columns share the key. */
    readonly isNullsNotDistinct: boolean;
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
): Record<string, JsonValue> {
    const encoded: Record<string, JsonValue> = {};
    for (const [property, column] of columns) {
        // write the columns the row has
        const value = row[property];
        if (value !== undefined) {
            encoded[property] = value === null ? null : column.definition.toJson(value);
        }
    }

    return encoded;
}

/** Mask the personal values a column's schema marks in its value. */
function maskOf(column: Column, value: ColumnValue): ColumnValue {
    return value === null ? null : schema.mask(column.definition.schema, JSON_VALUE.parse(value));
}

/** Read a table's full-text index, refusing non-text columns and an unlogged table. */
function fullTextOf(
    name: string,
    sqlName: string,
    columns: ColumnMap,
    options: Pick<TableOptions<ColumnMap>, "fullText" | "log">,
): FullTextIndex | undefined {
    // index no text by default
    const search = options.fullText ?? [];
    if (search.length > 0 && options.log === undefined) {
        throw new TypeError(`table ${name} searches text, which needs its changes logged`);
    }

    // require text columns
    const untexted = search.find((property) => columns[property]?.definition.kind !== "text");
    if (untexted !== undefined) {
        throw new TypeError(`table ${name} searches ${untexted}, which keeps no text`);
    }

    return search.length === 0
        ? undefined
        : new FullTextIndex(
              search,
              search.map((property) => present(columns[property], property).definition.name),
              sqlName,
          );
}
