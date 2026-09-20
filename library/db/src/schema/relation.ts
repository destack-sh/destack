import type { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";

/** A named relationship between logical tables. */
export interface Relation<
    Target extends string = string,
    Cardinality extends "one" | "many" = "one" | "many",
    Optional extends boolean = boolean,
> {
    /** The referenced table key in the relation collection. */
    readonly target: Target;
    /** The referenced table. */
    readonly table: Table;
    /** Whether the relationship returns one row or many rows. */
    readonly cardinality: Cardinality;
    /** The columns from the referencing row. */
    readonly from: readonly Column[];
    /** The columns from the referenced row. */
    readonly to: readonly Column[];
    /** Whether a single referenced row may be absent. */
    readonly optional: Optional;
}

/** A table and its named relationships. */
export interface TableRelations<
    Definition extends Table = Table,
    Relations extends Readonly<Record<string, Relation>> = Readonly<Record<string, Relation>>,
> {
    /** The logical table. */
    readonly table: Definition;
    /** Relationships indexed by application property. */
    readonly relations: Relations;
}

/** The columns used to connect two rows. */
export interface RelationColumns {
    /** The columns from the referencing row. */
    readonly from: Column | readonly Column[];
    /** The columns from the referenced row. */
    readonly to: Column | readonly Column[];
    /** Whether the referenced row may be absent. */
    readonly optional?: boolean;
}

/** Typed table columns and relationship declarations. */
export type RelationBuilder<Tables extends Record<string, Table>> =
    & {
        [Name in keyof Tables]: Tables[Name][typeof TABLE]["columns"];
    }
    & {
        /** Declare a single-row relationship. */
        one: {
            [Name in keyof Tables]: <Optional extends boolean = true>(
                columns: RelationColumns & { optional?: Optional },
            ) => Relation<Name & string, "one", Optional>;
        };
        /** Declare a multiple-row relationship. */
        many: {
            [Name in keyof Tables]: (
                columns: RelationColumns,
            ) => Relation<Name & string, "many", true>;
        };
    };

/** Declare relationships while retaining all supplied tables. */
export function defineRelations<
    Tables extends Record<string, Table>,
    Definitions extends Partial<Record<keyof Tables, Record<string, Relation>>> = {},
>(
    tables: Tables,
    define?: (
        builder: RelationBuilder<Tables>,
    ) => Definitions,
): {
    [Name in keyof Tables]: TableRelations<
        Tables[Name],
        Name extends keyof Definitions ? NonNullable<Definitions[Name]> : {}
    >;
} {
    const definitions: Partial<Record<string, Record<string, Relation>>> =
        define?.(createRelationBuilder(tables)) ?? {};

    return Object.fromEntries(
        Object.entries(tables).map(([name, table]) => [
            name,
            { table, relations: definitions[name] ?? {} },
        ]),
    ) as {
        [Name in keyof Tables]: TableRelations<
            Tables[Name],
            Name extends keyof Definitions ? NonNullable<Definitions[Name]> : {}
        >;
    };
}

/** Declare a composable group of table relationships. */
export function defineRelationsPart<
    Tables extends Record<string, Table>,
    Definitions extends Partial<Record<keyof Tables, Record<string, Relation>>>,
>(
    tables: Tables,
    define: (builder: RelationBuilder<Tables>) => Definitions,
): {
    [Name in keyof Definitions & keyof Tables]: TableRelations<
        Tables[Name],
        NonNullable<Definitions[Name]>
    >;
} {
    const definitions = define(createRelationBuilder(tables));

    return Object.fromEntries(
        Object.keys(definitions).map((name) => [
            name,
            { table: tables[name], relations: definitions[name] },
        ]),
    ) as {
        [Name in keyof Definitions & keyof Tables]: TableRelations<
            Tables[Name],
            NonNullable<Definitions[Name]>
        >;
    };
}

/** Normalize a relationship's column lists. */
function relation(
    target: string,
    table: Table,
    cardinality: "one" | "many",
    columns: RelationColumns,
): Relation {
    const from = Array.isArray(columns.from) ? columns.from : [columns.from as Column];
    const to = Array.isArray(columns.to) ? columns.to : [columns.to as Column];
    if (from.length === 0 || from.length !== to.length) {
        throw new TypeError("A relation requires matching, nonempty column lists.");
    }

    return { target, table, cardinality, from, to, optional: columns.optional ?? true };
}

/** Create the typed table and relation declarations. */
function createRelationBuilder<Tables extends Record<string, Table>>(
    tables: Tables,
): RelationBuilder<Tables> {
    // expose table columns and cardinality-specific declarations
    const one: Record<string, (columns: RelationColumns) => Relation> = {};
    const many: Record<string, (columns: RelationColumns) => Relation> = {};
    const columns: Record<string, Table[typeof TABLE]["columns"]> = {};
    for (const [name, table] of Object.entries(tables)) {
        columns[name] = table[TABLE].columns;
        one[name] = (columns) => relation(name, table, "one", columns);
        many[name] = (columns) => relation(name, table, "many", columns);
    }

    return { ...columns, one, many } as RelationBuilder<Tables>;
}
