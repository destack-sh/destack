import { type SQL, sql, type SQLWrapper } from "drizzle-orm";
import { Column, type ColumnBuilder } from "./column.ts";
import { check, ForeignKey, type PrimaryKey, type TableConstraint } from "./constraint.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { qualify } from "./namespace.ts";
import type { ChangeTier } from "../inspect/log.ts";
import { Tree } from "../tree/tree.ts";

/** The table declaration, separate from user-defined column properties. */
export const TABLE = Symbol("destack.table");

/** The declaration and columns of one logical SQL table. */
export class Table<
    Name extends string = string,
    Columns extends ColumnMap = ColumnMap,
> implements SQLWrapper {
    /** The table's identity, columns and declaration. */
    readonly [TABLE]: TableDeclaration & {
        /** The package declaring the table, supplied by the module transform. */
        readonly package: Package;
        /** The table name within its package. */
        readonly name: Name;
        /** The SQL identifier, qualified by the package namespace or naming a query alias. */
        readonly sqlName: string;
        /** Columns indexed by application property name. */
        readonly columns: Columns;
        /** The ancestor index maintained over the table's parent column. */
        readonly tree?: Tree;
        /** The original table when this declaration is a query alias. */
        readonly source?: Table;
        /** The properties holding the primary key in key order, none for a table without one. */
        readonly key: readonly string[];
        /** The columns by property in declaration order, which row codecs walk. */
        readonly entries: readonly (readonly [string, Column])[];
        /** The columns the log records, by property: every column except binary and sensitive ones. */
        readonly logged: Readonly<Record<string, Column>>;
    };
    /** The primary key's properties, read once the constraints can be evaluated. */
    #key: readonly string[] | undefined;
    /** The columns by property in declaration order, read once. */
    #entries: readonly (readonly [string, Column])[] | undefined;
    /** The logged columns by property, read once. */
    #logged: Readonly<Record<string, Column>> | undefined;
    /** The statements over the table, built once each, by name. */
    readonly #statements = new Map<string, unknown>();
    /** The selected application record type. */
    declare readonly $inferSelect: Select<Table<Name, Columns>>;
    /** The inserted application record type. */
    declare readonly $inferInsert: Insert<Table<Name, Columns>>;

    /** Retain the table's identity, columns and declaration, building its tree over its own columns. */
    constructor(
        identity: { readonly package: Package; readonly name: Name; readonly sqlName: string },
        columns: Columns,
        declaration: TableDeclaration,
        options: { readonly tree?: TreeColumns; readonly source?: Table } = {},
    ) {
        // declare the table, reading its key once its constraints can be evaluated, or an alias's from its source
        const { constraints, tier, version, moved, convert, aggregates } = declaration;
        const declared = {
            ...identity,
            columns,
            constraints,
            tier,
            version,
            moved,
            convert,
            aggregates,
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
        // reuse the statement of the same name
        const known = this.#statements.get(name) as Value | undefined;
        if (known !== undefined) {
            return known;
        }

        // build it once
        const built = build();
        this.#statements.set(name, built);

        return built;
    }

    /** Return the table identifier. */
    getSQL(): SQL {
        return sql`${sql.identifier(this[TABLE].sqlName)}`;
    }

    /** Emit table identifiers without parentheses. */
    shouldOmitSQLParens(): boolean {
        return true;
    }

    /** Collect declared constraints and column references for a dialect. */
    constraints(dialect: Dialect): readonly TableConstraint[] {
        const constraints = [...this[TABLE].constraints()];

        // preserve primary-key checks required by SQLite migration output
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

            // expand references after all table declarations have been evaluated
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

/** Compute new column values from a row's previous values, as SQL over the table's columns. */
export type RowConversion<Columns = ColumnMap> = (columns: Columns) => {
    readonly [Property in keyof Columns]?: SQL;
};

/** The previous names of a table and its columns. */
export interface TableMove {
    /** The table's previous name within its package. */
    readonly table?: string;
    /** Previous SQL column names indexed by current application property. */
    readonly columns?: Readonly<Record<string, string>>;
}

/** The properties of a single-parent tree maintained within each scope. */
interface TreeColumns<Property extends string = string> {
    /** The property holding the node identity. */
    readonly id: Property;
    /** The property holding the tree scope. */
    readonly scope: Property;
    /** The property holding the nullable parent identity. */
    readonly parent: Property;
}

/** A table's declaration beyond its identity and columns, with defaults applied. */
interface TableDeclaration {
    /** Evaluate constraints after referenced tables have been declared. */
    readonly constraints: () => readonly TableConstraint[];
    /** How long committed changes stay in the log, none for unlogged tables. */
    readonly tier: ChangeTier;
    /** The version of the row shape. */
    readonly version: number;
    /** The table's previous names. */
    readonly moved: TableMove;
    /** Row conversions indexed by the version each converts to. */
    readonly convert: Readonly<Record<number, RowConversion>>;
    /** The aggregates of this table's rows that other tables hold. */
    readonly aggregates: readonly Aggregate[];
}

/** An aggregate of a table's rows another table's rows hold, each over the rows referencing it: a count, sum, minimum or maximum. */
export interface Aggregate {
    /** The table holding the aggregate. */
    readonly into: () => Table;
    /** The holding table's property keeping the aggregate. */
    readonly column: string;
    /** This table's property referencing the holding rows. */
    readonly key: string;
    /** The aggregate function. */
    readonly function: "count" | "sum" | "min" | "max";
    /** The aggregated property, for sums, minimums and maximums. */
    readonly value?: string;
    /** The values the aggregated rows' properties hold. */
    readonly where?: Readonly<Record<string, string | number | boolean | null>>;
}

/** How a table is declared beyond its name and columns. */
export interface TableOptions<Columns> {
    /** Constraints and indexes, evaluated after referenced tables are declared. */
    readonly constraints?: (columns: Columns) => readonly TableConstraint[];
    /** Log committed changes, filed under the scope column each row lives in, which a logged table requires. */
    readonly log?: "scope" extends keyof Columns
        ? {
              /** The retention tier, the window by default. */
              readonly tier?: Exclude<ChangeTier, "none">;
          }
        : never;
    /** The properties of a single-parent tree maintained within each scope. */
    readonly tree?: TreeColumns<keyof Columns & string>;
    /** The version of the row shape, one by default, raised with each conversion. */
    readonly version?: number;
    /** The table's previous name and its columns' previous SQL names. */
    readonly moved?: {
        /** The table's previous name within its package. */
        readonly table?: string;
        /** Previous SQL column names indexed by current property. */
        readonly columns?: { readonly [Property in keyof Columns]?: string };
    };
    /** Row conversions indexed by the version each converts to. */
    readonly convert?: Readonly<Record<number, RowConversion<Columns>>>;
    /** The aggregates of this table's rows that other tables hold, kept current as the rows change. */
    readonly aggregates?: readonly Aggregate[];
}

/** Columns indexed by application property name. */
export type ColumnMap = Record<string, Column>;

/** Column builders indexed by application property name. */
export type ColumnBuilderMap = Record<string, ColumnBuilder<unknown, boolean, boolean, boolean>>;

/** Attach each column's application type and insertion flags. */
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

/** Properties that an insert must supply. */
type RequiredColumns<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["_"] extends { required: true; default: false }
        ? Property
        : never;
}[keyof Definition[typeof TABLE]["columns"]];

/** Properties computed by SQL and excluded from writes. */
type GeneratedColumns<Definition extends Table> = {
    [
        Property in keyof Definition[typeof TABLE]["columns"]
    ]: Definition[typeof TABLE]["columns"][Property]["_"]["generated"] extends true
        ? Property
        : never;
}[keyof Definition[typeof TABLE]["columns"]];

/** The application values accepted by an insert. */
export type Insert<Definition extends Table> = Pick<
    Select<Definition>,
    Exclude<RequiredColumns<Definition>, GeneratedColumns<Definition>>
> &
    Partial<Omit<Select<Definition>, RequiredColumns<Definition> | GeneratedColumns<Definition>>>;

/** Declare a table with typed columns, constraints, log retention, tree and shape history. */
export function defineTable<Name extends string, Builders extends ColumnBuilderMap>(
    name: Name,
    builders: Builders & { [Property in Extract<keyof Table, string>]?: never },
    options: TableOptions<TableColumnMap<Builders, Name>> = {},
    module?: ModuleMetadata,
): Table<Name, TableColumnMap<Builders, Name>> & TableColumnMap<Builders, Name> {
    // qualify the table by its stamped package
    const owner = declaringModule(module, "defineTable").package;
    const sqlName = qualify(owner, name);

    // reject generated columns with defaults and duplicate persisted column names
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

    // require one conversion for each version above the first
    const version = options.version ?? 1;
    for (let target = 2; target <= version; target++) {
        if (!options.convert?.[target]) {
            throw new TypeError(`missing conversion of ${name} to version ${target}`);
        }
    }

    // attach fresh columns while preserving their declared property order
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
            tier: options.log === undefined ? "none" : (options.log.tier ?? "window"),
            version,
            moved: (options.moved ?? {}) as TableMove,
            convert: (options.convert ?? {}) as Readonly<Record<number, RowConversion>>,
            aggregates: options.aggregates ?? [],
        },
        options.tree === undefined ? {} : { tree: options.tree },
    );

    return Object.assign(definition, columns);
}

/** Columns qualified by a query alias. */
export type AliasedColumnMap<Definition extends Table, Name extends string> = {
    [Property in keyof Definition[typeof TABLE]["columns"]]: Column<
        Definition[typeof TABLE]["columns"][Property]["_"]["value"],
        Definition[typeof TABLE]["columns"][Property]["_"]["required"],
        Definition[typeof TABLE]["columns"][Property]["_"]["default"],
        Name,
        Definition[typeof TABLE]["columns"][Property]["_"]["generated"]
    >;
};

/** Reference the same table under a distinct query name. */
export function alias<Definition extends Table, Name extends string>(
    source: Definition,
    name: Name,
): Table<Name, AliasedColumnMap<Definition, Name>> & AliasedColumnMap<Definition, Name> {
    // retain column types while giving each reference its own SQL qualifier
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

/** Read a table's primary key properties: a compound key constraint's, else the key columns'. */
function keyOf(table: Table): readonly string[] {
    // prefer a compound key constraint over column-level keys
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
