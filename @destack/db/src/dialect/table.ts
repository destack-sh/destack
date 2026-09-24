import type { Many, One } from "drizzle-orm/relations";
import type { Relation, TableRelations } from "../schema/relation.ts";
import type { ColumnBaseConfig } from "drizzle-orm";
import type { SQLiteColumn, SQLiteTableWithColumns } from "drizzle-orm/sqlite-core";
import type { PgColumn, PgTableWithColumns } from "drizzle-orm/pg-core";
import type { Dialect } from "./dialect.ts";
import type { Column } from "../table/column.ts";
import type { TABLE, Table } from "../table/table.ts";

/** A concrete Drizzle table retaining its logical application types. */
export type NativeTable<Driver extends Dialect, Definition extends Table> = {
    sqlite: SQLiteTableWithColumns<{
        name: Definition[typeof TABLE]["name"];
        schema: undefined;
        dialect: "sqlite";
        columns: NativeColumns<"sqlite", Definition>;
    }>;
    postgresql: PgTableWithColumns<{
        name: Definition[typeof TABLE]["name"];
        schema: undefined;
        dialect: "pg";
        columns: NativeColumns<"postgresql", Definition>;
    }>;
}[Driver];

/** Native relational queries retaining declared table and relationship types. */
export type NativeRelations<
    Driver extends Dialect,
    Definitions extends Record<string, TableRelations>,
> = {
    [Name in keyof Definitions]: {
        table: NativeTable<Driver, Definitions[Name]["table"]>;
        name: Name & string;
        relations: {
            [
                Property in keyof Definitions[Name]["relations"]
            ]: Definitions[Name]["relations"][Property] extends Relation<
                infer Target,
                infer Cardinality,
                infer Optional
            >
                ? Cardinality extends "one"
                    ? One<Target, Optional>
                    : Many<Target>
                : never;
        };
    };
};

/** Concrete columns indexed by application property name. */
type NativeColumns<Driver extends Dialect, Definition extends Table> = {
    [Property in keyof Definition[typeof TABLE]["columns"]]: {
        sqlite: SQLiteColumn<NativeColumn<Definition[typeof TABLE]["columns"][Property]>>;
        postgresql: PgColumn<"custom", NativeColumn<Definition[typeof TABLE]["columns"][Property]>>;
    }[Driver];
};

/** Drizzle inference fields supplied by one logical column. */
type NativeColumn<Definition extends Column> = Omit<
    ColumnBaseConfig<"custom">,
    "data" | "notNull" | "hasDefault" | "tableName" | "generated" | "identity"
> & {
    // oxlint-disable-next-line destack/no-sludge -- Drizzle column configuration key
    data: Definition["_"]["value"];
    notNull: Definition["_"]["required"];
    hasDefault: Definition["_"]["default"];
    tableName: Definition["table"];
    generated: Definition["_"]["generated"] extends true ? true : undefined;
    identity: undefined;
};
