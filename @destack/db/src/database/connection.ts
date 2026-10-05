import { schema } from "@destack/schema";
import { CHAIN_TERMS, exists, fill, or, render, sql, type SQLWrapper } from "../sql/index.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import { DatabaseDriver } from "./driver.ts";
import type { Session } from "./session.ts";
import { type Insert, type Table, TABLE } from "../table/table.ts";
import { SelectBuilder } from "../query/select.ts";
import { MutationQuery } from "../query/mutation.ts";
import {
    declareState,
    namespaceState,
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
import { type Announcer, CommitWatch } from "../log/watch.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import { Commit, typedChannel, type Channel } from "../channel/channel.ts";
import { PARAMETER_BUDGET, type Dialect } from "../dialect/dialect.ts";
import { Key } from "../query/key.ts";
import { qualify, relation } from "../table/namespace.ts";
import { Relations } from "../query/relation.ts";
import { RelationalQueryBuilder, type Queries } from "../query/find.ts";
import type { Model } from "../query/model.ts";
import type { Row } from "../table/row.ts";

/** Queries over one database connection or transaction. */
export class DatabaseConnection<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The session driver. */
    readonly driver: DatabaseDriver;
    /** The tables the database declares, in declaration order. */
    readonly tables: readonly Table[];
    /** The relations the database declares, which relational reads name. */
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

    /** Open a channel of a name to the database's other connections, refusing a sole writer's; its reader types the messages. */
    channel(name: string): Channel<unknown> {
        const open = this.state.openChannel;
        if (open === undefined) {
            throw new DatabaseError(
                "NO_CHANNEL",
                `the connection is its database's sole writer, without a channel ${name}`,
            );
        }

        return open(name);
    }

    /** Decide whether the database keeps a table's rows as copies from their owning service, as its declaration lists them. */
    copies(table: Table): boolean {
        return this.state.copies.has(table[TABLE].sqlName);
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

    /** Upsert rows as they are, writing only the columns each row has, a batch of alike rows per statement. */
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
            // keep the stored row when only the key is written
            const insert = this.insert(table).values(rows.slice(start, start + size));
            if (Object.keys(set).length === 0) {
                await insert.onConflictDoNothing();
            }
            // update the other written columns otherwise
            else {
                await insert.onConflictDoUpdate({ target, set });
            }
        }
    }

    /** Delete rows by key, a chain of keys per statement. */
    async remove<Definition extends Table>(
        table: Definition,
        rows: readonly Key<Definition>[],
    ): Promise<void> {
        // delete each chain of keys
        for (let start = 0; start < rows.length; start += CHAIN_TERMS) {
            const matches = rows
                .slice(start, start + CHAIN_TERMS)
                .map((row) => Key.match(table, row));
            await this.delete(table).where(or(...matches));
        }
    }

    /** Execute a SQL script in one round trip. */
    async executeScript(script: string): Promise<void> {
        await this.driver.commit(() => this.driver.session.exec(script));
    }

    /** Run SQL and read its rows by column name, as the database returns them. */
    execute(statement: SQLWrapper): Promise<Record<string, unknown>[]>;
    /** Run SQL and parse each row by a schema. */
    execute<Row>(statement: SQLWrapper, row: schema.Schema<Row>): Promise<Row[]>;
    /**
     * Run SQL and read its rows.
     *
     * @construct each row is parsed by the schema when one is given, and is the driver's record by column name otherwise.
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

    /** Plan and apply tables at once, beside the tables of declared states, such as a database resource's desired ones. */
    async migrate(
        tables: readonly Table[],
        options: DeclareOptions & { readonly states?: readonly (readonly TableState[])[] } = {},
    ): Promise<TablePlan> {
        // plan the declared tables beside the given states, then apply the plan
        const declared = declareState(tables, this.dialect, options);
        const plan = await this.plan(mergeStates([...(options.states ?? []), declared]));
        await this.apply(plan);

        return plan;
    }

    /** List the tables with an unapplied declaration, by their names within the database's namespace. */
    async unapplied(tables: readonly Table[], options: DeclareOptions = {}): Promise<string[]> {
        const { namespace } = this.state;
        const declared = declareState(tables, this.dialect, options);

        return unappliedTables(
            await readState(this),
            declared.map((entry) => namespaceState(entry, namespace)),
        );
    }

    /** Plan the migration from the applied tables to declared ones, named within the database's namespace. */
    async plan(state: Pick<Merge, "declared"> & Partial<Merge>): Promise<TablePlan> {
        const { namespace } = this.state;

        return planTables({
            applied: await readState(this),
            existing: await readTables(this),
            declared: state.declared.map((entry) => namespaceState(entry, namespace)),
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

    /** Commit a callback, or roll back on failure or once the callback rolls back, then resolving undefined. */
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
                    const isMarked =
                        session.dialect === "sqlite" &&
                        !isNested &&
                        options.isReadOnly !== true &&
                        (await openTransaction(session, this.state));
                    const result = await this.#transact(session, operation, signal);
                    if (isMarked) {
                        await closeTransaction(session, this.state);
                    }

                    return result;
                },
                {
                    isReadOnly: options.isReadOnly ?? false,
                    isolationLevel: options.isolationLevel ?? "repeatable read",
                },
            );

        // report a lost commit as a concurrent update
        try {
            return this.driver.transaction
                ? await this.driver.transaction.run(execute, "report")
                : await this.driver.commit(execute);
        } catch (error) {
            throw classifyError(error);
        }
    }

    /** Run work in a transaction and roll its writes back, returning what it read or planned. */
    async rehearse<Value>(
        operation: (transaction: DatabaseConnection<Models>) => Promise<Value>,
        options: TransactionOptions = {},
    ): Promise<Value> {
        // run the work, keep its result, and unwind the transaction
        let result: { readonly value: Value } | undefined;
        try {
            await this.transaction(async (transaction) => {
                result = { value: await operation(transaction) };
                throw new Rollback();
            }, options);
        } catch (error) {
            if (!(error instanceof Rollback)) {
                throw error;
            }
        }

        // return the kept result
        if (result === undefined) {
            throw new TypeError("a rehearsal ended without its result");
        }

        return result.value;
    }

    /** Bind a transaction session and drain its queries. */
    #transact<Value>(
        session: Session,
        operation: (transaction: DatabaseConnection<Models>) => Promise<Value>,
        signal?: AbortSignal,
    ): Promise<Value> {
        const state = new TransactionState(signal);
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

/** The operations, shutdown and commit watch of one physical connection. */
export class ConnectionState {
    /** Where the connection's database runs. */
    readonly locality: Locality;
    /** The SQL names of the tables the database copies, empty for a connection over bare tables. */
    readonly copies: ReadonlySet<string>;
    /** The commits this connection's readers wait for. */
    readonly commits: CommitWatch;
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel: ((name: string) => Channel<unknown>) | undefined;
    /** The database's namespace in a SQLite store several databases share, absent for a database of its own. */
    readonly namespace: string | undefined;
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

    /** Create the state of a new connection. */
    constructor(
        locality: Locality,
        openChannel: ((name: string) => Channel<unknown>) | undefined,
        announcer: Announcer,
        copies: readonly string[],
        namespace?: string,
    ) {
        // keep the channels and the namespace, and watch commits on the log channel
        this.locality = locality;
        this.copies = new Set(copies);
        this.openChannel = openChannel;
        this.namespace = namespace;
        this.commits = new CommitWatch(
            openChannel === undefined ? undefined : typedChannel(openChannel(LOG_TOPIC), Commit),
            announcer,
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

/** The unwinding of a rehearsed transaction. */
class Rollback extends Error {}

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
