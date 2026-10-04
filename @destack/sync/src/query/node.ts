import {
    type Row,
    Key,
    TABLE,
    type Column,
    type ColumnValue,
    type SQL,
    type SQLWrapper,
    type Table,
    Condition,
    Order,
    type OrderBy,
    Predicate,
    type Binding,
    type Extras,
    Namespace,
    type Scalar,
    Expression,
    describeLog,
    DatabaseError,
    sql,
    type Tree,
    type Aggregate,
    type Measure,
    type Path,
    type QueryOptions,
    Relations,
} from "@destack/db";
import { canonicalize, type JsonValue } from "@destack/schema";
import type { Query } from "./query.ts";

/** A relation a node includes or measures: its table and path, and what it selects of the related rows. */
interface Include extends QueryOptions {
    /** The related table. */
    readonly table: Table;
    /** How the related rows join the parent's. */
    readonly on: Path;
}

/** The column kinds sums and averages add up. */
const SUMMED_KINDS: ReadonlySet<string> = new Set(["integer", "real", "bigint"]);

/** One node of a resolved query tree. */
export class Node {
    /** The node's path of names, such as `tasks.comments`. */
    readonly name: string;
    /** The logged table. */
    readonly table: Table;
    /** The scopes the node's rows live in. */
    readonly scopes: Query["scopes"];
    /** The logged columns, by property. */
    readonly columns: Readonly<Record<string, Column>>;
    /** The primary key's properties in key order. */
    readonly key: readonly string[];
    /** The computed values, by name. */
    readonly extras: Extras;
    /** The condition the rows meet, resolved against their fields. */
    readonly where: Predicate;
    /** How the rows sort, as the query writes it. */
    readonly orderBy: OrderBy;
    /** How the rows sort, completed by the primary key. */
    readonly order: Order;
    /** The most rows the node selects per selected parent row, or in all for a root. */
    readonly limit: number | undefined;
    /** Whether the node's windows arrange every candidate. */
    readonly isArranged: boolean;
    /** How the rows relate to the parent node's rows, absent for a root. */
    readonly path: Path | undefined;
    /** The tree index of a descendants or ancestors path. */
    readonly closure: Table | undefined;
    /** The parent node, absent for a root. */
    readonly parent: Node | undefined;
    /** The count of nodes above this one. */
    readonly depth: number;
    /** The included nodes and the join row nodes of junction paths. */
    readonly children: readonly Node[];
    /** What the node's rows are to its parent: included rows, join rows or relation witnesses. */
    readonly kind: "include" | "join" | "relation";
    /** The relations the condition and computed values follow, by name and condition. */
    readonly #relations = new Map<string, Node>();
    /** The relations by name and condition object, forgetting conditions no longer read. */
    readonly #found = new Map<string, WeakMap<Condition, Node>>();
    /** The relations computed values look up. */
    readonly #lookups: ReadonlySet<string>;
    /** The relations and conditions computed values measure. */
    readonly #rollups: ReadonlySet<string>;
    /** The columns the parent node reads through this relation node. */
    readonly #looks = new Set<string>();

    /** The schema's relations, which the node's includes, conditions and computed values name. */
    readonly schema: Relations;

    /** The node's aggregates. */
    readonly aggregate: Aggregate | undefined;
    /** Whether the node selects rows rather than only groups. */
    readonly hasRows: boolean;
    /** The rows in the tree's scopes meeting the condition. */
    readonly #selection: Condition;
    /** The node's selection, resolved on first render. */
    #selected: Predicate | undefined;
    /** The names conditions on the rows compare: the columns and computed values. */
    readonly #fields: ReadonlySet<string>;
    /** The name of what the node selects. */
    readonly selection: string;

    /** Resolve a query or include as a node. */
    constructor(
        name: string,
        query: Query | Include,
        within: Query["scopes"],
        parent?: Node,
        kind: Node["kind"] = "include",
    ) {
        // name relations by the root's schema and read every scope for rows joined by scope
        const table = query.table;
        this.schema =
            parent?.schema ??
            ("relations" in query ? query.relations : undefined) ??
            new Relations();
        const scopes = scopesOf(query, within);
        this.name = name;
        this.kind = kind;
        this.table = table;
        this.scopes = scopes;
        this.columns =
            describeLog(table) === undefined ? table[TABLE].columns : table[TABLE].logged;
        this.key = table[TABLE].key;

        // name the computed values
        this.extras = query.extras ?? {};
        this.#requireUnshadowed();

        // resolve the condition and select the rows meeting it in the tree's scopes
        const condition = query.where ?? {};
        this.#fields = Namespace.fields(table, { extras: this.extras });
        this.where = this.resolve(condition);
        this.#selection = { AND: [Node.scoped(scopes), condition] };
        this.selection = this.#describeSelection(query, condition);

        // order and limit the rows
        this.orderBy = query.orderBy ?? {};
        this.order = Order.complete(Order.of(this.orderBy), table);
        this.limit = limitOf(query, name);
        this.isArranged = query.limit !== undefined && this.#isOrderedByRelations();

        // place the node below its parent along its path
        this.parent = parent;
        this.depth = parent === undefined ? 0 : parent.depth + 1;
        this.path = "on" in query ? query.on : undefined;
        if (this.path !== undefined) {
            this.#requirePath(this.path, this.#parent());
        }
        this.closure = this.#closure();

        // select rows unless measuring them
        this.aggregate = query.aggregate;
        this.hasRows = query.aggregate === undefined && (parent?.hasRows ?? true);

        // resolve the includes and each measured relation once
        const includes = Object.entries(query.with ?? {}).map(([child, selected]) =>
            this.#include(child, selected),
        );
        const lookups = Object.values(this.extras).flatMap((value) => Expression.lookups(value));
        const rollups = Object.values(this.extras).flatMap((value) => Expression.rollups(value));
        const relations = this.#measureRelations(lookups, rollups);

        // note the lookups and rollups and the logged columns they read
        this.#lookups = new Set(lookups.map(({ via }) => via));
        this.#rollups = new Set(rollups.map(({ via, where }) => JSON.stringify([via, where])));
        this.#lookColumns(lookups, rollups);
        this.#requireLookups();

        // add the join row node of each junction include
        this.children = [...includes, ...relations].flatMap((child) => [
            child,
            ...this.#joinOf(child),
        ]);

        // require logged, comparable columns in values, condition and order
        this.#requireColumns();

        // require aggregates alone
        if (query.aggregate !== undefined) {
            this.#requireAggregate(query);
        }
    }

    /** Describe what the node selects as its identity. */
    #describeSelection(query: Query | Include, condition: Condition): string {
        return JSON.stringify([
            this.table[TABLE].sqlName,
            this.extras,
            this.#selection,
            query.orderBy ?? {},
            "on" in query ? describePath(query.on) : null,
            this.#followed(condition),
        ]);
    }

    /** Read the tree index a descendants or ancestors path follows. */
    #closure(): Table | undefined {
        return this.path?.kind === "descendants" || this.path?.kind === "ancestors"
            ? this.#tree().ancestors
            : undefined;
    }

    /** Resolve each relation the condition, lookups and rollups measure once. */
    #measureRelations(
        lookups: ReturnType<typeof Expression.lookups>,
        rollups: ReturnType<typeof Expression.rollups>,
    ): Node[] {
        const uses = measuredRelations(this.where, lookups, rollups);

        return [...uses.entries()].map(([key, use]) => {
            const node = this.#measure(use, rollups);
            this.#relations.set(key, node);

            return node;
        });
    }

    /** Note the logged columns the lookups and rollups read on their relation nodes. */
    #lookColumns(
        lookups: ReturnType<typeof Expression.lookups>,
        rollups: ReturnType<typeof Expression.rollups>,
    ): void {
        for (const { via, column } of lookups) {
            this.relation(via, {}).#look(column);
        }
        for (const rollup of rollups) {
            if (rollup.column !== undefined) {
                this.relation(rollup.via, rollup.where).#look(rollup.column);
            }
        }
    }

    /** Refuse computed values named like the table's columns. */
    #requireUnshadowed(): void {
        const definition = this.table[TABLE];
        const shadowing = Object.keys(this.extras).find((computed) =>
            Object.hasOwn(definition.columns, computed),
        );
        if (shadowing !== undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `computed value ${shadowing} shadows a column of ${definition.name}`,
            );
        }
    }

    /** Decide whether the node sorts by computed values that read related rows. */
    #isOrderedByRelations(): boolean {
        return this.order.some(({ column }) => {
            const expression = this.extras[column];

            return (
                expression !== undefined &&
                (Expression.lookups(expression).length > 0 ||
                    Expression.rollups(expression).length > 0)
            );
        });
    }

    /** Include a named relation's rows, as every row or a selection of them. */
    #include(child: string, selected: true | QueryOptions): Node {
        // join the relation's condition and the selection's
        const relation = this.schema.get(this.table, child);
        const options = selected === true ? {} : selected;
        const where = joinedCondition(relation.where, options.where);

        return new Node(
            `${this.name}.${child}`,
            {
                ...options,
                table: relation.table,
                on: relation.on,
                ...(where === undefined ? {} : { where }),
            },
            this.scopes,
            this,
        );
    }

    /** Resolve a measured relation as an aggregate of its related rows. */
    #measure(use: RelationMeasure, rollups: readonly { readonly via: string }[]): Node {
        // require a key or junction relation to another table
        const { via, where, values } = use;
        const relation = this.schema.get(this.table, via);
        if (relation.on.kind !== "key" && relation.on.kind !== "junction") {
            throw new DatabaseError(
                "INVALID_QUERY",
                `relation ${via} of ${this.name} follows a key or junction path`,
            );
        } else if (relation.table === this.table && rollups.some((rollup) => rollup.via === via)) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `rollup ${via} of ${this.name} measures another table`,
            );
        }

        return new Node(
            relationName(this.name, via, where),
            {
                table: relation.table,
                on: relation.on,
                where: joinedCondition(relation.where, where) ?? where,
                aggregate: { values },
            },
            this.scopes,
            this,
            "relation",
        );
    }

    /** Require each lookup to follow a key path onto another table's single key. */
    #requireLookups(): void {
        for (const via of this.#lookups) {
            const relation = this.relation(via, {});
            const path = relation.#path();
            const isKeyLookup =
                path.kind === "key" &&
                relation.table !== this.table &&
                relation.key.length === 1 &&
                relation.key[0] === path.column;
            if (!isKeyLookup) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `lookup ${via} of ${this.name} follows a key path onto another table's key`,
                );
            }
        }
    }

    /** Build the join row node of a junction include, none for any other child. */
    #joinOf(child: Node): Node[] {
        if (child.path?.kind !== "junction" || child.kind !== "include") {
            return [];
        }
        const on = {
            kind: "key" as const,
            column: child.path.from.column,
            parent: child.path.from.key,
        };

        return [
            new Node(
                `${child.name}#join`,
                { table: child.path.table, on },
                this.scopes,
                this,
                "join",
            ),
        ];
    }

    /** Require logged, comparable columns in the computed values, the condition and the order. */
    #requireColumns(): void {
        // require the computed values, the condition and the order
        const namespace = this.namespace();
        for (const expression of Object.values(this.extras)) {
            Expression.require(expression, this.table, namespace);
            this.#requireLogged(Expression.columns(expression));
        }
        Predicate.require(this.where, this.table, namespace);
        this.#requireLogged(Predicate.fields(this.where));
        Order.require(Order.of(this.orderBy), this.table, namespace);
        this.#requireLogged(new Set(Object.keys(this.orderBy)));
    }

    /** List the columns the node reads. */
    reads(): string[] {
        const names = [
            ...Predicate.fields(this.where),
            ...this.order.map((key) => key.column),
            ...(this.path === undefined ? [] : columnsOf(this.path, "included")),
            ...this.children.flatMap((child) =>
                child.path === undefined ? [] : columnsOf(child.path, "parent"),
            ),
            ...this.grouping,
            ...Object.values(this.aggregate?.values ?? {}).flatMap((measure) =>
                measure.function === "count" ? [] : [measure.column],
            ),
            ...this.#looks,
        ];

        return [...new Set(names.flatMap((name) => this.columnsOf(name)))];
    }

    /** List the logged columns a column or computed value reads. */
    columnsOf(name: string): string[] {
        const expression = this.extras[name];

        return expression === undefined ? [name] : [...Expression.columns(expression)];
    }

    /** List this node and every node below it. */
    nodes(): Node[] {
        return [this, ...this.children.flatMap((child) => child.nodes())];
    }

    /** Name a row by its table and its key's values. */
    keyOf(row: Row): string {
        return Key.name(this.table, row);
    }

    /** The selected row's column with the partition's value, absent for a root. */
    get parentColumn(): string | undefined {
        switch (this.path?.kind) {
            case undefined:
                return undefined;
            case "key":
                return this.path.parent;
            case "junction":
                return this.path.from.key;
            case "descendants":
            case "ancestors":
                return this.#treeKey();
        }
    }

    /** The group field naming the selected row of a group, absent for a root. */
    get partitionName(): string | undefined {
        switch (this.path?.kind) {
            case undefined:
                return undefined;
            case "key":
                return this.path.column;
            case "junction":
                return this.path.from.column;
            case "descendants":
                return "ancestor";
            case "ancestors":
                return "descendant";
        }
    }

    /** Name the group of a parent value's related rows. */
    groupFor(value: ColumnValue | undefined): Record<string, Scalar> {
        const link = this.link();

        return { [link.partitionName]: link.parent.scalar(link.parentColumn, value) };
    }

    /** Read the value naming a selected parent row's partition. */
    valueOf(parent: Row): ColumnValue | undefined {
        return parent[this.link().parentColumn];
    }

    /** Name the partition of a parent value. */
    partition(value: ColumnValue | undefined): string {
        if (this.path === undefined) {
            return "";
        }
        const link = this.link();

        return JSON.stringify(link.parent.scalar(link.parentColumn, value));
    }

    /** Express rows in some scopes as a condition. */
    static scoped(scopes: Query["scopes"]): Condition {
        return scopes === "every" ? {} : { scope: { in: scopes } };
    }

    /** Decide whether a row lives in some scopes. */
    static isScoped(row: Row, scopes: Query["scopes"]): boolean {
        const scope = row["scope"];

        return scopes === "every" || (typeof scope === "string" && scopes.includes(scope));
    }

    /** Read the values a row sorts by, refusing a row without one. */
    orderOf(row: Row): Row {
        const values: Record<string, ColumnValue> = {};
        for (const { column } of this.order) {
            const value = row[column];
            if (value === undefined) {
                throw new TypeError(`${this.name} row has no ${column} to sort by`);
            }
            values[column] = value;
        }

        return values;
    }

    /** Read the logged columns a row's place in the order is decided by, as a cursor keeps them. */
    positionOf(row: Row): Row {
        const values: Record<string, ColumnValue> = {};
        for (const { column } of this.order) {
            for (const name of this.columnsOf(column)) {
                const value = row[name];
                if (value === undefined) {
                    throw new TypeError(`${this.name} row has no ${name} to place it by`);
                }
                values[name] = value;
            }
        }

        return values;
    }

    /** Write a row's logged columns in JSON form without concealed ones. */
    encode(row: Row, concealed: readonly string[]): Record<string, JsonValue> {
        return describeLog(this.table) === undefined
            ? this.table[TABLE].encode(row, concealed)
            : this.table[TABLE].encodeLogged(row, concealed);
    }

    /** Name the closure rows with a tree row at one end. */
    paths(row: Row, end: "ancestor" | "descendant"): Row {
        // read the row's scope and identity
        const tree = this.#tree().definition;
        const scope = row[tree.scope];
        const id = row[tree.id];
        if (scope === undefined || id === undefined) {
            throw new TypeError(`${this.name} row has no tree scope or identity`);
        }

        return { scope, [end]: id };
    }

    /** Read the measures of no rows. */
    emptyValues(): Record<string, Scalar> {
        return Object.fromEntries(
            Object.entries(this.#aggregate().values).map(([name, measure]) => [
                name,
                measure.function === "count" ? 0 : null,
            ]),
        );
    }

    /** Compare two rows by the node's order. */
    compare(left: Row, right: Row): number {
        return Order.rows(this.order, left, right);
    }

    /** Find the node of a relation under a condition. */
    relation(via: string, where: Condition): Node {
        // reuse the relation of the same condition
        const known = this.#found.get(via)?.get(where);
        if (known !== undefined) {
            return known;
        }

        // find it once
        const found = this.#relations.get(JSON.stringify([via, where]));
        if (found === undefined) {
            throw new DatabaseError("INVALID_QUERY", `${this.name} follows no relation ${via}`);
        }
        const byCondition = this.#found.get(via) ?? new WeakMap<Condition, Node>();
        this.#found.set(via, byCondition.set(where, found));

        return found;
    }

    /** Whether computed values read related rows. */
    get isRelating(): boolean {
        return this.#lookups.size > 0 || this.#rollups.size > 0;
    }

    /** The nodes of the relations the condition follows. */
    get relations(): Node[] {
        return [...this.#relations.values()];
    }

    /** Select the node's rows, within one partition when given one. */
    select(partition?: ColumnValue, visibility?: Visibility): SQL {
        // resolve the selection once and a partition's key per render
        this.#selected ??= this.resolve(this.#selection);
        const key = this.#partition(partition);
        const predicate =
            key === undefined ? this.#selected : Predicate.all(this.#selected, this.resolve(key));

        return Predicate.render(predicate, this.bind(visibility));
    }

    /** Bind the node's condition. */
    bind(visibility?: Visibility): Binding<SQLWrapper> {
        return Predicate.bind(this.table, {}, this.namespace(visibility));
    }

    /** Name what the node's condition, order and computed values read beyond its columns. */
    namespace(visibility?: Visibility): Namespace {
        return this.schema.namespace(this.table, this.extras, (table, path) => {
            // read rows of the node's scopes or of every scope through a scope key
            const scopes = path.kind === "key" && path.column === "scope" ? "every" : this.scopes;

            return sql`${Condition.render(Node.scoped(scopes), table)} AND ${visibility?.(table) ?? sql`true`}`;
        });
    }

    /** Note a logged column the parent node reads through this relation node. */
    #look(column: string): void {
        if (!Object.hasOwn(this.columns, column)) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `relation ${this.name} reads no logged column ${column} of ${this.table[TABLE].name}`,
            );
        }
        this.#looks.add(column);
    }

    /** Resolve a condition on the node's rows. */
    resolve(condition: Condition): Predicate {
        return Condition.resolve(condition, this.#fields);
    }

    /** Express the node's rows as a condition, within one partition when given one other than null. */
    condition(partition?: ColumnValue): Condition {
        const key = this.#partition(partition);

        return key === undefined ? this.#selection : { AND: [this.#selection, key] };
    }

    /** Express one partition's key as a condition, absent for every partition. */
    #partition(partition?: ColumnValue): Condition | undefined {
        // select every partition for null or the one partition a key joins
        if (this.path === undefined || partition === undefined || partition === null) {
            return undefined;
        } else if (this.path.kind === "key") {
            return Condition.equal({
                [this.path.column]: this.scalar(this.path.column, partition),
            });
        }

        // refuse partitions of other paths
        throw new TypeError(`a ${this.path.kind} path selects its partitions through other rows`);
    }

    /** The group columns, the key-joined column first for an include. */
    get grouping(): string[] {
        return [
            ...(this.path?.kind === "key" ? [this.path.column] : []),
            ...(this.aggregate?.groupBy ?? []),
        ];
    }

    /** Read a row's group within a partition, in JSON form. */
    groupOf(row: Row, value?: ColumnValue): Record<string, Scalar> {
        // group by the declared columns and by the partition below a root
        const grouped = (this.aggregate?.groupBy ?? []).map((name): [string, Scalar] => [
            name,
            this.scalar(name, row[name]),
        ]);
        if (this.path === undefined) {
            return Object.fromEntries(grouped);
        }
        const link = this.link();
        const partition =
            this.path.kind === "key"
                ? this.scalar(this.path.column, row[this.path.column])
                : link.parent.scalar(link.parentColumn, value);

        return Object.fromEntries([[link.partitionName, partition], ...grouped]);
    }

    /** Write a column or computed value in its scalar JSON form, as groups and partitions compare it. */
    scalar(name: string, value: ColumnValue | undefined): Scalar {
        // write a missing value as null and a computed one as it is without negative zero
        if (value === null || value === undefined) {
            return null;
        }
        const json = Object.hasOwn(this.extras, name)
            ? value
            : this.column(name).definition.toJson(value);
        if ((typeof json === "object" && json !== null) || typeof json === "bigint") {
            throw new TypeError(`${this.name}.${name} is no scalar to group or partition by`);
        }

        return json === 0 ? 0 : json;
    }

    /** Read the partition value a group names, in JSON form, failing for a root. */
    partitionOf(group: Readonly<Record<string, Scalar>>): Scalar {
        const value = group[this.link().partitionName];
        if (value === undefined) {
            throw new TypeError(`${this.name} group names no partition`);
        }

        return value;
    }

    /** Read the parent value a partition's JSON form names, failing for a root. */
    parentValueOf(json: Scalar): ColumnValue {
        const link = this.link();

        return link.parent.fromJson(link.parentColumn, json);
    }

    /** Read a column value from its JSON form. */
    fromJson(name: string, json: Scalar): ColumnValue {
        return json === null || Object.hasOwn(this.extras, name)
            ? json
            : this.column(name).definition.fromJson(json);
    }

    /** Read the kind of a column or computed value. */
    kindOf(name: string): string {
        const expression = this.extras[name];

        return expression === undefined
            ? this.column(name).definition.kind
            : Expression.kind(expression, this.table, this.namespace());
    }

    /** Render a column or computed value in SQL. */
    sql(name: string, visibility?: Visibility): SQLWrapper {
        const expression = this.extras[name];

        return expression === undefined
            ? this.column(name)
            : Expression.render(expression, this.table, this.namespace(visibility));
    }

    /** Read a logged column, failing for one the node does not read. */
    column(name: string): Column {
        const column = this.columns[name];
        if (column === undefined) {
            throw new DatabaseError("INVALID_QUERY", `${this.name} reads no column ${name}`);
        }

        return column;
    }

    /** Read how the node joins its parent, failing for a root. */
    link(): {
        readonly parent: Node;
        readonly parentColumn: string;
        readonly partitionName: string;
    } {
        // require every column of the join
        const parent = this.parent;
        const parentColumn = this.parentColumn;
        const partitionName = this.partitionName;
        if (parent === undefined || parentColumn === undefined || partitionName === undefined) {
            throw new TypeError(`root ${this.name} has no parent`);
        }

        return { parent, parentColumn, partitionName };
    }

    /** Read the parent node, failing for a root. */
    #parent(): Node {
        if (this.parent === undefined) {
            throw new TypeError(`root ${this.name} has no parent`);
        }

        return this.parent;
    }

    /** Read the path to the parent, failing for a root. */
    #path(): Path {
        if (this.path === undefined) {
            throw new TypeError(`root ${this.name} has no path`);
        }

        return this.path;
    }

    /** Read the table's tree, which a descendants or ancestors path requires. */
    #tree(): Tree {
        const tree = this.table[TABLE].tree;
        if (tree === undefined) {
            throw new DatabaseError("INVALID_QUERY", `${this.name} follows a tree its table lacks`);
        }

        return tree;
    }

    /** Read the single key column a tree path follows. */
    #treeKey(): string {
        const [key] = this.key;
        if (key === undefined || this.key.length !== 1) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `${this.name} follows a tree by one key column`,
            );
        }

        return key;
    }

    /** Read the node's aggregate, failing for a node that selects rows. */
    #aggregate(): Aggregate {
        if (this.aggregate === undefined) {
            throw new TypeError(`${this.name} measures no rows`);
        }

        return this.aggregate;
    }

    /** Require logged columns or computed values. */
    #requireLogged(names: ReadonlySet<string>): void {
        for (const name of names) {
            if (!Object.hasOwn(this.columns, name) && !Object.hasOwn(this.extras, name)) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `query column is not logged: ${this.table[TABLE].name}.${name}`,
                );
            }
        }
    }

    /** Require a path over logged columns. */
    #requirePath(path: Path, parent: Node): void {
        // require a key join over logged columns
        if (path.kind === "key") {
            this.#requireLogged(new Set([path.column]));
            parent.#requireLogged(new Set([path.parent]));
        }
        // require a logged join table
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
        // require the selected row's own table with one key column
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

    /** Describe the relations a condition and the computed values follow, as the node's identity names them. */
    #followed(condition: Condition): unknown {
        const followed = new Set([
            ...Predicate.relations(Condition.resolve(condition, this.#fields)).map(
                ({ via }) => via,
            ),
            ...Object.values(this.extras).flatMap((value) => [
                ...Expression.lookups(value).map(({ via }) => via),
                ...Expression.rollups(value).map(({ via }) => via),
            ]),
        ]);

        return [...followed].toSorted().map((via) => {
            const relation = this.schema.get(this.table, via);

            return [
                via,
                relation.table[TABLE].sqlName,
                describePath(relation.on),
                relation.where ?? null,
            ];
        });
    }

    /** Require a valid aggregate. */
    #requireAggregate(query: Query | Include): void {
        // refuse order, limit and includes
        const aggregate = this.#aggregate();
        if (query.with !== undefined || query.orderBy !== undefined || query.limit !== undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `aggregate query holds no rows to order or include: ${this.name}`,
            );
        }

        // group by orderable columns and measure orderable or summed ones
        Order.require(
            (aggregate.groupBy ?? []).map((column) => ({ column, direction: "asc" as const })),
            this.table,
            this.namespace(),
        );
        this.#requireLogged(new Set(aggregate.groupBy ?? []));
        for (const [name, measure] of Object.entries(aggregate.values)) {
            if (measure.function !== "count") {
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

/** The SQL admitting the visible rows of a table. */
export type Visibility = (table: Table) => SQL | undefined;

/** Name a measure of a relation's rows. */
export function measureName(measure: string, column: string): string {
    return `${measure}(${column})`;
}

/** Read the scopes of a node's rows. */
function scopesOf(query: Query | Include, within: Query["scopes"]): Query["scopes"] {
    return "on" in query && query.on.kind === "key" && query.on.column === "scope"
        ? "every"
        : within;
}

/** Read a query's positive integer limit. */
function limitOf(query: Query | Include, name: string): number | undefined {
    if (query.limit !== undefined && (!Number.isInteger(query.limit) || query.limit < 1)) {
        throw new DatabaseError("INVALID_QUERY", `query limit must be a positive integer: ${name}`);
    }

    return query.limit;
}

/** Name the aggregate of a relation under a condition. */
function relationName(name: string, via: string, where: Condition): string {
    return `${name}?${via}${canonicalize(where)}`;
}

/** Describe a path in JSON. */
function describePath(path: Path): unknown {
    return path.kind === "junction" ? { ...path, table: path.table[TABLE].sqlName } : path;
}

/** List the columns a path reads on one side. */
function columnsOf(path: Path, side: "included" | "parent"): string[] {
    const isIncluded = side === "included";
    switch (path.kind) {
        case "key":
            return [isIncluded ? path.column : path.parent];
        case "junction":
            return [isIncluded ? path.to.key : path.from.key];
        case "descendants":
        case "ancestors":
            return isIncluded ? [path.column] : [];
    }
}

/** A relation and condition a node measures, with the measures its uses read. */
interface RelationMeasure {
    /** The relation's name. */
    readonly via: string;
    /** The condition on the related rows. */
    readonly where: Condition;
    /** The measures, by name, a count among them. */
    readonly values: Record<string, Measure>;
}

/** Collect each relation and condition a node's condition, lookups and rollups measure, once each by key. */
function measuredRelations(
    where: Predicate,
    lookups: readonly { readonly via: string; readonly column: string }[],
    rollups: readonly {
        readonly function: Measure["function"];
        readonly via: string;
        readonly column?: string | undefined;
        readonly where: Condition;
    }[],
): Map<string, RelationMeasure> {
    // count each relation's rows and add what a use reads
    const uses = new Map<string, RelationMeasure>();
    const use = (via: string, related: Condition, measured?: [string, Measure]) => {
        // keep one entry per relation and condition
        const key = JSON.stringify([via, related]);
        const entry = uses.get(key) ?? {
            via,
            where: related,
            values: { count: { function: "count" } },
        };
        uses.set(key, entry);
        if (measured !== undefined) {
            entry.values[measured[0]] = measured[1];
        }
    };

    // measure the condition's relations, the lookups' least values and the rollups' measures
    for (const { via, where: related } of Predicate.relations(where)) {
        use(via, related);
    }
    for (const { via, column } of lookups) {
        use(via, {}, [measureName("min", column), { function: "min", column }]);
    }
    for (const { function: measure, via, column, where: related } of rollups) {
        if (measure === "count" || column === undefined) {
            use(via, related);
        } else {
            use(via, related, [measureName(measure, column), { function: measure, column }]);
        }
    }

    return uses;
}

/** Join a relation's condition and a selection's, absent when neither has one. */
function joinedCondition(
    relation: Condition | undefined,
    selected: Condition | undefined,
): Condition | undefined {
    if (relation === undefined || selected === undefined) {
        return relation ?? selected;
    }

    return { AND: [relation, selected] };
}
