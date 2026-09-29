import { type SQL, sql, type SQLWrapper } from "drizzle-orm";
import { Column, type ColumnBuilder } from "./column.ts";
import { check, ForeignKey, type PrimaryKey, type TableConstraint } from "./constraint.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { qualify } from "./namespace.ts";
import type { ChangeRetention } from "../inspect/log.ts";
import type { DatabaseTier } from "../declare/database.ts";
import { Tree } from "../tree/tree.ts";

/** The key of a table's declaration, shared by every copy of this module. */
export const TABLE = Symbol.for("destack.table");

/** One logical SQL table. */
export class Table<
    Name extends string = string,
    Columns extends ColumnMap = ColumnMap,
> implements SQLWrapper {
    /** The table's identity, columns and declaration. */
    readonly [TABLE]: TableDeclaration & {
        /** The declaring package. */
        readonly package: Package;
        /** The table name within its package. */
        readonly name: Name;
        /** The SQL identifier. */
        readonly sqlName: string;
        /** The columns by property. */
        readonly columns: Columns;
        /** The ancestor index over the parent column. */
        readonly tree?: Tree;
        /** The source table of a query alias. */
        readonly source?: Table;
        /** The primary key properties in key order. */
        readonly key: readonly string[];
        /** The columns in declaration order. */
        readonly entries: readonly (readonly [string, Column])[];
        /** The logged columns: every column but binary and sensitive ones. */
        readonly logged: Readonly<Record<string, Column>>;
    };
    /** The cached primary key properties. */
    #key: readonly string[] | undefined;
    /** The cached columns in declaration order. */
    #entries: readonly (readonly [string, Column])[] | undefined;
    /** The cached logged columns. */
    #logged: Readonly<Record<string, Column>> | undefined;
    /** The built statements by name. */
    readonly #statements = new Map<string, unknown>();
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
        // declare the table and its key
        const { constraints, retention, tier, version, moved, convert, aggregates, dependents } =
            declaration;
        const declared = {
            ...identity,
            columns,
            constraints,
            retention,
            ...(tier === undefined ? {} : { tier }),
            version,
            moved,
            convert,
            aggregates,
            dependents,
            ...(options.source === undefined ? {} : { source: options.source }),
        };
        Object.defineProperties(declared, {
            key: {
                get: () => (this.#key ??= options.source?.[TABLE].key ?? keyOf(this)),
                enumerable: true,
            },
            entries: {
                get: () => (this.#entries ??= Object.entries(columns)),
                enumerable: true,
            },
            logged: {
                get: () =>
                    (this.#logged ??= Object.fromEntries(
                        Object.entries(columns).filter(
                            ([, column]) =>
                                column.definition.kind !== "binary" &&
                                column.definition.classification !== "sensitive",
                        ),
                    )),
                enumerable: true,
            },
        });
        this[TABLE] = declared as typeof declared & {
            readonly key: readonly string[];
            readonly entries: readonly (readonly [string, Column])[];
            readonly logged: Readonly<Record<string, Column>>;
        };

        // build the tree over the declared columns
        if (options.tree) {
            Object.defineProperty(this[TABLE], "tree", {
                value: new Tree({ name: "tree", table: this, ...options.tree }),
                enumerable: true,
            });
        }
    }

    /** Read a statement over the table, building it once per name. */
    statement<Value>(name: string, build: () => Value): Value {
        // reuse the built statement
        const known = this.#statements.get(name) as Value | undefined;
        if (known !== undefined) {
            return known;
        }

        // build it once
        const built = build();
        this.#statements.set(name, built);

        return built;
    }

    /** Read the package declaring a value that is a table, from any copy of this module. */
    static package(value: unknown): Package | undefined {
        return typeof value === "object" && value !== null && TABLE in value
            ? (value as Table)[TABLE].package
            : undefined;
    }

    /** Return the table identifier. */
    getSQL(): SQL {
        return sql`${sql.identifier(this[TABLE].sqlName)}`;
    }

    /** Emit table identifiers without parentheses. */
    shouldOmitSQLParens(): boolean {
        return true;
    }

    /** Collect the constraints for a dialect. */
    constraints(dialect: Dialect): readonly TableConstraint[] {
        const constraints = [...this[TABLE].constraints()];

        // keep primary key checks
        for (const column of Object.values(this[TABLE].columns)) {
            const definition = column.definition;
            if (
                dialect === "sqlite" &&
                definition.primaryKey &&
                definition.types.sqlite !== "integer"
            ) {
                constraints.push(
                    check(
                        `${this[TABLE].name}_${definition.name}_not_null`,
                        sql`${column} IS NOT NULL`,
                    ),
                );
            }

            // expand references once every table is declared
            const reference = definition.reference;
            if (reference) {
                constraints.push(
                    new ForeignKey(
                        {
                            columns: [column],
                            foreignColumns: [reference.column()],
                        },
                        reference,
                    ),
                );
            }
        }

        return constraints;
    }
}

/** Compute new column values from a row's previous values, as SQL. */
export type RowConversion<Columns = ColumnMap> = (columns: Columns) => {
    readonly [Property in keyof Columns]?: SQL;
};

/** The previous names of a table and its columns. */
export interface TableMove {
    /** The table's previous name within its package. */
    readonly table?: string;
    /** The previous SQL column names by property. */
    readonly columns?: Readonly<Record<string, string>>;
}

/** The properties of a single-parent tree per scope. */
interface TreeColumns<Property extends string = string> {
    /** The property holding the node identity. */
    readonly id: Property;
    /** The property holding the tree scope. */
    readonly scope: Property;
    /** The property holding the nullable parent identity. */
    readonly parent: Property;
}

/** A table's declaration with defaults applied. */
interface TableDeclaration {
    /** Evaluate the constraints. */
    readonly constraints: () => readonly TableConstraint[];
    /** How long the log keeps the table's changes. */
    readonly retention: ChangeRetention;
    /** The tier of every database holding the table, any tier when absent. */
    readonly tier?: DatabaseTier;
    /** The version of the row shape. */
    readonly version: number;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** The row conversions by target version. */
    readonly convert: Readonly<Record<number, RowConversion>>;
    /** The aggregates this table's rows feed or hold. */
    readonly aggregates: readonly Aggregate[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents: readonly Dependent[];
}

/** An aggregate of one table's rows held by the rows they reference. */
export type Aggregate = AggregateOptions &
    (
        | {
              /** The holding table. */
              readonly into: () => Table;
          }
        | {
              /** The aggregated table. */
              readonly from: () => Table;
          }
    );

/** The options of an aggregate. */
interface AggregateOptions {
    /** The holding property. */
    readonly column: string;
    /** The aggregated table's property referencing the holding rows. */
    readonly key: string;
    /** The aggregate function. */
    readonly function: "count" | "sum" | "min" | "max";
    /** The aggregated property, for sums and extremes. */
    readonly value?: string;
    /** The values the aggregated rows hold. */
    readonly where?: Readonly<Record<string, string | number | boolean | null>>;
}

/** Rows of another table referencing this table's rows, as a polymorphic reference does. */
export interface Dependent {
    /** The table holding the dependent rows. */
    readonly from: () => Table;
    /** The dependent table's property referencing this table's rows. */
    readonly key: string;
    /** The values the dependent rows hold, such as the referenced type. */
    readonly where?: Readonly<Record<string, string | number | boolean | null>>;
    /** Cascade or restrict the deletion. */
    readonly onDelete: "cascade" | "restrict";
}

/** The options of a table. */
export interface TableOptions<Columns> {
    /** The tier of every database holding the table, any tier when absent. */
    readonly tier?: DatabaseTier;
    /** The constraints and indexes. */
    readonly constraints?: (columns: Columns) => readonly TableConstraint[];
    /** Log committed changes under the scope column. */
    readonly log?: "scope" extends keyof Columns
        ? {
              /** How long the log keeps the changes, the window by default. */
              readonly retention?: Exclude<ChangeRetention, "none">;
          }
        : never;
    /** The properties of a single-parent tree per scope. */
    readonly tree?: TreeColumns<keyof Columns & string>;
    /** The row shape version, one by default. */
    readonly version?: number;
    /** The previous names of the table and its columns. */
    readonly moved?: {
        /** The table's previous name within its package. */
        readonly table?: string;
        /** The previous SQL column names by property. */
        readonly columns?: { readonly [Property in keyof Columns]?: string };
    };
    /** The row conversions by target version. */
    readonly convert?: Readonly<Record<number, RowConversion<Columns>>>;
    /** The aggregates this table's rows feed or hold. */
    readonly aggregates?: readonly Aggregate[];
    /** The rows of other tables referencing this table's rows. */
    readonly dependents?: readonly Dependent[];
}

/** The columns by property. */
export type ColumnMap = Record<string, Column>;

/** The column builders by property. */
export type ColumnBuilderMap = Record<string, ColumnBuilder<unknown, boolean, boolean, boolean>>;

/** Attach each column's type and flags. */
export type TableColumnMap<Builders extends ColumnBuilderMap, Name extends string = string> = {
    [Property in keyof Builders]: Column<
        Builders[Property]["_"]["value"],
        Builders[Property]["_"]["required"],
        Builders[Property]["_"]["default"],
        Name,
        Builders[Property]["_"]["generated"]
    >;
};

/** The selected application record. */
export type Select<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["_"]["required"] extends true
        ? Definition[typeof TABLE]["columns"][Property]["_"]["value"]
        : Definition[typeof TABLE]["columns"][Property]["_"]["value"] | null;
};

/** The properties an insert must supply. */
type RequiredColumns<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["_"] extends { required: true; default: false }
        ? Property
        : never;
}[keyof Definition[typeof TABLE]["columns"]];

/** The generated properties. */
type GeneratedColumns<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["_"]["generated"] extends true
        ? Property
        : never;
}[keyof Definition[typeof TABLE]["columns"]];

/** The values of an insert. */
export type Insert<Definition extends Table> = Pick<
    Select<Definition>,
    Exclude<RequiredColumns<Definition>, GeneratedColumns<Definition>>
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
    const owner = declaringModule(module, "defineTable").package;
    const sqlName = qualify(owner, name);

    // reject generated defaults and duplicate names
    const names = new Set<string>();
    for (const builder of Object.values(builders)) {
        const definition = builder.definition;
        if (
            definition.generated &&
            (definition.default !== undefined ||
                definition.runtimeDefault ||
                definition.runtimeUpdate)
        ) {
            throw new TypeError(`generated SQL column cannot define defaults: ${definition.name}`);
        }
        if (names.has(definition.name)) {
            throw new TypeError(`duplicate SQL column: ${definition.name}`);
        }
        names.add(definition.name);
    }

    // require a conversion per version
    const version = options.version ?? 1;
    for (let target = 2; target <= version; target++) {
        if (!options.convert?.[target]) {
            throw new TypeError(`missing conversion of ${name} to version ${target}`);
        }
    }

    // attach the columns in order
    const columns = Object.fromEntries(
        Object.entries(builders).map(([property, builder]) => [
            property,
            new Column(sqlName, builder.definition),
        ]),
    ) as TableColumnMap<Builders, Name>;
    const definition = new Table(
        { package: owner, name, sqlName },
        columns,
        {
            constraints: () => options.constraints?.(columns) ?? [],
            retention: options.log === undefined ? "none" : (options.log.retention ?? "window"),
            ...(options.tier === undefined ? {} : { tier: options.tier }),
            version,
            moved: (options.moved ?? {}) as TableMove,
            convert: (options.convert ?? {}) as Readonly<Record<number, RowConversion>>,
            aggregates: options.aggregates ?? [],
            dependents: options.dependents ?? [],
        },
        options.tree === undefined ? {} : { tree: options.tree },
    );

    return Object.assign(definition, columns);
}

/** The columns of a query alias. */
export type AliasedColumnMap<Definition extends Table, Name extends string> = {
    [Property in keyof Definition[typeof TABLE]["columns"]]: Column<
        Definition[typeof TABLE]["columns"][Property]["_"]["value"],
        Definition[typeof TABLE]["columns"][Property]["_"]["required"],
        Definition[typeof TABLE]["columns"][Property]["_"]["default"],
        Name,
        Definition[typeof TABLE]["columns"][Property]["_"]["generated"]
    >;
};

/** Reference a table under another query name. */
export function alias<Definition extends Table, Name extends string>(
    source: Definition,
    name: Name,
): Table<Name, AliasedColumnMap<Definition, Name>> & AliasedColumnMap<Definition, Name> {
    // qualify each column by the alias
    const columns = Object.fromEntries(
        Object.entries(source[TABLE].columns).map(([property, column]) => [
            property,
            new Column(name, column.definition),
        ]),
    ) as AliasedColumnMap<Definition, Name>;
    const definition = new Table(
        { package: source[TABLE].package, name, sqlName: name },
        columns,
        source[TABLE],
        { source },
    );

    return Object.assign(definition, columns);
}

/** Read a table's primary key properties. */
function keyOf(table: Table): readonly string[] {
    // prefer a compound key constraint
    const columns = table[TABLE].columns;
    const declared = table
        .constraints("sqlite")
        .find((constraint): constraint is PrimaryKey => constraint.kind === "primaryKey");
    const keys =
        declared?.columns ??
        Object.values(columns).filter((column) => column.definition.primaryKey);

    // name each key column by its property
    const properties = new Map(
        Object.entries(columns).map(([property, column]) => [column, property]),
    );

    return keys.map((column) => properties.get(column)!);
}
