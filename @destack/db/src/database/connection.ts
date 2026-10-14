import { canonicalize, schema } from "@destack/schema";
import { exists, fill, render, type SQL, sql, type SQLWrapper } from "../sql/index.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import { DatabaseDriver } from "./driver.ts";
import type { Session } from "./session.ts";
import { type Insert, type Table, TABLE } from "../table/table.ts";
import { SelectBuilder } from "../query/select.ts";
import { MutationQuery } from "../query/mutation.ts";
import {
    declareState,
    localState,
    namespaceState,
    readEmpty,
    readState,
    readTables,
    unappliedTables,
    type DeclareOptions,
    type TableState,
} from "../migration/state.ts";
import { applyPlan } from "../migration/apply.ts";
import { planTables, type TablePlan } from "../migration/plan.ts";
import { mergeStates, type Merge } from "../migration/merge.ts";
import type { Selection } from "../query/selection.ts";
import { type TransactionOptions, TransactionState } from "./transaction.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";
import { Log } from "../log/log.ts";
import { Transaction } from "../log/commit.ts";
import { type Announcer, CommitWatch } from "../log/watch.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import { Commit, typedChannel, type Channel } from "../channel/channel.ts";
import { PARAMETER_BUDGET, type Dialect } from "../dialect/dialect.ts";
import { Key } from "../query/key.ts";
import { elementValue, jsonElements } from "../query/statement.ts";
import { qualify, relation } from "../table/namespace.ts";
import { Relations } from "../query/relation.ts";
import { RelationalQueryBuilder, type Queries } from "../query/find.ts";
import type { Model } from "../query/model.ts";
import type { Row } from "../table/row.ts";

/** The referential actions that write the referencing rows. */
const WRITING_ACTIONS: ReadonlySet<string> = new Set(["cascade", "set null", "set default"]);

/** Queries over one database connection or transaction. */
export class DatabaseConnection<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The session driver. */
    readonly driver: DatabaseDriver;
    /** The tables the database declares, in declaration order. */
    readonly tables: readonly Table[];
    /** The relations the database declares, which relational reads reference. */
    readonly relations: Relations<Models>;
    /** The physical connection state. */
    readonly state: ConnectionState;
    /** The relational reads, made on first use. */
    #query: Queries<Models> | undefined;

    /** Bind a driver to the declared tables and relations. */
    constructor(driver: DatabaseDriver, tables: readonly Table[], relations: Relations<Models>) {
        // keep the driver's state beside the declaration
        this.state = driver.state;
        this.driver = driver;
        this.tables = tables;
        this.relations = relations;
    }

    /** Read the tables the relations name. */
    get query(): Queries<Models> {
        this.#query ??= RelationalQueryBuilder.of(this, this.relations);

        return this.#query;
    }

    /** Open a named channel to the database's other connections, refusing a sole writer's. */
    channel(name: string): Channel<unknown> {
        const open = this.state.openChannel;
        if (open === undefined) {
            throw new DatabaseError(
                "NO_CHANNEL",
                `the connection is its database's only writer and has no channel ${name}`,
            );
        }

        return open(name);
    }

    /** Decide whether the database keeps a table's rows as a replica, as its declaration lists. */
    replicates(table: Table): boolean {
        return this.state.replicated.has(table[TABLE].sqlName);
    }

    /** The database's SQL dialect. */
    get dialect(): Dialect {
        return this.driver.dialect;
    }

    /** The log of the database. */
    get log(): Log {
        return new Log(this);
    }

    /** Select the source table's records. */
    select(): SelectBuilder;
    /** Select explicit fields. */
    select<Fields extends Selection>(fields: Fields): SelectBuilder<Fields>;
    /**
     * Select records or fields.
     *
     * @construct the builder keeps the fields it is given, and the source's columns without them.
     */
    select(fields?: Selection): SelectBuilder<Selection | undefined> {
        return new SelectBuilder(this.driver, fields, false);
    }

    /** Select the source table's distinct records. */
    selectDistinct(): SelectBuilder;
    /** Select distinct explicit fields. */
    selectDistinct<Fields extends Selection>(fields: Fields): SelectBuilder<Fields>;
    /**
     * Select distinct records or fields.
     *
     * @construct the builder keeps the fields it is given, and the source's columns without them.
     */
    selectDistinct(fields?: Selection): SelectBuilder<Selection | undefined> {
        return new SelectBuilder(this.driver, fields, true);
    }

    /** Insert application records. */
    insert<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "insert"> {
        return MutationQuery.of(this.driver, table, "insert");
    }

    /** Update application records. */
    update<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "update"> {
        return MutationQuery.of(this.driver, table, "update");
    }

    /** Delete application records. */
    delete<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "delete"> {
        return MutationQuery.of(this.driver, table, "delete");
    }

    /** Upsert rows as they are, writing only the columns each row has. */
    async upsert<Definition extends Table>(
        table: Definition,
        rows: readonly Insert<Definition>[],
    ): Promise<void> {
        // group the rows by the columns they have
        const records: readonly Row[] = rows;
        const groups = Map.groupBy(records, (row) => Object.keys(row).toSorted().join());

        // write each group
        for (const group of groups.values()) {
            await this.#upsertAlike(table, group);
        }
    }

    /** Upsert rows that have the same columns, in batches within the parameter budget. */
    async #upsertAlike(table: Table, rows: readonly Row[]): Promise<void> {
        // update the written columns outside the key on conflict
        const definition = table[TABLE];
        const target = definition.key.map((name) => definition.column(name));
        const written = Object.keys(rows[0] ?? {});
        const set = Object.fromEntries(
            written
                .filter((name) => !definition.key.includes(name))
                .map((name) => [
                    name,
                    sql`excluded.${sql.identifier(definition.column(name).definition.name)}`,
                ]),
        );

        // insert each batch
        const size = Math.floor(PARAMETER_BUDGET / written.length);
        for (let start = 0; start < rows.length; start += size) {
            // keep the stored row when only the key is written, or when an append-only table keeps it unchanged
            const insert = this.insert(table).values(rows.slice(start, start + size));
            if (Object.keys(set).length === 0 || definition.isAppendOnly) {
                await insert.onConflictDoNothing();
            }
            // update the other written columns otherwise
            else {
                await insert.onConflictDoUpdate({ target, set });
            }
        }
    }

    /** Read rows by key in one statement, binding the keys as one JSON array. */
    async get<Definition extends Table>(
        table: Definition,
        rows: readonly Key<Definition>[],
    ): Promise<Row[]> {
        // skip the round trip of no keys
        if (rows.length === 0) {
            return [];
        }

        return this.select().from(table).where(this.#keyed(table, rows));
    }

    /** Delete rows by key in one statement, binding the keys as one JSON array. */
    async remove<Definition extends Table>(
        table: Definition,
        rows: readonly Key<Definition>[],
    ): Promise<void> {
        // skip the round trip of no keys
        if (rows.length === 0) {
            return;
        }

        await this.delete(table).where(this.#keyed(table, rows));
    }

    /** Match a table's rows by key against the tuples of one bound JSON array. */
    #keyed(table: Table, rows: readonly Row[]): SQL {
        // match each key column against its value in the JSON array's tuples
        const definition = table[TABLE];
        const columns = definition.key.map((property) => definition.column(property));
        const values = columns.map((column, index) => elementValue(column, index, this.dialect));
        const tuples = sql.param(JSON.stringify(rows.map((row) => Key.tuple(table, row))));

        return sql`(${sql.join(columns, sql`, `)}) IN (SELECT ${sql.join(values, sql`, `)} FROM ${jsonElements(tuples, "wanted")})`;
    }

    /** Execute a SQL script in one round trip as a write of every declared table. */
    async executeScript(script: string): Promise<void> {
        const every = new Set(this.state.cascades.keys());
        await this.driver.commit(() => this.driver.session.exec(script), every);
    }

    /** Run SQL and read its rows by column name, as the database returns them. */
    execute(statement: SQLWrapper): Promise<Record<string, unknown>[]>;
    /** Run SQL and parse each row by a schema. */
    execute<Row>(statement: SQLWrapper, row: schema.Schema<Row>): Promise<Row[]>;
    /**
     * Run SQL and read its rows.
     *
     * @construct a given schema parses each row, else each row is the driver's record by column.
     */
    async execute(statement: SQLWrapper, row?: schema.Schema): Promise<unknown[]> {
        const rows = await this.driver.all(
            fill(render(statement, this.dialect, this.state.namespace)),
        );

        return row === undefined ? rows : rows.map((value) => row.parse(value));
    }

    /** Decide whether a query has rows, in one statement. */
    async exists(query: SQLWrapper): Promise<boolean> {
        // select the existence and decode it as the dialect answers it
        const check = exists(query);
        const [row] = await this.values(sql`SELECT ${check}`);
        if (row === undefined) {
            throw new TypeError("an existence check returned no row");
        }

        return schema.boolean().parse(check.decode(row[0], this.dialect));
    }

    /** Run SQL and read its rows as an array of values. */
    values(statement: SQLWrapper): Promise<unknown[][]> {
        return this.driver.values(fill(render(statement, this.dialect, this.state.namespace)));
    }

    /** Run a SQL write. */
    run(statement: SQLWrapper): Promise<void> {
        return this.driver.execute(fill(render(statement, this.dialect, this.state.namespace)));
    }

    /** Plan and apply tables at once, beside declared states, such as a resource's desired ones. */
    async migrate(
        tables: readonly Table[],
        options: DeclareOptions & {
            /** The states kept beside the tables, such as a resource's desired ones. */
            readonly states?: readonly (readonly TableState[])[];
            /** What happens to the applied tables neither the tables nor the states declare. */
            readonly others?: "keep" | "drop";
        } = {},
    ): Promise<TablePlan> {
        // declare the tables, its replicated tables as replicas
        const declared = declareState(tables, this.dialect, {
            replicated: this.state.replicated,
            ...options,
        });

        // keep the other applied tables as they are when asked, as their declarations had them
        const { namespace } = this.state;
        const names = new Set(declared.map((entry) => namespaceState(entry, namespace).table.name));
        const others =
            options.others === "keep"
                ? (await readState(this))
                      .filter((applied) => !names.has(applied.table.name))
                      .map((applied) => localState(applied, namespace))
                : [];

        // plan the declared tables beside the given states and the kept tables, then apply the plan
        const plan = await this.plan(mergeStates([...(options.states ?? []), others, declared]));
        await this.apply(plan);

        return plan;
    }

    /** List the tables with an unapplied declaration, by name in the database's namespace. */
    async unapplied(tables: readonly Table[], options: DeclareOptions = {}): Promise<string[]> {
        const { namespace } = this.state;
        const declared = declareState(tables, this.dialect, {
            replicated: this.state.replicated,
            ...options,
        });

        return unappliedTables(
            await readState(this),
            declared.map((entry) => namespaceState(entry, namespace)),
        );
    }

    /** Plan the migration from applied to declared tables, within the database's namespace. */
    async plan(state: Pick<Merge, "declared"> & Partial<Merge>): Promise<TablePlan> {
        // read the applied tables a declaration changes that have no rows
        const { namespace } = this.state;
        const applied = await readState(this);
        const declared = state.declared.map((entry) => namespaceState(entry, namespace));
        const changing = declared.flatMap((entry) => {
            const previous = applied.find(
                (each) =>
                    each.table.name === entry.table.name || each.table.name === entry.moved?.table,
            );

            return previous !== undefined && canonicalize(previous) !== canonicalize(entry)
                ? [previous.table.name]
                : [];
        });

        return planTables({
            applied,
            existing: await readTables(this),
            declared,
            empty: await readEmpty(this, changing),
            conflicts: (state.conflicts ?? []).map((conflict) => ({
                ...conflict,
                table: relation(conflict.table, namespace),
            })),
            dialect: this.dialect,
            ...(namespace === undefined ? {} : { namespace }),
        });
    }

    /** Apply a plan in one transaction and record the declared state. */
    apply(plan: TablePlan): Promise<void> {
        return applyPlan(this, plan);
    }

    /** Commit a callback, or roll back on failure or its own rollback, then resolve undefined. */
    async transaction<Value>(
        operation: (transaction: DatabaseConnection<Models>) => Promise<Value>,
        options: TransactionOptions = {},
    ): Promise<Value> {
        // reject use after the enclosing transaction
        this.driver.transaction?.assertActive();

        // combine the caller's and the enclosing signal
        const signals = [this.driver.transaction?.signal, options.signal].filter(
            (signal): signal is AbortSignal => signal !== undefined,
        );
        const signal = signals.length ? AbortSignal.any(signals) : undefined;
        signal?.throwIfAborted();

        // open the transaction or savepoint, deferring constraints and marking the outermost SQLite write for the log
        const isNested = this.driver.transaction !== undefined;
        const written = this.driver.transaction?.written ?? new Set<string>();
        const execute = () =>
            this.driver.session.transaction(
                async (session) => {
                    // defer constraints to the commit when asked
                    if (options.constraints === "deferred") {
                        await session.run(
                            session.dialect === "sqlite"
                                ? "PRAGMA defer_foreign_keys = ON"
                                : "SET CONSTRAINTS ALL DEFERRED",
                            [],
                        );
                    }

                    // mark the outermost SQLite write for the log, and keep its identity in nested ones
                    const marked =
                        session.dialect === "sqlite" && !isNested && options.isReadOnly !== true
                            ? await openTransaction(session, this.state)
                            : undefined;
                    const id = marked ?? this.driver.transaction?.id;
                    const result = await this.#transact(session, operation, written, id, signal);
                    if (marked !== undefined) {
                        await closeTransaction(session, this.state);
                    }

                    return result;
                },
                {
                    isReadOnly: options.isReadOnly ?? false,
                    isolationLevel: options.isolationLevel ?? "repeatable read",
                },
            );

        // report a lost commit as a concurrent update, which the outermost transaction runs again
        const enclosing = this.driver.transaction;
        if (enclosing !== undefined) {
            try {
                return await enclosing.run(execute, "report");
            } catch (error) {
                throw classifyError(error);
            }
        }

        // forget the writes of an attempt that lost, then announce the written tables once committed
        return Transaction.retry(() => {
            written.clear();

            return this.driver.commit(execute, written);
        }, signal);
    }

    /** Bind a transaction session and drain its queries. */
    #transact<Value>(
        session: Session,
        operation: (transaction: DatabaseConnection<Models>) => Promise<Value>,
        written: Set<string>,
        id: string | undefined,
        signal?: AbortSignal,
    ): Promise<Value> {
        const state = new TransactionState(written, id, signal);
        const transaction = new DatabaseConnection(
            new DatabaseDriver(session, this.state, state),
            this.tables,
            this.relations,
        );

        return state.execute(() => operation(transaction));
    }
}

/** Where a connection's database runs: in the process, or across a network. */
export type Locality = "embedded" | "networked";

/** How a physical connection runs: where, over which channels, and who announces commits. */
export interface ConnectionOptions {
    /** Where the connection runs. */
    readonly locality: Locality;
    /** Open a named channel shared with the database's other writers, absent for a sole writer. */
    readonly openChannel: ((name: string) => Channel<unknown>) | undefined;
    /** Who announces the commits of logged tables. */
    readonly announcer: Announcer;
    /** The declared tables. */
    readonly tables: readonly Table[];
    /** The dialect the tables render in. */
    readonly dialect: Dialect;
    /** The SQL names of the tables the database replicates from elsewhere. */
    readonly replicated: ReadonlySet<string>;
    /** The namespace of the database's relations, absent for none. */
    readonly namespace?: string;
}

/** The operations, shutdown and commit watch of one physical connection. */
export class ConnectionState {
    /** Where the connection's database runs. */
    readonly locality: Locality;
    /** The SQL names of the tables the database replicates, empty over bare tables. */
    readonly replicated: ReadonlySet<string>;
    /** The SQL tables a write to each declared table changes, itself first, through cascades. */
    readonly cascades: ReadonlyMap<string, readonly string[]>;
    /** The commits this connection's readers wait for. */
    readonly commits: CommitWatch;
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel: ((name: string) => Channel<unknown>) | undefined;
    /** The database's namespace in a shared SQLite store, absent for a database of its own. */
    readonly namespace: string | undefined;
    /** The searched tables whose full-text rows SQLite writes keep, by SQL name. */
    readonly searched: ReadonlyMap<string, Table>;
    /** Whether the database keeps a log. */
    isLogged = false;
    /** The submitted statements and transactions. */
    operations = 0;
    /** The run statements and transactions, including nested ones. */
    statements = 0;
    /** The operations to finish before close. */
    readonly #pending = new Set<Promise<void>>();
    /** The shutdown. */
    #closing?: Promise<void>;

    /** Create the state of a new connection over declared tables. */
    constructor(options: ConnectionOptions) {
        // keep the channels, the namespace and what writes to the declared tables change
        const { openChannel, tables } = options;
        this.locality = options.locality;
        this.replicated = options.replicated;
        this.openChannel = openChannel;
        this.namespace = options.namespace;
        this.cascades = listCascades(tables, options.dialect);
        this.searched = new Map(
            tables
                .filter((table) => table[TABLE].fullText !== undefined)
                .map((table) => [table[TABLE].sqlName, table]),
        );

        // watch commits on the log channel, announcing those the database leaves unannounced
        const logged = tables
            .filter((table) => table[TABLE].retention !== "none")
            .map((table) => table[TABLE].sqlName);
        this.commits = new CommitWatch(
            openChannel === undefined ? undefined : typedChannel(openChannel(LOG_TOPIC), Commit),
            new Set(options.announcer === "database" ? logged : []),
            [...this.cascades.keys()],
        );
    }

    /** Submit an operation. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // reject work once closing
        if (this.#closing) {
            throw new DatabaseError("CONNECTION_CLOSED", "the connection is closing or closed");
        }

        // count and track the operation
        this.operations += 1;
        const result = Promise.resolve().then(operation);
        const settled = result.then(
            () => {
                this.#pending.delete(settled);
            },
            () => {
                this.#pending.delete(settled);
            },
        );
        this.#pending.add(settled);

        return result;
    }

    /** Stop submissions, drain work, and close the client once. */
    close(operation: () => Promise<void>): Promise<void> {
        this.#closing ??= Promise.all(this.#pending)
            .then(() => this.commits.stop())
            .then(operation);

        return this.#closing;
    }
}

/** List the tables a write to each table changes, itself first, through triggers and actions. */
function listCascades(tables: readonly Table[], dialect: Dialect): Map<string, readonly string[]> {
    // start each table with no changed table
    const direct = new Map<string, Set<string>>();
    for (const table of tables) {
        direct.set(table[TABLE].sqlName, new Set());
    }

    // link each written table to the tables its triggers and referential actions write
    for (const table of tables) {
        const definition = table[TABLE];
        for (const [written, changed] of cascadeLinks(table, dialect)) {
            direct.get(written)?.add(changed);
        }
        if (definition.tree !== undefined) {
            direct.get(definition.sqlName)?.add(definition.tree.ancestors[TABLE].sqlName);
        }
    }

    // close each table's links
    const cascades = new Map<string, readonly string[]>();
    for (const name of direct.keys()) {
        const reached = new Set([name]);
        for (const next of reached) {
            for (const changed of direct.get(next) ?? []) {
                reached.add(changed);
            }
        }
        cascades.set(name, [...reached]);
    }

    return cascades;
}

/** List a table's links from a written table to a table the write changes. */
function cascadeLinks(table: Table, dialect: Dialect): (readonly [string, string])[] {
    // link an aggregate's source to its holder
    const definition = table[TABLE];
    const aggregates = definition.aggregates.map((aggregate) => {
        const source = "from" in aggregate ? aggregate.from() : table;
        const holder = "into" in aggregate ? aggregate.into() : table;

        return [source[TABLE].sqlName, holder[TABLE].sqlName] as const;
    });

    // link the table to the dependents its deletion deletes
    const dependents = definition.dependents
        .filter((dependent) => dependent.onDelete === "cascade")
        .map((dependent) => [definition.sqlName, dependent.from()[TABLE].sqlName] as const);

    // link each referenced table to the table whose rows its writes update or delete
    const references = definition.constraints(dialect).flatMap((constraint) => {
        const isWriting =
            constraint.kind === "foreignKey" &&
            [constraint.actions.onDelete, constraint.actions.onUpdate].some(
                (action) => action !== undefined && WRITING_ACTIONS.has(action),
            );

        return isWriting ? [[constraint.references, definition.sqlName] as const] : [];
    });

    return [...aggregates, ...dependents, ...references];
}

/** Require distinct SQL names across a database's tables and their indexes and keys. */
export function requireDistinct(tables: readonly Table[], dialect: Dialect): void {
    const relations = new Set<string>();
    for (const table of tables) {
        // refuse query aliases and repeated tables
        const definition = table[TABLE];
        if (definition.source !== undefined) {
            throw new TypeError(`query aliases cannot declare SQL tables: ${definition.name}`);
        } else if (relations.has(definition.sqlName)) {
            throw new TypeError(`duplicate SQL table: ${definition.sqlName}`);
        }
        relations.add(definition.sqlName);
    }

    // refuse names shared by tables, indexes and keys
    for (const table of tables) {
        for (const constraint of table[TABLE].constraints(dialect)) {
            const isRelation = ["index", "unique"].includes(constraint.kind);
            if (constraint.name === undefined || !isRelation) {
                continue;
            }
            const name = qualify(table[TABLE].package, constraint.name);
            if (relations.has(name)) {
                throw new TypeError(`duplicate SQL relation name: ${name}`);
            }
            relations.add(name);
        }
    }
}
