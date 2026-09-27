import type { Row } from "@destack/db";
import {
    encodeColumns,
    Key,
    TABLE,
    type Column,
    type JsonValue,
    type SQL,
    type SQLWrapper,
    type Table,
} from "@destack/db";
import {
    Condition,
    Expression,
    Order,
    type Binding,
    type Computed,
    type Namespace,
    type Scalar,
} from "@destack/db/query";
import { describeLog } from "@destack/db/log";
import { DatabaseError } from "@destack/db/error";
import { sql } from "@destack/db";
import { canonicalize } from "@destack/schema/json";
import type { Aggregate, Include, Measure, Path, Query, Relation } from "./query.ts";

/** The column kinds a sum or average adds up. */
const SUMMED_KINDS: ReadonlySet<string> = new Set(["integer", "real", "bigint"]);

/** One node of a resolved query tree: rows of a table, how they join the parent node's rows, and what they include. */
export class Node {
    /** The node's path of names, such as `tasks.comments`. */
    readonly name: string;
    /** The logged table. */
    readonly table: Table;
    /** The scopes the tree's rows live in. */
    readonly scopes: readonly string[];
    /** The logged columns, by property. */
    readonly columns: Readonly<Record<string, Column>>;
    /** The logged columns by property in declaration order, which encoding walks. */
    readonly #logged: readonly (readonly [string, Column])[];
    /** The primary key's properties in key order. */
    readonly key: readonly string[];
    /** The values computed from each row's columns, by name. */
    readonly computed: Computed;
    /** The condition the rows meet. */
    readonly where: Condition | undefined;
    /** How the rows sort, completed by the primary key. */
    readonly order: Order;
    /** The most rows the node holds per held parent row, or in all for a root. */
    readonly limit: number | undefined;
    /** Whether the node's windows arrange every candidate, since its order sorts by values related rows compute. */
    readonly isArranged: boolean;
    /** How the rows relate to the parent node's rows, absent for a root. */
    readonly path: Path | undefined;
    /** The tree index a descendants or ancestors path reads: every ancestor of every row, with its depth. */
    readonly closure: Table | undefined;
    /** The parent node, absent for a root. */
    readonly parent: Node | undefined;
    /** How many nodes lie above this one: zero for a root. */
    readonly depth: number;
    /** The nodes these rows include, and the nodes holding the join rows their junction paths pass through. */
    readonly children: readonly Node[];
    /** What the node's rows are to its parent: included rows, join rows a path passes through, or witnesses of a relation; results leave out all but included rows. */
    readonly kind: "include" | "join" | "relation";
    /** The relations the condition and computed values follow, by their name and condition. */
    readonly #relations = new Map<string, Node>();
    /** The relations found by name and condition object, which conditions and expressions pass as they are. */
    readonly #found = new Map<string, Map<Condition | undefined, Node>>();
    /** The relations computed values look up, each relating one row. */
    readonly #lookups: ReadonlySet<string>;
    /** The relations and conditions computed values measure, by their key. */
    readonly #rollups: ReadonlySet<string>;
    /** The columns the parent node looks up or measures through this relation node. */
    readonly #looks = new Set<string>();

    /** The aggregates the node holds instead of its rows. */
    readonly aggregate: Aggregate | undefined;
    /** Whether the node holds rows: it measures none, and its parent holds rows. */
    readonly isHolding: boolean;
    /** The rows in the tree's scopes meeting the condition, in every partition. */
    readonly #selection: Condition;
    /** The name of the node's table, computed values, rows, order and join column, equal for nodes that select alike. */
    readonly selection: string;

    /** Resolve and validate a query or include as a node of a tree. */
    constructor(
        name: string,
        query: Query | Include,
        scopes: readonly string[],
        parent?: Node,
        kind: Node["kind"] = "include",
    ) {
        // read the table's columns, the logged ones where a log records its changes
        const table = query.table;
        const definition = table[TABLE];
        this.name = name;
        this.kind = kind;
        this.table = table;
        this.scopes = scopes;
        this.columns = describeLog(table) === undefined ? definition.columns : table[TABLE].logged;
        this.#logged = Object.entries(this.columns);
        this.key = table[TABLE].key;

        // name computed values apart from the columns
        this.computed = query.compute ?? {};
        for (const computed of Object.keys(this.computed)) {
            if (Object.hasOwn(definition.columns, computed)) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `computed value ${computed} shadows a column of ${definition.name}`,
                );
            }
        }
        this.where = query.where;
        this.#selection = Condition.all(
            Node.scoped(scopes),
            ...(query.where === undefined ? [] : [query.where]),
        );
        this.selection = JSON.stringify([
            definition.sqlName,
            this.computed,
            this.#selection,
            query.order ?? [],
            "on" in query ? describePath(query.on) : null,
            describeRelations(query.relations),
        ]);
        this.order = Order.complete(query.order ?? [], table);
        if (query.limit !== undefined && (!Number.isInteger(query.limit) || query.limit < 1)) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `query limit must be a positive integer: ${name}`,
            );
        }
        this.limit = query.limit;
        this.isArranged =
            query.limit !== undefined &&
            this.order.some(({ column }) => {
                const expression = this.computed[column];

                return (
                    expression !== undefined &&
                    (Expression.lookups(expression).length > 0 ||
                        Expression.rollups(expression).length > 0)
                );
            });
        this.parent = parent;
        this.depth = parent === undefined ? 0 : parent.depth + 1;
        this.path = "on" in query ? query.on : undefined;
        if (this.path !== undefined) {
            this.#requirePath(this.path, parent!);
        }
        this.closure =
            this.path?.kind === "descendants" || this.path?.kind === "ancestors"
                ? definition.tree!.ancestors
                : undefined;

        // hold rows unless measuring them, below a node holding rows
        this.aggregate = query.aggregate;
        this.isHolding = query.aggregate === undefined && (parent?.isHolding ?? true);

        // resolve the includes and the relations the condition and computed values follow, holding the join rows of junction paths
        const includes = Object.entries(query.include ?? {}).map(
            ([child, include]) => new Node(`${name}.${child}`, include, scopes, this),
        );
        const lookups = Object.values(this.computed).flatMap(Expression.lookups);
        const rollups = Object.values(this.computed).flatMap(Expression.rollups);

        // measure each relation and condition once, counting its rows and measuring what lookups and rollups read of them
        const uses = new Map<
            string,
            { via: string; where: Condition | undefined; values: Record<string, Measure> }
        >();
        const use = (
            via: string,
            where: Condition | undefined,
            measured?: string,
            measure?: Measure,
        ) => {
            // count the relation's rows under the condition, measuring what else a use reads
            const key = JSON.stringify([via, where ?? null]);
            const entry = uses.get(key) ?? { via, where, values: { count: { function: "count" } } };
            uses.set(key, entry);
            if (measured !== undefined) {
                entry.values[measured] = measure!;
            }
        };
        for (const { via, where } of Condition.relations(query.where ?? Condition.all())) {
            use(via, where);
        }
        for (const { via, column } of lookups) {
            use(via, undefined, measureName("min", column), { function: "min", column });
        }
        for (const { function: measure, via, column, where } of rollups) {
            if (measure === "count") {
                use(via, where);
            } else {
                use(via, where, measureName(measure, column!), { function: measure, column });
            }
        }

        // resolve each as an aggregate of the related rows by the value naming their holder, over a key or junction path
        const relations = [...uses.entries()].map(([key, { via, where, values }]) => {
            // require a declared relation over a key or junction path, measuring another table
            const relation = query.relations?.[via];
            if (relation === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `relation ${via} is not declared by ${name}`,
                );
            } else if (relation.on.kind !== "key" && relation.on.kind !== "junction") {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `relation ${via} of ${name} follows a key or junction path`,
                );
            } else if (relation.table === table && rollups.some((rollup) => rollup.via === via)) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `rollup ${via} of ${name} measures another table`,
                );
            }
            const conditions = [relation.where, where].filter(
                (condition): condition is Condition => condition !== undefined,
            );
            const node = new Node(
                relationName(name, via, where),
                {
                    table: relation.table,
                    on: relation.on,
                    ...(conditions.length === 0 ? {} : { where: Condition.all(...conditions) }),
                    ...(relation.relations === undefined ? {} : { relations: relation.relations }),
                    aggregate: { values },
                },
                scopes,
                this,
                "relation",
            );
            this.#relations.set(key, node);

            return node;
        });
        // look up through relations relating one row of another table, and note the columns lookups and rollups read of them
        this.#lookups = new Set(lookups.map(({ via }) => via));
        for (const { via, column } of lookups) {
            this.relation(via, undefined).#looks.add(column);
        }
        this.#rollups = new Set(
            rollups.map(({ via, where }) => JSON.stringify([via, where ?? null])),
        );
        for (const rollup of rollups) {
            if (rollup.column !== undefined) {
                this.relation(rollup.via, rollup.where).#looks.add(rollup.column);
            }
        }
        for (const via of this.#lookups) {
            const relation = this.relation(via, undefined);
            const path = relation.path!;
            if (
                path.kind !== "key" ||
                relation.table === table ||
                relation.key.length !== 1 ||
                relation.key[0] !== path.column
            ) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `lookup ${via} of ${name} follows a key path onto another table's key`,
                );
            }
        }
        this.children = [...includes, ...relations].flatMap((child) => [
            child,
            ...(child.path?.kind === "junction" && child.kind === "include"
                ? [
                      new Node(
                          `${child.name}#join`,
                          {
                              table: child.path.table,
                              on: {
                                  kind: "key",
                                  column: child.path.from.column,
                                  parent: child.path.from.key,
                              },
                          },
                          scopes,
                          this,
                          "join",
                      ),
                  ]
                : []),
        ]);

        // require computed values, the condition and the order over logged, comparable columns, computed values and relations
        const namespace = this.namespace();
        for (const expression of Object.values(this.computed)) {
            Expression.require(expression, table, namespace);
            this.#requireLogged(Expression.columns(expression));
        }
        if (query.where !== undefined) {
            Condition.require(query.where, table, namespace);
            this.#requireLogged(Condition.columns(query.where));
        }
        Order.require(query.order ?? [], table, namespace);
        this.#requireLogged(new Set((query.order ?? []).map((key) => key.column)));

        // require aggregates alone, over orderable or summed columns
        if (query.aggregate !== undefined) {
            this.#requireAggregate(query);
        }
    }

    /** List the columns the node reads to decide and measure its rows: condition, order, joins, groups and measures. */
    reads(): string[] {
        const names = [
            ...(this.where === undefined ? [] : Condition.columns(this.where)),
            ...this.order.map((key) => key.column),
            ...(this.path === undefined ? [] : columnsOf(this.path, "included")),
            ...this.children.flatMap((child) =>
                child.path === undefined ? [] : columnsOf(child.path, "held"),
            ),
            ...this.grouping,
            ...Object.values(this.aggregate?.values ?? {}).flatMap((measure) =>
                measure.column === undefined ? [] : [measure.column],
            ),
            ...this.#looks,
        ];

        return [...new Set(names.flatMap((name) => this.columnsOf(name)))];
    }

    /** List the logged columns a column or computed value reads. */
    columnsOf(name: string): string[] {
        return Object.hasOwn(this.computed, name)
            ? [...Expression.columns(this.computed[name]!)]
            : [name];
    }

    /** List this node and every node below it. */
    nodes(): Node[] {
        return [this, ...this.children.flatMap((child) => child.nodes())];
    }

    /** Name a row by its table and its key's values. */
    keyOf(row: Row): string {
        return Key.name(this.table, row);
    }

    /** The held row's column whose value names the partition of rows it includes, absent for a root. */
    get parentColumn(): string | undefined {
        const path = this.path;

        return path === undefined
            ? undefined
            : path.kind === "key"
              ? path.parent
              : path.kind === "junction"
                ? path.from.key
                : this.key[0]!;
    }

    /** The group field naming the held row whose partition a group measures, absent for a root. */
    get partitionName(): string | undefined {
        const path = this.path;

        return path === undefined
            ? undefined
            : path.kind === "key"
              ? path.column
              : path.kind === "junction"
                ? path.from.column
                : path.kind === "descendants"
                  ? "ancestor"
                  : "descendant";
    }

    /** Name the group a held value's related rows form in the aggregate of a relation or aggregate include. */
    groupFor(value: unknown): Record<string, Scalar> {
        return { [this.partitionName!]: this.parent!.json(this.parentColumn!, value) as Scalar };
    }

    /** Read the value a held row of the parent node names this node's partition by. */
    valueOf(parent: Row): unknown {
        return parent[this.parentColumn!];
    }

    /** Name the partition a held row's value selects, and a root's one partition. */
    partition(value: unknown): string {
        return this.path === undefined
            ? ""
            : JSON.stringify(this.parent!.json(this.parentColumn!, value));
    }

    /** Express a logged table's rows living in some scopes as a condition on its scope column. */
    static scoped(scopes: readonly string[]): Condition {
        return Condition.oneOf("scope", scopes);
    }

    /** Decide whether a logged table's row lives in some scopes. */
    static isScoped(row: Row, scopes: readonly string[]): boolean {
        return scopes.includes(row.scope as string);
    }

    /** Read the values a row sorts by, which an arrangement keeps. */
    orderOf(row: Row): Row {
        const values: Record<string, unknown> = {};
        for (const { column } of this.order) {
            values[column] = row[column];
        }

        return values;
    }

    /** Write a row's logged columns in their JSON form, leaving out the concealed ones. */
    encode(row: Row, concealed: readonly string[]): Record<string, JsonValue> {
        return encodeColumns(this.#logged, row, concealed);
    }

    /** Name the closure rows holding a tree row at one end of their paths: its scope, and its identity at that end. */
    paths(row: Row, end: "ancestor" | "descendant"): Row {
        const tree = this.table[TABLE].tree!.definition;

        return { scope: row[tree.scope], [end]: row[tree.id] };
    }

    /** Read an aggregate node's measures of no rows: a count of zero, and no value for the others. */
    emptyValues(): Record<string, Scalar> {
        return Object.fromEntries(
            Object.entries(this.aggregate!.values).map(([name, measure]) => [
                name,
                measure.function === "count" ? 0 : null,
            ]),
        );
    }

    /** Compare two rows holding their computed values by the node's order. */
    compare(left: Row, right: Row): number {
        return Order.rows(this.order, left, right);
    }

    /** Find the node of a relation the condition follows with a condition, once per condition. */
    relation(via: string, where: Condition | undefined): Node {
        // reuse the relation found for the same condition
        const known = this.#found.get(via)?.get(where);
        if (known !== undefined) {
            return known;
        }

        // find it by name and condition once
        const found = this.#relations.get(JSON.stringify([via, where ?? null]))!;
        const byCondition = this.#found.get(via) ?? new Map<Condition | undefined, Node>();
        this.#found.set(via, byCondition.set(where, found));

        return found;
    }

    /** Whether computed values read related rows, whose values depend on the position read, not on a row's image alone. */
    get isRelating(): boolean {
        return this.#lookups.size > 0 || this.#rollups.size > 0;
    }

    /** The nodes of the relations the condition follows. */
    get relations(): Node[] {
        return [...this.#relations.values()];
    }

    /** Select the node's rows in its scopes meeting its condition, within one partition when given one, relations through the rows visibility. */
    select(partition?: unknown, visibility?: Visibility): SQL {
        return Condition.render(this.condition(partition), this.bind(visibility));
    }

    /** Bind the node's condition to its table's columns, computed values and relations, which follow the rows visibility. */
    bind(visibility?: Visibility): Binding<SQLWrapper> {
        return Condition.bind(this.table, {}, this.namespace(visibility));
    }

    /** Name what the node's condition, order and computed values read beyond its columns, relations through the rows visibility. */
    namespace(visibility?: Visibility): Namespace {
        return {
            computed: this.computed,
            exists: (via, where) => this.relation(via, where).within(visibility),
            lookup: (via, column) => {
                // read the related row's column where its key joins the held row's
                const relation = this.relation(via, undefined);
                const read = relation.columns[column];
                if (read === undefined) {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `relation ${via} looks up no column ${column} of ${relation.table[TABLE].name}`,
                    );
                }

                return {
                    definition: read.definition,
                    value: sql`(SELECT ${read} FROM ${relation.table} WHERE ${relation.joins(this, visibility)}
                        AND ${relation.select(undefined, visibility)} AND ${visibility?.(relation.table) ?? sql`true`})`,
                };
            },
            rollup: (measure, via, column, where) => {
                // measure the related rows the relation matches and the audience admits, joined to the held row
                const relation = this.relation(via, where);
                const read = column === undefined ? undefined : relation.columns[column];
                if (column !== undefined && read === undefined) {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `relation ${via} measures no column ${column} of ${relation.table[TABLE].name}`,
                    );
                }
                const aggregate =
                    measure === "count"
                        ? sql`count(*)`
                        : sql`${sql.raw(measure)}(${measure === "sum" ? read! : Order.text(read!)})`;

                return {
                    ...(read === undefined ? {} : { definition: read.definition }),
                    value: sql`(SELECT ${aggregate} FROM ${relation.table} WHERE ${relation.joins(this, visibility)}
                        AND ${relation.select(undefined, visibility)} AND ${visibility?.(relation.table) ?? sql`true`})`,
                };
            },
        };
    }

    /** Join a relation node's rows to a held row of its parent in SQL: by key, or through the join rows naming them. */
    joins(parent: Node, visibility?: Visibility): SQL {
        const path = this.path!;
        if (path.kind === "key") {
            return sql`${this.columns[path.column]!} = ${parent.columns[path.parent]!}`;
        } else if (path.kind === "junction") {
            const joins = path.table[TABLE].columns;

            return sql`${this.columns[path.to.key]!} IN (SELECT ${joins[path.to.column]!} FROM ${path.table}
                WHERE ${joins[path.from.column]!} = ${parent.columns[path.from.key]!}
                AND ${this.#joinRows(path.table, visibility)})`;
        }

        throw new TypeError(`a ${path.kind} relation has no SQL join`);
    }

    /** Test in SQL whether a relation node holds a related row for its parent's rows: a semi-join on the parent's column. */
    within(visibility?: Visibility): SQL {
        // select the related rows the node matches and the audience admits
        const path = this.path!;
        const held = this.parent!.columns[this.parentColumn!]!;
        const related = (column: string) =>
            sql`SELECT ${this.columns[column]!} FROM ${this.table} WHERE ${this.columns[column]!} IS NOT NULL
                AND ${this.select(undefined, visibility)} AND ${visibility?.(this.table) ?? sql`true`}`;

        // name the held rows' values directly, or through the join rows naming related rows
        let values: SQL;
        if (path.kind === "key") {
            values = related(path.column);
        } else if (path.kind === "junction") {
            const joins = path.table[TABLE].columns;
            values = sql`SELECT ${joins[path.from.column]!} FROM ${path.table} WHERE ${joins[path.from.column]!} IS NOT NULL
                AND ${this.#joinRows(path.table, visibility)}
                AND ${joins[path.to.column]!} IN (${related(path.to.key)})`;
        } else {
            throw new TypeError(`a ${path.kind} relation has no SQL test`);
        }

        return sql`(${held} IS NOT NULL AND ${held} IN (${values}))`;
    }

    /** Match the join rows of a junction path in the node's scopes that the audience admits. */
    #joinRows(table: Table, visibility?: Visibility): SQL {
        const scoped = Condition.render(Node.scoped(this.scopes), Condition.bind(table));

        return sql`${scoped} AND ${visibility?.(table) ?? sql`true`}`;
    }

    /** Express the node's rows in its scopes meeting its condition, within one partition when given one, as a condition. */
    condition(partition?: unknown): Condition {
        // select every partition, or one a key joins
        if (this.path === undefined || partition === undefined) {
            return this.#selection;
        } else if (this.path.kind === "key") {
            return Condition.all(
                this.#selection,
                Condition.eq(this.path.column, this.json(this.path.column, partition) as Scalar),
            );
        }

        // refuse partitions that joins through other rows decide
        throw new TypeError(`a ${this.path.kind} path selects its partitions through other rows`);
    }

    /** The columns whose values form an aggregate's groups, the key-joined column first for an include. */
    get grouping(): string[] {
        return [
            ...(this.path?.kind === "key" ? [this.path.column] : []),
            ...(this.aggregate?.groupBy ?? []),
        ];
    }

    /** Read the group a row holding its computed values belongs to within a partition, in JSON: the partition's held value first for an include. */
    groupOf(row: Row, value?: unknown): Record<string, Scalar> {
        const grouped = (this.aggregate?.groupBy ?? []).map((name) => [
            name,
            this.json(name, row[name]) as Scalar,
        ]);
        const partition =
            this.path === undefined
                ? []
                : [
                      [
                          this.partitionName!,
                          this.path.kind === "key"
                              ? (this.json(this.path.column, row[this.path.column]) as Scalar)
                              : (this.parent!.json(this.parentColumn!, value) as Scalar),
                      ],
                  ];

        return Object.fromEntries([...partition, ...grouped]);
    }

    /** Write a value of a column in its JSON form, and a computed value as it is, zero unsigned as JSON holds it. */
    json(name: string, value: unknown): unknown {
        return value === null || value === undefined
            ? null
            : Object.hasOwn(this.computed, name)
              ? value === 0
                  ? 0
                  : value
              : this.columns[name]!.definition.toJson(value);
    }

    /** Read a value of a column from its JSON form, and a computed value as it is. */
    fromJson(name: string, json: Scalar): unknown {
        return json === null || Object.hasOwn(this.computed, name)
            ? json
            : this.columns[name]!.definition.fromJson(json);
    }

    /** Read the kind of a column or computed value. */
    kindOf(name: string): string {
        return Object.hasOwn(this.computed, name)
            ? Expression.kind(this.computed[name]!, this.table, this.namespace())
            : this.columns[name]!.definition.kind;
    }

    /** Render a column or computed value in SQL, to group and measure by. */
    sql(name: string, visibility?: Visibility): SQLWrapper {
        return Object.hasOwn(this.computed, name)
            ? Expression.render(this.computed[name]!, this.table, this.namespace(visibility))
            : this.columns[name]!;
    }

    /** Require columns among the logged ones or the computed values, since changes carry logged columns only. */
    #requireLogged(names: ReadonlySet<string>): void {
        for (const name of names) {
            if (!Object.hasOwn(this.columns, name) && !Object.hasOwn(this.computed, name)) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `query column is not logged: ${this.table[TABLE].name}.${name}`,
                );
            }
        }
    }

    /** Require a path over logged columns: a logged join table, or a parent column of a table with one key column. */
    #requirePath(path: Path, parent: Node): void {
        // require a key join over logged columns
        if (path.kind === "key") {
            this.#requireLogged(new Set([path.column]));
            parent.#requireLogged(new Set([path.parent]));
        }
        // require a logged join table naming both rows
        else if (path.kind === "junction") {
            const joins = path.table[TABLE];
            const logged = path.table[TABLE].logged;
            if (describeLog(path.table) === undefined) {
                throw new DatabaseError("INVALID_QUERY", `join table is not logged: ${joins.name}`);
            }
            for (const name of [path.from.column, path.to.column]) {
                if (!Object.hasOwn(logged, name)) {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `join column is not logged: ${joins.name}.${name}`,
                    );
                }
            }
            this.#requireLogged(new Set([path.to.key]));
            parent.#requireLogged(new Set([path.from.key]));
        }
        // require the held row's own table, whose tree index follows the parent column from its key
        else if (parent.table !== this.table || this.key.length !== 1) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `a ${path.kind} path follows one table keyed by one column: ${this.name}`,
            );
        } else if (
            this.table[TABLE].tree?.definition.parent !== path.column ||
            this.table[TABLE].tree.definition.id !== this.key[0]
        ) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `a ${path.kind} path follows the tree index of its table's parent column: ${this.name}`,
            );
        } else {
            this.#requireLogged(new Set([path.column]));
        }
    }

    /** Require an aggregate without rows of its own, over logged orderable or summed columns. */
    #requireAggregate(query: Query | Include): void {
        // hold aggregates instead of rows, which nothing else then orders, limits or includes
        const aggregate = query.aggregate!;
        if (query.include !== undefined || query.order !== undefined || query.limit !== undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `aggregate query holds no rows to order or include: ${this.name}`,
            );
        }

        // group by orderable columns or computed values, and measure orderable or summed ones
        Order.require(
            (aggregate.groupBy ?? []).map((column) => ({ column, direction: "asc" as const })),
            this.table,
            this.namespace(),
        );
        this.#requireLogged(new Set(aggregate.groupBy ?? []));
        for (const [name, measure] of Object.entries(aggregate.values)) {
            if ((measure.function === "count") !== (measure.column === undefined)) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `measure ${name} counts rows or measures one column`,
                );
            } else if (measure.column !== undefined) {
                Order.require(
                    [{ column: measure.column, direction: "asc" }],
                    this.table,
                    this.namespace(),
                );
                this.#requireLogged(new Set([measure.column]));
                const kind = this.kindOf(measure.column);
                if (
                    (measure.function === "sum" || measure.function === "avg") &&
                    !SUMMED_KINDS.has(kind)
                ) {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `measure ${name} adds up a non-numeric column`,
                    );
                }
            }
        }
    }
}

/** The SQL admitting the rows of a table a reader sees, absent when it sees every row. */
export type Visibility = (table: Table) => SQL | undefined;

/** Name a measure of a relation's rows: a function of one column. */
export function measureName(measure: string, column: string): string {
    return `${measure}(${column})`;
}

/** Name the aggregate of a relation's rows under a condition, by the node following it. */
function relationName(name: string, via: string, where: Condition | undefined): string {
    return where === undefined ? `${name}?${via}` : `${name}?${via}${canonicalize(where)}`;
}

/** Describe relations in JSON, their tables by SQL name. */
function describeRelations(relations: Readonly<Record<string, Relation>> | undefined): unknown {
    return Object.entries(relations ?? {}).map(([via, relation]) => [
        via,
        relation.table[TABLE].sqlName,
        describePath(relation.on),
        describeRelations(relation.relations),
    ]);
}

/** Describe a path in JSON, a join table by its SQL name. */
function describePath(path: Path): unknown {
    return path.kind === "junction" ? { ...path, table: path.table[TABLE].sqlName } : path;
}

/** List the columns a path reads of the included rows or of the held rows. */
function columnsOf(path: Path, side: "included" | "held"): string[] {
    return path.kind === "key"
        ? [side === "included" ? path.column : path.parent]
        : path.kind === "junction"
          ? [side === "included" ? path.to.key : path.from.key]
          : side === "included"
            ? [path.column]
            : [];
}
