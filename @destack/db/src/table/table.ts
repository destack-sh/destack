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
import {
    mapValues,
    present,
    schema,
    Version,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
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
import { recordShape, type Shape } from "./schema.ts";

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
            throw new TypeError(`${description.name} has key columns out of declaration order`);
        }

        // build each column by kind
        const columns = Object.fromEntries(
            description.columns.map((column) => {
                const isUnlogged = state.log !== undefined && !logged.has(column.name);
                // leave an unlogged column's sealed text opaque, as a copy never opens it
                const definition = {
                    ...COLUMNS[isUnlogged ? "text" : column.kind](column.name).definition,
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
                isShared: false,
                moved: {},
                convert: {},
                aggregates: [],
                dependents: [],
                predefined: [],
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

    /** Complete an inserted row in memory with its defaults, requiring every column. */
    static complete<Definition extends Table>(
        table: Definition,
        row: Row,
    ): Select<Definition> & Row {
        // fill the defaults and require every column
        const definition = table[TABLE];
        const completed = definition.defaults(row);
        const missing = definition.entries.find(
            ([property]) => !Object.hasOwn(completed, property),
        );
        if (missing !== undefined) {
            const [property, column] = missing;
            const isComputed =
                column.definition.generated !== undefined ||
                column.definition.default instanceof SQL;
            throw new TypeError(
                isComputed
                    ? `column ${property} of ${definition.name} takes a value only the database computes`
                    : `column ${property} of ${definition.name} needs a value`,
            );
        }

        return mapValues<Row, Select<Definition> & Row>(completed, (value) => value);
    }

    /** Type a row as a table's logged record, requiring each logged column, allowing the rest. */
    static image<Definition extends Table>(
        table: Definition,
        row: Row,
    ): RowImage<Definition> & Row {
        // require every logged column
        const definition = table[TABLE];
        const missing = Object.keys(definition.logged).find(
            (property) => !Object.hasOwn(row, property),
        );
        if (missing !== undefined) {
            throw new TypeError(`logged column ${missing} of ${definition.name} has no value`);
        }

        return mapValues<Row, RowImage<Definition> & Row>(row, (value) => value);
    }

    /** Take a row of a table's column values, built at runtime, failing for other properties. */
    static values<Definition extends Table>(
        table: Definition,
        row: Row,
    ): Partial<Select<Definition>> & Row {
        // refuse other properties and null in a column without nulls
        for (const [property, value] of Object.entries(row)) {
            if (value === null && !table[TABLE].column(property).definition.nullable) {
                throw new TypeError(`column ${property} of ${table[TABLE].name} is not nullable`);
            }
        }

        return mapValues<Row, Partial<Select<Definition>> & Row>(row, (value) => value);
    }

    /** Read the columns a row has from JSON form, an undefined value being absent as in JSON. */
    static decode<Definition extends Table>(
        table: Definition,
        row: JsonObject,
    ): Partial<Select<Definition>> & Row {
        const decoded: Row = Object.fromEntries(
            table[TABLE].entries.flatMap(([property, column]) => {
                const value = Object.hasOwn(row, property) ? row[property] : undefined;

                return value === undefined
                    ? []
                    : [[property, value === null ? null : column.definition.fromJson(value)]];
            }),
        );

        return mapValues<Row, Partial<Select<Definition>> & Row>(decoded, (value) => value);
    }

    /** Validate a table's selected record. */
    static selectSchema<Definition extends Table>(
        table: Definition,
    ): schema.Object<Shape<Select<Definition>>> {
        return schema.object(
            mapValues<Record<string, schema.Schema>, Shape<Select<Definition>>>(
                recordShape(table[TABLE], "select"),
                (field) => field,
            ),
        );
    }

    /** Validate a table's inserted record. */
    static insertSchema<Definition extends Table>(
        table: Definition,
    ): schema.Object<Shape<Insert<Definition>>> {
        return schema.object(
            mapValues<Record<string, schema.Schema>, Shape<Insert<Definition>>>(
                recordShape(table[TABLE], "insert"),
                (field) => field,
            ),
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
    /** How long the log retains the table's changes. */
    readonly retention: ChangeRetention;
    /** Whether rows are only ever inserted, deletes going unlogged. */
    readonly isAppendOnly: boolean;
    /** Whether transactions share cached reads of the table, each refreshed by the table's log. */
    readonly isShared: boolean;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by the release introducing them. */
    readonly convert: Readonly<Record<Version, RowConversion>>;
    /** The aggregates this table's rows feed or store. */
    readonly aggregates: readonly Aggregate[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
    /** The predefined rows every database has, identical everywhere. */
    readonly predefined: readonly Row[];
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
        // store the identity, columns and declaration
        this.package = identity.package;
        this.name = identity.name;
        this.sqlName = identity.sqlName;
        this.columns = columns;
        this.retention = declaration.retention;
        this.isAppendOnly = declaration.isAppendOnly;
        this.isShared = declaration.isShared;
        this.moved = declaration.moved;
        this.convert = declaration.convert;
        this.aggregates = declaration.aggregates;
        this.dependents = declaration.dependents;
        this.predefined = declaration.predefined;
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
                        throw new TypeError(
                            `unique index ${constraint.name} cannot key an expression`,
                        );
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
        // return a row nothing conceals as it is
        if (concealed.length === 0) {
            return row;
        }

        return Object.fromEntries(
            Object.entries(row).flatMap(([property, value]) => {
                // show a shown column, mask a personal one and leave out the rest
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
            throw new DatabaseError("INVALID_QUERY", `${this.name} has no text to search`);
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
            // retain primary key checks
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

    /** Select a row's logged columns, as log readers see it. */
    image(row: Row): Row {
        // pass the row on when the table logs every column
        const logged = this.logged;
        if (Object.keys(logged).length === this.entries.length) {
            return row;
        }

        return Object.fromEntries(
            Object.entries(row).filter(([property]) => Object.hasOwn(logged, property)),
        );
    }

    /** Write a row's logged columns as JSON without concealed ones, as log readers see it. */
    encodeLogged(row: Row, concealed: readonly string[]): Record<string, JsonValue> {
        return encodeRow(Object.entries(this.logged), this.conceal(row, concealed));
    }

    /** Fill a row's omitted columns in memory: each default, else null in a nullable column. */
    defaults(row: Row): Row {
        const filled: [string, ColumnValue][] = [];
        for (const [property, { definition }] of this.entries) {
            // take a given value and leave out a computed one
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
    /** How long the log retains the table's changes. */
    readonly retention: ChangeRetention;
    /** Whether rows are only ever inserted, deletes going unlogged. */
    readonly isAppendOnly: boolean;
    /** Whether transactions share cached reads of the table, each refreshed by the table's log. */
    readonly isShared: boolean;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by the release introducing them. */
    readonly convert: Readonly<Record<Version, RowConversion>>;
    /** The aggregates this table's rows feed or store. */
    readonly aggregates: readonly Aggregate[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
    /** The predefined rows every database has, identical everywhere. */
    readonly predefined: readonly Row[];
    /** The table's full-text index, absent for a table nothing searches. */
    readonly fullText: FullTextIndex | undefined;
}

/** A count, sum or extreme of one table's rows, stored on the rows they reference. */
export type Aggregate = AggregateOptions &
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

/** The options of an aggregate: a count, or a sum or extreme of a property. */
type AggregateOptions = {
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

/** How a table logs its committed changes: an append-only table is never shared. */
export type LogOptions = {
    /** How long the log retains the changes, the window by default. */
    readonly retention?: Exclude<ChangeRetention, "none">;
} & (
    | {
          /** Whether rows are only inserted, as events are. */
          readonly appendOnly?: false;
          /** Whether transactions share cached reads of the table, refreshed by its log. */
          readonly isShared?: boolean;
      }
    | {
          /** Rows are only inserted, as events are. */
          readonly appendOnly: true;
          /** The table is never shared, as its deletes go unlogged. */
          readonly isShared?: false;
      }
);

/** The options of a table. */
export interface TableOptions<Columns> {
    /** The constraints and indexes. */
    readonly constraints?: (columns: Columns) => readonly TableConstraint[];
    /** Log committed changes under the scope column, or the database's scope without one. */
    readonly log?: LogOptions;
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
    /** The aggregates this table's rows feed or store. */
    readonly aggregates?: readonly Aggregate[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents?: readonly Dependent[];
    /** The predefined rows every database has, identical everywhere and never written. */
    readonly predefined?: readonly PredefinedRow<Columns>[];
    /** The text properties a full-text index covers, in a logged table. */
    readonly fullText?: readonly (keyof Columns & string)[];
}

/** A predefined row: the value of every column but generated ones, by property. */
export type PredefinedRow<Columns> = {
    readonly [
        Property in keyof Columns as Columns[Property] extends Column<infer Definition>
            ? Definition extends { readonly generated: unknown }
                ? never
                : Property
            : never
    ]: Columns[Property] extends Column<infer Definition> ? ValueIn<Definition> : never;
};

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
            isShared: options.log?.isShared === true,
            moved: options.moved ?? {},
            convert: options.convert ?? {},
            aggregates: options.aggregates ?? [],
            dependents: options.dependents ?? [],
            predefined: options.predefined ?? [],
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
            isShared: source[TABLE].isShared,
            moved: source[TABLE].moved,
            convert: source[TABLE].convert,
            aggregates: source[TABLE].aggregates,
            dependents: source[TABLE].dependents,
            predefined: source[TABLE].predefined,
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

/** Attach builders' columns to a table's SQL name, in property order. */
function attachColumns<Builders extends ColumnBuilderMap, Name extends string>(
    sqlName: string,
    builders: Builders,
): TableColumnMap<Builders, Name> {
    return mapValues<Builders, TableColumnMap<Builders, Name>>(
        builders,
        (builder) => new Column(sqlName, builder.definition),
    );
}

/** Qualify a table's columns by an alias, in property order. */
function aliasColumns<Definition extends Table, Name extends string>(
    source: Definition,
    name: Name,
): AliasColumnMap<Definition, Name> {
    return mapValues<ColumnMap, AliasColumnMap<Definition, Name>>(
        source[TABLE].columns,
        (column) => new Column(name, column.definition, true),
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
        throw new TypeError(`table ${name} searches text, so its changes must be logged`);
    }

    // require text columns
    const untexted = search.find((property) => columns[property]?.definition.kind !== "text");
    if (untexted !== undefined) {
        throw new TypeError(`table ${name} searches ${untexted}, which is not text`);
    }

    return search.length === 0
        ? undefined
        : new FullTextIndex(
              search,
              search.map((property) => present(columns[property], property).definition.name),
              sqlName,
          );
}
