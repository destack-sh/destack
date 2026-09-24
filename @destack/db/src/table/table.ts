import { type SQL, sql, type SQLWrapper } from "drizzle-orm";
import { Column, type ColumnBuilder } from "./column.ts";
import { check, ForeignKey, type TableConstraint } from "./constraint.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The table declaration, separate from user-defined column properties. */
export const TABLE = Symbol("destack.table");

/** The declaration and columns of one logical SQL table. */
export class Table<
    Name extends string = string,
    Columns extends ColumnMap = ColumnMap,
> implements SQLWrapper {
    /** The SQL name, columns, and deferred constraints. */
    readonly [TABLE]: {
        /** The SQL table name. */
        readonly name: Name;
        /** Columns indexed by application property name. */
        readonly columns: Columns;
        /** Evaluate constraints after referenced tables have been declared. */
        readonly constraints: () => readonly TableConstraint[];
        /** The original table when this declaration is a query alias. */
        readonly source?: Table;
    };
    /** The selected application record type. */
    declare readonly $inferSelect: Select<Table<Name, Columns>>;
    /** The inserted application record type. */
    declare readonly $inferInsert: Insert<Table<Name, Columns>>;

    /** Retain the table declaration. */
    constructor(
        name: Name,
        columns: Columns,
        constraints: () => readonly TableConstraint[],
        source?: Table,
    ) {
        this[TABLE] = { name, columns, constraints, source };
    }

    /** Return the table identifier. */
    getSQL(): SQL {
        return sql`${sql.identifier(this[TABLE].name)}`;
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

/** Columns indexed by application property name. */
export type ColumnMap = Record<string, Column>;

/** Column builders indexed by application property name. */
export type ColumnBuilders = Record<string, ColumnBuilder<unknown, boolean, boolean, boolean>>;

/** Attach each column's application type and insertion flags. */
export type TableColumns<Builders extends ColumnBuilders, Name extends string = string> = {
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

/** Declare a table with typed columns and deferred constraints. */
export function table<Name extends string, Builders extends ColumnBuilders>(
    name: Name,
    builders: Builders & { [Property in Extract<keyof Table, string>]?: never },
    constraints?: (columns: TableColumns<Builders, Name>) => readonly TableConstraint[],
): Table<Name, TableColumns<Builders, Name>> & TableColumns<Builders, Name> {
    // reject duplicate persisted column names before constructing SQL
    const names = new Set<string>();
    for (const builder of Object.values(builders)) {
        const definition = builder.definition;
        if (
            definition.generated &&
            (definition.default !== undefined ||
                definition.runtimeDefault ||
                definition.runtimeUpdate)
        ) {
            throw new TypeError(`Generated SQL column cannot define defaults: ${definition.name}.`);
        }
        const name = builder.definition.name;
        if (names.has(name)) {
            throw new TypeError(`duplicate SQL column: ${name}`);
        }
        names.add(name);
    }

    // attach fresh columns while preserving their declared property order
    const columns = Object.fromEntries(
        Object.entries(builders).map(([property, builder]) => [
            property,
            new Column(name, builder.definition),
        ]),
    ) as TableColumns<Builders, Name>;
    const definition = new Table(name, columns, () => constraints?.(columns) ?? []);

    return Object.assign(definition, columns);
}

/** Columns qualified by a query alias. */
export type AliasedColumns<Definition extends Table, Name extends string> = {
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
): Table<Name, AliasedColumns<Definition, Name>> & AliasedColumns<Definition, Name> {
    // retain column types while giving each reference its own SQL qualifier
    const columns = Object.fromEntries(
        Object.entries(source[TABLE].columns).map(([property, column]) => [
            property,
            new Column(name, column.definition),
        ]),
    ) as AliasedColumns<Definition, Name>;
    const definition = new Table(name, columns, () => [], source);

    return Object.assign(definition, columns);
}
