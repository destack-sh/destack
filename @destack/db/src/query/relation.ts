import { TABLE, type Select, type Table } from "../table/table.ts";
import type { Many, Model, One } from "./model.ts";
import { DatabaseError } from "../error/error.ts";
import { sql, type SQL } from "../sql/index.ts";
import { Condition } from "./condition.ts";
import { Order } from "./order.ts";
import type { Extras, Namespace } from "./namespace.ts";
import type { ColumnDefinition, Column } from "../table/column.ts";
import type { Rollup } from "../expression/expression.ts";

/**
 * How related rows join a row of a relation's table.
 *
 * A key path joins two columns.
 * A junction path joins through a join table.
 * A descendants or ancestors path follows a tree's parent column.
 */
export type Path =
    | {
          /** Join a column of the related row to one of the row. */
          readonly kind: "key";
          /** The related row's column, by property. */
          readonly column: string;
          /** The row's column, by property. */
          readonly parent: string;
      }
    | {
          /** Join through the rows of a join table. */
          readonly kind: "junction";
          /** The join table. */
          readonly table: Table;
          /** The join table's column naming the row, and the row's column. */
          readonly from: { readonly column: string; readonly key: string };
          /** The join table's column naming the related row, and the related column. */
          readonly to: { readonly column: string; readonly key: string };
      }
    | {
          /** Follow a tree's parent column down or up. */
          readonly kind: "descendants" | "ancestors";
          /** The column naming a row's parent. */
          readonly column: string;
      };

/** A path joining a column of the related row to one of the row. */
export type KeyPath = Extract<Path, { readonly kind: "key" }>;

/** A path joining through the rows of a join table. */
export type JunctionPath = Extract<Path, { readonly kind: "junction" }>;

/** A path following a tree's parent column down or up. */
export type TreePath = Extract<Path, { readonly kind: "descendants" | "ancestors" }>;

/** A named edge from a table's rows to related rows: one row, or many. */
export interface Relation<
    Target extends Table = Table,
    Cardinality extends "one" | "many" = "one" | "many",
> {
    /** The related table. */
    readonly table: Target;
    /** Whether a row relates to one row or to many. */
    readonly cardinality: Cardinality;
    /** How the related rows join a row. */
    readonly on: Path;
    /** The condition every related row meets. */
    readonly where?: Condition;
}

/** A column a relation joins on, through a join table's column when it joins many to many. */
export interface RelationColumn {
    /** The column's table. */
    readonly table: Table;
    /** The column, by property. */
    readonly column: string;
    /** The join table's column naming this column's rows. */
    readonly via?: RelationColumn;
    /** Join through a join table's column naming this column's rows. */
    through(column: RelationColumn): RelationColumn;
}

/** A relation joining its rows by a key or through a join table. */
type JoinedRelation = Relation & { readonly on: KeyPath | JunctionPath };

/** The rows a relational read keeps of a table along a path, every row when absent. */
export type Filter = (table: Table, path: Path) => SQL | undefined;

/** The schema's relations, by table, as queries name them, with the tables relational reads name. */
export class Relations<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The models of the named tables, as relational reads type them. */
    declare readonly _: { readonly models: Models };
    /** The tables relational reads name, by name. */
    readonly tables: Readonly<Record<string, Table>>;
    /** The relations of each table, by name. */
    readonly #tables: ReadonlyMap<Table, Readonly<Record<string, Relation>>>;

    /** Keep the relations of each table, and the tables relational reads name. */
    constructor(
        relations: ReadonlyMap<Table, Readonly<Record<string, Relation>>> = new Map(),
        tables: Readonly<Record<string, Table>> = {},
    ) {
        this.#tables = relations;
        this.tables = tables;
    }

    /** List a table's relations by name, with `descendants` and `ancestors` for a table keeping a tree. */
    of(table: Table): Readonly<Record<string, Relation>> {
        return { ...treeRelations(table), ...this.#tables.get(table) };
    }

    /** Read one relation of a table, refusing a name the table does not relate by. */
    get(table: Table, name: string): Relation {
        const relation = this.of(table)[name];
        if (relation === undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `${table[TABLE].name} has no relation ${name}`,
            );
        }

        return relation;
    }

    /**
     * Name what a table's conditions and computed values read through its relations, as SQL subqueries.
     *
     * A condition on a relation tests for a related row, a lookup reads the related row's column, and a rollup measures the related rows.
     */
    namespace(table: Table, extras: Extras = {}, filter: Filter = () => undefined): Namespace {
        const subquery = new Subquery(this, table, filter);

        return {
            extras,
            exists: (via, where) => subquery.exists(via, where),
            lookup: (via, column) => subquery.lookup(via, column),
            rollup: (measure, via, column, where) => subquery.rollup(measure, via, column, where),
        };
    }

    /** Combine relations of several schemas, refusing a table's relation named twice. */
    static merge(...schemas: readonly Relations[]): Relations {
        // keep each named table once, and each table's relations under distinct names
        const tables = new Map<Table, Record<string, Relation>>();
        const named: Record<string, Table> = {};
        for (const relations of schemas) {
            for (const [name, table] of Object.entries(relations.tables)) {
                if (Object.hasOwn(named, name) && named[name] !== table) {
                    throw new DatabaseError("INVALID_QUERY", `relations name two tables ${name}`);
                }
                named[name] = table;
            }
            for (const [table, declared] of relations.#tables) {
                const merged = tables.get(table) ?? {};
                for (const [name, relation] of Object.entries(declared)) {
                    if (Object.hasOwn(merged, name)) {
                        throw new DatabaseError(
                            "INVALID_QUERY",
                            `${table[TABLE].name} relates by ${name} twice`,
                        );
                    }
                    merged[name] = relation;
                }
                tables.set(table, merged);
            }
        }

        return new Relations(tables, named);
    }
}

/** The SQL subqueries reading a table's related rows, each correlated to the table's row. */
class Subquery {
    /** The schema's relations. */
    readonly #relations: Relations;
    /** The table whose rows the subqueries correlate to. */
    readonly #table: Table;
    /** The rows the read keeps of each table along a path. */
    readonly #filter: Filter;

    /** Read a table's related rows through the schema's relations, keeping the rows a filter keeps. */
    constructor(relations: Relations, table: Table, filter: Filter) {
        this.#relations = relations;
        this.#table = table;
        this.#filter = filter;
    }

    /** Render whether the row has a related row meeting a condition. */
    exists(via: string, where: Condition): SQL {
        // name the related rows' values the row's column takes
        const relation = this.#joined(via);
        const target = relation.table[TABLE];
        const related = (column: string) =>
            sql`SELECT ${target.column(column)} FROM ${relation.table} WHERE ${target.column(column)} IS NOT NULL
                AND ${this.#related(relation, where)}`;
        const on = relation.on;

        // test the row's key among the related values
        if (on.kind === "key") {
            const parent = this.#table[TABLE].column(on.parent);

            return sql`(${parent} IS NOT NULL AND ${parent} IN (${related(on.column)}))`;
        }
        // test the row's key among the join rows naming a related value
        else {
            const through = on.table[TABLE];
            const parent = this.#table[TABLE].column(on.from.key);

            return sql`(${parent} IS NOT NULL AND ${parent} IN (SELECT ${through.column(on.from.column)} FROM ${on.table}
                WHERE ${through.column(on.from.column)} IS NOT NULL AND ${this.#through(on)}
                AND ${through.column(on.to.column)} IN (${related(on.to.key)})))`;
        }
    }

    /** Read a column of the one related row a key names. */
    lookup(
        via: string,
        column: string,
    ): { readonly definition: ColumnDefinition; readonly value: SQL } {
        // require a key path onto another table's single-column key
        const relation = this.#joined(via);
        const target = relation.table[TABLE];
        if (
            relation.on.kind !== "key" ||
            relation.table === this.#table ||
            target.key.length !== 1 ||
            target.key[0] !== relation.on.column
        ) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `lookup ${via} of ${this.#table[TABLE].name} follows a key path onto another table's key`,
            );
        }

        // read the related column
        const read = target.columns[column];
        if (read === undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `relation ${via} looks up no column ${column} of ${target.name}`,
            );
        }

        return {
            definition: read.definition,
            value: sql`(SELECT ${read} FROM ${relation.table} WHERE ${this.#join(relation)} AND ${this.#related(relation, {})})`,
        };
    }

    /** Measure another table's related rows meeting a condition. */
    rollup(
        measure: Rollup,
        via: string,
        column: string | undefined,
        where: Condition,
    ): { readonly definition?: ColumnDefinition; readonly value: SQL } {
        // require another table
        const relation = this.#joined(via);
        if (relation.table === this.#table) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `rollup ${via} of ${this.#table[TABLE].name} measures another table`,
            );
        }

        // require the measured column
        const target = relation.table[TABLE];
        const read = column === undefined ? undefined : target.columns[column];
        if (column !== undefined && read === undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `relation ${via} measures no column ${column} of ${target.name}`,
            );
        }

        // measure the related rows
        const aggregate = aggregateOf(measure, read);
        const value = sql`(SELECT ${aggregate} FROM ${relation.table} WHERE ${this.#join(relation)} AND ${this.#related(relation, where)})`;

        return read === undefined ? { value } : { definition: read.definition, value };
    }

    /** Read a key or junction relation of the table, refusing a tree path. */
    #joined(via: string): JoinedRelation {
        // refuse a tree path
        const relation = this.#relations.get(this.#table, via);
        const on = relation.on;
        if (on.kind !== "key" && on.kind !== "junction") {
            throw new DatabaseError(
                "INVALID_QUERY",
                `relation ${via} of ${this.#table[TABLE].name} follows a key or junction path`,
            );
        }

        return { ...relation, on };
    }

    /** Render the join of a related row to the table's row. */
    #join(relation: JoinedRelation): SQL {
        // match the row's key
        const target = relation.table[TABLE];
        const on = relation.on;
        if (on.kind === "key") {
            return sql`${target.column(on.column)} = ${this.#table[TABLE].column(on.parent)}`;
        }
        // name the related rows through the join rows of the row
        else {
            const through = on.table[TABLE];

            return sql`${target.column(on.to.key)} IN (SELECT ${through.column(on.to.column)} FROM ${on.table}
                WHERE ${through.column(on.from.column)} = ${this.#table[TABLE].column(on.from.key)} AND ${this.#through(on)})`;
        }
    }

    /** Render the related rows the filter keeps, meeting the relation's condition and another. */
    #related(relation: Relation, where: Condition): SQL {
        // meet both conditions in the related table's namespace, on the rows the filter keeps
        const condition = relation.where === undefined ? where : { AND: [relation.where, where] };
        const namespace = this.#relations.namespace(relation.table, {}, this.#filter);
        const rendered = Condition.render(condition, relation.table, namespace);

        return sql`${rendered} AND ${this.#filter(relation.table, relation.on) ?? sql`true`}`;
    }

    /** Render the join rows the filter keeps. */
    #through(path: JunctionPath): SQL {
        return this.#filter(path.table, path) ?? sql`true`;
    }
}

/** The columns of a table, as relations name them. */
type ColumnsOf<Of extends Table> = Of extends Table<string, infer Columns> ? Columns : never;

/** Declare a relation to a table, to one related row or many. */
type RelationTo<
    Target extends Table = Table,
    Cardinality extends "one" | "many" = "one" | "many",
> = (options: {
    /** The row's column. */
    readonly from: RelationColumn;
    /** The related row's column. */
    readonly to: RelationColumn;
    /** The condition every related row meets. */
    readonly where?: Condition;
}) => Relation<Target, Cardinality>;

/** The builder `defineRelations` passes: `r.one.<table>`, `r.many.<table>` and `r.<table>.<column>`. */
export type RelationBuilder<Tables extends Readonly<Record<string, Table>>> = {
    /** Declare a relation to one row of a table. */
    readonly one: { readonly [Name in keyof Tables]: RelationTo<Tables[Name], "one"> };
    /** Declare a relation to many rows of a table. */
    readonly many: { readonly [Name in keyof Tables]: RelationTo<Tables[Name], "many"> };
} & {
    readonly [Name in keyof Tables]: {
        readonly [Property in keyof ColumnsOf<Tables[Name]>]: RelationColumn;
    };
};

/** The relations a schema declares, by table name and relation name. */
type Declared<Tables> = { readonly [Name in keyof Tables]?: Readonly<Record<string, Relation>> };

/** The models of a schema's tables, as relational reads type them. */
export type ModelsOf<Tables extends Readonly<Record<string, Table>>, Relations> = {
    readonly [Name in keyof Tables]: TableModel<Tables, Relations, Name>;
};

/** The model of one table: its selected row, and the related tables' models. */
type TableModel<
    Tables extends Readonly<Record<string, Table>>,
    Relations,
    Name extends keyof Tables,
> = {
    readonly row: Select<Tables[Name]>;
    readonly value: Select<Tables[Name]>;
    readonly relations: Name extends keyof Relations
        ? {
              readonly [Key in keyof Relations[Name]]: Relations[Name][Key] extends Relation<
                  infer Target,
                  infer Cardinality
              >
                  ? Cardinality extends "one"
                      ? One<TableModel<Tables, Relations, NameOf<Tables, Target>>>
                      : Many<TableModel<Tables, Relations, NameOf<Tables, Target>>>
                  : never;
          }
        : {};
};

/** The name of a table among a schema's tables. */
type NameOf<Tables extends Readonly<Record<string, Table>>, Target> = {
    [Name in keyof Tables]: [Tables[Name]] extends [Target]
        ? [Target] extends [Tables[Name]]
            ? Name
            : never
        : never;
}[keyof Tables];

/** Declare a schema's relations by table. */
export function defineRelations<
    const Tables extends Readonly<Record<string, Table>>,
    const Defined extends Declared<Tables> = {},
>(
    tables: Tables,
    define: (builder: RelationBuilder<Tables>) => Defined,
): Relations<ModelsOf<Tables, Defined>> {
    // key each declared table's relations by the table itself
    const declared: Readonly<Record<string, Readonly<Record<string, Relation>> | undefined>> =
        define(builderOf(tables));
    const relations = new Map<Table, Readonly<Record<string, Relation>>>();
    for (const [name, named] of Object.entries(declared)) {
        const table = tables[name];
        if (table === undefined) {
            throw new DatabaseError("INVALID_QUERY", `relations name no table ${name}`);
        }
        if (named !== undefined) {
            relations.set(table, named);
        }
    }

    return new Relations(relations, tables);
}

/** Build the relation builder of some tables. */
function builderOf<const Tables extends Readonly<Record<string, Table>>>(
    tables: Tables,
): RelationBuilder<Tables>;
/**
 * Build the relation builder of some tables.
 *
 * @construct the builder holds a relation declaration and a column reference for every key of the tables it is given.
 */
function builderOf(
    tables: Readonly<Record<string, Table>>,
): Record<string, Record<string, RelationColumn | RelationTo>> {
    // declare relations to each table, and name each table's columns
    const entries = Object.entries(tables);
    const one = entries.map(([name, table]): [string, RelationTo] => [
        name,
        relationTo(table, "one"),
    ]);
    const many = entries.map(([name, table]): [string, RelationTo] => [
        name,
        relationTo(table, "many"),
    ]);
    const columns = entries.map(([name, table]): [string, Record<string, RelationColumn>] => [
        name,
        columnsOf(table),
    ]);

    return {
        one: Object.fromEntries(one),
        many: Object.fromEntries(many),
        ...Object.fromEntries(columns),
    };
}

/** Name each column of a table for a relation, by property. */
function columnsOf(table: Table): Record<string, RelationColumn> {
    return Object.fromEntries(
        Object.keys(table[TABLE].columns).map((column) => [column, columnOf(table, column)]),
    );
}

/** Name a table's column for a relation. */
function columnOf(table: Table, column: string, via?: RelationColumn): RelationColumn {
    return {
        table,
        column,
        ...(via === undefined ? {} : { via }),
        through: (joined) => columnOf(table, column, joined),
    };
}

/** Declare relations to a table, one related row or many. */
function relationTo(table: Table, cardinality: "one" | "many"): RelationTo {
    return ({ from, to, where }) => {
        // require the related end on the related table
        const on = pathOf(table, from, to);
        if (to.table !== table) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `a relation to ${table[TABLE].name} joins a column of ${to.table[TABLE].name}`,
            );
        }

        return { table, cardinality, on, ...(where === undefined ? {} : { where }) };
    };
}

/** Read the path a relation's two ends join along: two columns, or a join table's two columns. */
function pathOf(table: Table, from: RelationColumn, to: RelationColumn): Path {
    // join through a join table when both ends name its columns
    if (from.via !== undefined && to.via !== undefined) {
        if (from.via.table !== to.via.table) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `a relation to ${table[TABLE].name} joins through two tables`,
            );
        }

        return {
            kind: "junction",
            table: from.via.table,
            from: { column: from.via.column, key: from.column },
            to: { column: to.via.column, key: to.column },
        };
    }
    // join two columns directly
    else if (from.via === undefined && to.via === undefined) {
        return { kind: "key", column: to.column, parent: from.column };
    }
    // refuse a join table on one end only
    else {
        throw new DatabaseError(
            "INVALID_QUERY",
            `a relation to ${table[TABLE].name} names a join table on one end only`,
        );
    }
}

/** List the relations following a table's tree down and up, none for a table without a tree. */
function treeRelations(table: Table): Readonly<Record<string, Relation>> {
    // relate no rows without a tree
    const tree = table[TABLE].tree?.definition;
    if (tree === undefined) {
        return {};
    }

    return {
        descendants: {
            table,
            cardinality: "many",
            on: { kind: "descendants", column: tree.parent },
        },
        ancestors: { table, cardinality: "many", on: { kind: "ancestors", column: tree.parent } },
    };
}

/** Render a rollup's aggregate: a row count, or a function of the measured column. */
function aggregateOf(measure: Rollup, column: Column | undefined): SQL {
    // count the rows
    if (measure === "count" || column === undefined) {
        return sql`count(*)`;
    }
    // add up the column's numbers
    else if (measure === "sum") {
        return sql`sum(${column})`;
    }
    // take the least or greatest value, text by byte
    else {
        return sql`${sql.raw(measure)}(${Order.text(column)})`;
    }
}
