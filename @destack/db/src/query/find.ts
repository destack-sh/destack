import { sql, type SQL } from "../sql/index.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import type { Column } from "../table/column.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { DatabaseError } from "../error/error.ts";
import { Expression } from "../expression/expression.ts";
import { Comparable, Condition } from "./condition.ts";
import { Order } from "./order.ts";
import type { QueryOptions } from "./option.ts";
import type { Path, Relation, Relations, TreePath } from "./relation.ts";
import type { FindOptions, FindResult, Model } from "./model.ts";

/** The most parent values one statement reads rows for, a parameter each. */
const BATCH = 500;

/** A level's options: what it selects of the rows and which columns it keeps. */
type ReadOptions = QueryOptions & {
    /** The columns each result keeps: only those set true, or all but those set false. */
    readonly columns?: Readonly<Record<string, boolean | undefined>>;
};

/** The parent rows a level reads rows for: the path joining them and the parents' values. */
interface Link {
    /** How the rows join their parent rows. */
    readonly path: Path;
    /** The parent rows' joined values, in JSON form, by their name. */
    readonly values: ReadonlyMap<string, Comparable>;
}

/** One batch of parents a statement reads rows for. */
interface Parents {
    /** How the rows join their parent rows. */
    readonly join: Join;
    /** The parents' joined values, in JSON form. */
    readonly values: readonly Comparable[];
}

/** One row a level read: its values, and the parent value it was read for. */
interface Read {
    /** The row's table columns. */
    readonly columns: Row;
    /** The row's columns and extras, then its includes. */
    readonly row: Record<string, unknown>;
    /** The parent value's name, absent for a root. */
    readonly parent: string | undefined;
}

/** The column of a level's rows that its related rows join on. */
interface Parent {
    /** The column, by property. */
    readonly name: string;
    /** The column. */
    readonly column: Column;
}

/** A selected value's decoder, as columns and SQL expressions decode. */
interface Decoder<Value> {
    /** Decode a driver value. */
    decode(value: unknown, dialect: Dialect): Value;
}

/** Relational reads of a schema's named tables. */
export type Queries<Models extends Readonly<Record<string, Model>>> = {
    readonly [Name in keyof Models]: RelationalQueryBuilder<Models[Name]>;
};

/** Relational reads of one table. */
export class RelationalQueryBuilder<Entity extends Model = Model> {
    /** The connection reading the rows. */
    readonly #connection: DatabaseConnection;
    /** The read table. */
    readonly #table: Table;
    /** The schema's relations, which `with`, conditions and extras name. */
    readonly #relations: Relations;

    /** Read a table through a connection by the schema's relations. */
    constructor(connection: DatabaseConnection, table: Table, relations: Relations) {
        this.#connection = connection;
        this.#table = table;
        this.#relations = relations;
    }

    /** Read each table the relations name through a connection. */
    static of<Models extends Readonly<Record<string, Model>>>(
        connection: DatabaseConnection,
        relations: Relations<Models>,
    ): Queries<Models>;
    /**
     * Read each table the relations name through a connection.
     *
     * @construct the relations name each table under its model's key, which is how Queries maps the models.
     */
    static of(
        connection: DatabaseConnection,
        relations: Relations,
    ): Record<string, RelationalQueryBuilder> {
        return Object.fromEntries(
            Object.entries(relations.tables).map(([name, table]) => [
                name,
                new RelationalQueryBuilder(connection, table, relations),
            ]),
        );
    }

    /** Read the rows meeting the options, each with its included relations. */
    findMany<const Options extends FindOptions<Entity>>(
        options?: Options,
    ): Promise<FindResult<Entity, Options>[]>;
    /**
     * Read the rows meeting the options, each with its included relations.
     *
     * @construct each result holds its table's selected columns, the extras and each included relation's results, which is how FindResult maps the options.
     */
    findMany(options: ReadOptions = {}): Promise<unknown[]> {
        return this.#find(options);
    }

    /** Read the first row meeting the options, with its included relations. */
    findFirst<const Options extends Omit<FindOptions<Entity>, "limit">>(
        options?: Options,
    ): Promise<FindResult<Entity, Options> | undefined>;
    /**
     * Read the first row meeting the options, with its included relations.
     *
     * @construct the result is the first of findMany's, which FindResult maps the same way.
     */
    async findFirst(options: ReadOptions = {}): Promise<unknown> {
        const [first] = await this.#find({ ...options, limit: 1 });

        return first;
    }

    /** Read the rows of a level and below in one read transaction. */
    #find(options: ReadOptions): Promise<Record<string, unknown>[]> {
        return this.#connection.transaction(
            async (transaction) => {
                const reads = await read(transaction, this.#relations, this.#table, options);

                return reads.map((entry) => entry.row);
            },
            { isReadOnly: true },
        );
    }
}

/** One level of a relational read: the statement selecting a table's rows and their decoding. */
class Level {
    /** The read table. */
    readonly table: Table;
    /** The most rows selected, per parent below a root. */
    readonly #limit: number | undefined;
    /** The table's columns, by property, in selection order. */
    readonly #columns: readonly (readonly [string, Column])[];
    /** The extras as selected, by name, after the columns. */
    readonly #extras: readonly (readonly [string, SQL])[];
    /** The selected columns and extras, each under its position's alias. */
    readonly #fields: readonly SQL[];
    /** The ORDER BY list, completed by the key. */
    readonly #order: SQL;
    /** The condition the rows meet. */
    readonly #where: SQL;

    /** Render a level's selection, order and condition in the table's namespace. */
    constructor(relations: Relations, table: Table, options: ReadOptions) {
        // select the columns and extras
        const extras = options.extras ?? {};
        const namespace = relations.namespace(table, extras);
        this.table = table;
        this.#limit = options.limit;
        this.#columns = Object.entries(table[TABLE].columns);
        this.#extras = Object.entries(extras).map(([name, expression]) => [
            name,
            Expression.select(expression, table, namespace),
        ]);

        // alias each field by its position
        const values = [
            ...this.#columns.map(([, column]) => column),
            ...this.#extras.map(([, value]) => value),
        ];
        this.#fields = values.map(
            (value, index) => sql`${value} AS ${sql.identifier(`f${index}`)}`,
        );

        // order by the options and the key, and render the condition
        const order = Order.complete(Order.of(options.orderBy ?? {}), table);
        Order.require(order, table, namespace);
        this.#where = Condition.render(options.where ?? {}, table, namespace);
        this.#order = sql.join(Order.render(order, table, namespace), sql.raw(", "));
    }

    /** Read the level's rows, one statement per batch of parent values below a root. */
    async select(connection: DatabaseConnection, link: Link | undefined): Promise<Read[]> {
        // read a root's rows in one statement
        const dialect = connection.dialect;
        if (link === undefined) {
            const rows = await connection.values(this.statement(undefined));

            return rows.map((values) => this.decode(values, undefined, dialect));
        }

        // read the rows of each batch of parents
        const join = Join.of(this.table, link.path);
        const reads: Read[] = [];
        for (const values of batches([...link.values.values()], BATCH)) {
            const rows = await connection.values(this.statement({ join, values }));
            for (const row of rows) {
                reads.push(this.decode(row, join, dialect));
            }
        }

        return reads;
    }

    /** Render the statement reading the rows, of one batch of parents below a root. */
    statement(parents: Parents | undefined): SQL {
        // select the fields, then the parent value when joined
        const parent =
            parents === undefined
                ? []
                : [sql`${parents.join.column} AS ${sql.identifier("parent")}`];
        const selected = sql.join([...this.#fields, ...parent], sql.raw(", "));

        // read the rows meeting the condition, of the batch's parents when joined
        const where =
            parents === undefined
                ? this.#where
                : sql`${this.#where} AND ${parents.join.filter(parents.values)}`;
        const source = sql`FROM ${this.table}${parents?.join.from ?? sql``} WHERE ${where}`;
        const limit = this.#limit;

        // read every row
        if (limit === undefined) {
            return sql`SELECT ${selected} ${source} ORDER BY ${this.#order}`;
        }
        // read a root's first rows
        else if (parents === undefined) {
            return sql`SELECT ${selected} ${source} ORDER BY ${this.#order} LIMIT ${limit}`;
        }
        // read each parent's first rows, ranked within its parent's rows
        else {
            const rank = sql.identifier("rank");
            const ranked = sql`SELECT ${selected}, ROW_NUMBER() OVER (PARTITION BY ${parents.join.column} ORDER BY ${this.#order}) AS ${rank} ${source}`;

            return sql`SELECT * FROM (${ranked}) AS ${sql.identifier("ranked")} WHERE ${rank} <= ${limit} ORDER BY ${rank}`;
        }
    }

    /** Decode one selected row: its columns, its extras after them, and its parent value last. */
    decode(values: readonly unknown[], join: Join | undefined, dialect: Dialect): Read {
        // decode the columns and the extras
        const offset = this.#columns.length;
        const columns: Row = Object.fromEntries(
            this.#columns.map(([name, column], index) => [
                name,
                decoded(column.definition, values[index], dialect),
            ]),
        );
        const extras = Object.fromEntries(
            this.#extras.map(([name, value], index) => [
                name,
                decoded(value, values[offset + index], dialect),
            ]),
        );

        // name the parent value after the fields
        const parent = join?.parent(values[offset + this.#extras.length], dialect);

        return { columns, row: { ...columns, ...extras }, parent };
    }
}

/** How a level's rows join their parents: the joined table, and the table and column naming each row's parent. */
class Join {
    /** The joined table and its join condition, absent for a key path. */
    readonly from: SQL | undefined;
    /** The table of the column naming the parent. */
    readonly table: Table;
    /** The column naming the parent, by property. */
    readonly name: string;
    /** The column naming the parent. */
    readonly column: Column;

    /** Join through a table's column naming each row's parent. */
    constructor(from: SQL | undefined, table: Table, name: string, column: Column) {
        // keep the join and the parent column
        this.from = from;
        this.table = table;
        this.name = name;
        this.column = column;
    }

    /** Join a level's rows to their parents along a path. */
    static of(table: Table, path: Path): Join {
        // name the parent by the row's key column
        const definition = table[TABLE];
        if (path.kind === "key") {
            return new Join(undefined, table, path.column, definition.column(path.column));
        }
        // name the parent by the junction rows naming the row
        else if (path.kind === "junction") {
            const through = path.table[TABLE];
            const from = sql` INNER JOIN ${path.table} ON ${through.column(path.to.column)} = ${definition.column(path.to.key)}`;

            return new Join(from, path.table, path.from.column, through.column(path.from.column));
        }
        // name the parent by the tree index rows naming the row
        else {
            return Join.#tree(table, path);
        }
    }

    /** Join a level's rows to their parents through a tree's ancestor index. */
    static #tree(table: Table, path: TreePath): Join {
        // name the row on one end of an index row and the parent on the other
        const tree = treeOf(table);
        const index = tree.ancestors[TABLE];
        const row = path.kind === "descendants" ? "descendant" : "ancestor";
        const parent = path.kind === "descendants" ? "ancestor" : "descendant";

        // join the index rows of the row's scope, without the row itself
        const definition = table[TABLE];
        const from = sql` INNER JOIN ${tree.ancestors} ON ${index.column(row)} = ${definition.column(tree.definition.id)}
            AND ${index.column("scope")} = ${definition.column(tree.definition.scope)} AND ${index.column("depth")} > 0`;

        return new Join(from, tree.ancestors, parent, index.column(parent));
    }

    /** Select the rows of some parents. */
    filter(values: readonly Comparable[]): SQL {
        return Condition.render({ [this.name]: { in: values } }, this.table);
    }

    /** Name the parent a selected driver value names. */
    parent(value: unknown, dialect: Dialect): string {
        const definition = this.column.definition;

        return nameOf(definition.toJson(definition.decode(value, dialect)));
    }
}

/** Read one level's rows, for some parent rows below a root, and their included relations. */
async function read(
    connection: DatabaseConnection,
    relations: Relations,
    table: Table,
    options: ReadOptions,
    link?: Link,
): Promise<Read[]> {
    // require a positive whole limit, and read nothing for no parents
    const { limit } = options;
    if (limit !== undefined && (!Number.isInteger(limit) || limit < 1)) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `query limit must be a positive integer: ${limit}`,
        );
    } else if (link?.values.size === 0) {
        return [];
    }

    // read the level's rows
    const level = new Level(relations, table, options);
    const reads = await level.select(connection, link);

    // include each named relation's rows
    for (const [name, selected] of Object.entries(options.with ?? {})) {
        await include(connection, relations, table, reads, name, selected === true ? {} : selected);
    }

    // keep the selected columns, every extra and every include
    return reads.map((entry) => ({
        ...entry,
        row: keep(entry.row, table, options.columns),
    }));
}

/** Set a relation's rows on each read row: a one relation's row or null, or a many relation's rows. */
async function include(
    connection: DatabaseConnection,
    relations: Relations,
    table: Table,
    reads: readonly Read[],
    name: string,
    options: ReadOptions,
): Promise<void> {
    // read the related rows of every distinct parent value at once
    const relation = relations.get(table, name);
    const parent = parentOf(table, relation.on);
    const values = new Map<string, Comparable>();
    for (const entry of reads) {
        const value = parentValue(parent, entry);
        if (value !== undefined) {
            values.set(nameOf(value), value);
        }
    }
    const link = { path: relation.on, values };
    const included = await read(
        connection,
        relations,
        relation.table,
        related(relation, options),
        link,
    );

    // set each row's related rows by its parent value
    const grouped = Map.groupBy(included, (entry) => entry.parent);
    for (const entry of reads) {
        const value = parentValue(parent, entry);
        const found = value === undefined ? undefined : grouped.get(nameOf(value));
        const rows = (found ?? []).map((child) => child.row);
        entry.row[name] = relation.cardinality === "one" ? (rows[0] ?? null) : rows;
    }
}

/** Read a relation's rows by a nested read's options, meeting the relation's condition too. */
function related(relation: Relation, options: ReadOptions): ReadOptions {
    // keep the read's condition
    if (relation.where === undefined) {
        return options;
    }
    // take the relation's condition
    else if (options.where === undefined) {
        return { ...options, where: relation.where };
    }
    // meet both conditions
    else {
        return { ...options, where: { AND: [relation.where, options.where] } };
    }
}

/** Read the column of a level's rows that a path joins their related rows on. */
function parentOf(table: Table, path: Path): Parent {
    // read the key path's parent column
    let name: string;
    if (path.kind === "key") {
        name = path.parent;
    }
    // read the column the junction rows name
    else if (path.kind === "junction") {
        name = path.from.key;
    }
    // read the tree's identifier column
    else {
        name = treeOf(table).definition.id;
    }

    return { name, column: table[TABLE].column(name) };
}

/** Read a row's value of its parent column in JSON form, absent for a row without one. */
function parentValue(parent: Parent, entry: Read): Comparable | undefined {
    const value = entry.columns[parent.name];
    if (value === null || value === undefined) {
        return undefined;
    }

    return Comparable.parse(parent.column.definition.toJson(value));
}

/** Read the tree a table keeps, failing for a table without one. */
function treeOf(table: Table) {
    const tree = table[TABLE].tree;
    if (tree === undefined) {
        throw new DatabaseError("INVALID_QUERY", `${table[TABLE].name} keeps no tree`);
    }

    return tree;
}

/** Keep a row's selected columns, with everything beyond the table's columns. */
function keep(
    row: Record<string, unknown>,
    table: Table,
    selection: Readonly<Record<string, boolean | undefined>> | undefined,
): Record<string, unknown> {
    // keep every column without a selection
    if (selection === undefined) {
        return row;
    }

    // keep the columns set true, or all but those set false
    const columns = table[TABLE].columns;
    const isPicked = Object.values(selection).includes(true);
    const isKept = (name: string) =>
        !Object.hasOwn(columns, name) ||
        (isPicked ? selection[name] === true : selection[name] !== false);

    return Object.fromEntries(Object.entries(row).filter(([name]) => isKept(name)));
}

/** Decode a selected driver value, null as missing. */
function decoded<Value>(decoder: Decoder<Value>, value: unknown, dialect: Dialect): Value | null {
    return value === null ? null : decoder.decode(value, dialect);
}

/** Name a value in JSON form, as parents and their rows match. */
function nameOf(json: unknown): string {
    return JSON.stringify(json);
}

/** Split values into runs of at most a size. */
function batches<Value>(values: readonly Value[], size: number): Value[][] {
    return Array.from({ length: Math.ceil(values.length / size) }, (_, index) =>
        values.slice(index * size, (index + 1) * size),
    );
}
