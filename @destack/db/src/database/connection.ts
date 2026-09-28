import { assertNever, classifyError, DatabaseError } from "../error/error.ts";
import { DatabaseDriver, type NativeDatabase } from "./driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { DrizzleDatabase } from "../dialect/drizzle.ts";
import type { Table } from "../table/table.ts";
import { SelectBuilder, type SelectedSubquery, type SelectQuery } from "../query/select.ts";
import { or, sql, type SQL, type WithSubquery } from "drizzle-orm";
import { MutationQuery } from "../query/mutation.ts";
import type { ResourceState } from "@destack/package/declare";
import {
    declareState,
    readState,
    unappliedTables,
    type DeclareOptions,
} from "../migration/state.ts";
import { planMigration, planStates } from "../migration/database.ts";
import { applyPlan } from "../migration/apply.ts";
import type { TablePlan } from "../migration/plan.ts";
import type { Selection } from "../query/selection.ts";
import { type TransactionOptions, TransactionState } from "./transaction.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";
import { Log } from "../log/log.ts";
import type { CommitNotifier } from "../log/notifier.ts";
import { CommitWatch } from "../log/watch.ts";
import { PARAMETER_BUDGET, type Dialect } from "../dialect/dialect.ts";
import { Key } from "../query/key.ts";
import { CHAIN_TERMS } from "../query/predicate.ts";
import type { Row } from "../table/row.ts";
import { TABLE } from "../table/table.ts";
import { Session } from "../sqlite/session.ts";
import type { PostgresJsSession } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { Sql } from "postgres";

/** Portable queries over one database connection, or one transaction on it. */
export class DatabaseConnection<Driver extends Dialect = Dialect> {
    /** The native Drizzle database and its query lifetime. */
    readonly driver: DatabaseDriver;
    /** The compiler binding logical tables to the native dialect. */
    readonly compiler: SchemaCompiler<Driver>;
    /** The physical connection's submission and shutdown state. */
    readonly state: ConnectionState;

    /** Bind logical tables to a physical connection. */
    constructor(driver: DatabaseDriver, compiler: SchemaCompiler<Driver>) {
        this.state = driver.state;
        this.driver = driver;
        this.compiler = compiler;
    }

    /** The database's SQL dialect. */
    get dialect(): Driver {
        return this.driver.native.dialect as Driver;
    }

    /** The log of the database. */
    get log(): Log {
        return new Log(this);
    }

    /** Select application records or explicit fields. */
    select<Fields extends Selection | undefined = undefined>(
        fields?: Fields,
    ): SelectBuilder<Fields> {
        return new SelectBuilder(this.driver, this.compiler, fields as Fields);
    }

    /** Select distinct application records or explicit fields. */
    selectDistinct<Fields extends Selection | undefined = undefined>(
        fields?: Fields,
    ): SelectBuilder<Fields> {
        return new SelectBuilder(this.driver, this.compiler, fields as Fields, {
            isDistinct: true,
        });
    }

    /** Declare a named common table expression using Drizzle's query description. */
    $with<const Alias extends string>(alias: Alias) {
        return {
            as: <
                Result,
                Fields extends Selection,
                Nullable extends string,
                Automatic extends boolean,
            >(
                query: SelectQuery<Result, Fields, Nullable, Automatic>,
            ): SelectedSubquery<Result, Alias> => {
                // name the query through the connection's native CTE constructor
                const database = this.driver.native.database as unknown as DrizzleDatabase;

                return database.$with(alias).as(query) as unknown as SelectedSubquery<
                    Result,
                    Alias
                >;
            },
        };
    }

    /** Include named common table expressions in the next selection. */
    with(...queries: WithSubquery[]) {
        return {
            select: <Fields extends Selection | undefined = undefined>(fields?: Fields) =>
                new SelectBuilder(this.driver, this.compiler, fields as Fields, {
                    withList: queries,
                }),
            selectDistinct: <Fields extends Selection | undefined = undefined>(fields?: Fields) =>
                new SelectBuilder(this.driver, this.compiler, fields as Fields, {
                    isDistinct: true,
                    withList: queries,
                }),
        };
    }

    /** Insert application records. */
    insert<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "insert"> {
        return new MutationQuery(this.driver, this.compiler, table, "insert");
    }

    /** Update application records. */
    update<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "update"> {
        return new MutationQuery(this.driver, this.compiler, table, "update");
    }

    /** Delete application records. */
    delete<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "delete"> {
        return new MutationQuery(this.driver, this.compiler, table, "delete");
    }

    /**
     * Write rows to a table as they are, inserting new keys and updating held ones, a batch of rows per statement.
     *
     * An upsert fires the log's update trigger on held rows and its insert trigger on new ones.
     */
    async upsert(table: Table, rows: readonly Row[]): Promise<void> {
        // update every written column but the key from the proposed row
        const key = table[TABLE].key;
        const columns = table[TABLE].columns;
        const written = [...new Set(rows.flatMap((row) => Object.keys(row)))];
        const set = Object.fromEntries(
            written
                .filter((name) => !key.includes(name))
                .map((name) => [
                    name,
                    sql`excluded.${sql.identifier(columns[name]!.definition.name)}`,
                ]),
        );

        // write rows with every written column, a batch within the parameter budget
        const size = Math.max(1, Math.floor(PARAMETER_BUDGET / Math.max(written.length, 1)));
        for (let start = 0; start < rows.length; start += size) {
            const batch = rows
                .slice(start, start + size)
                .map((row) => Object.fromEntries(written.map((name) => [name, row[name] ?? null])));
            const insert = this.insert(table).values(batch as never);
            await (Object.keys(set).length === 0
                ? insert.onConflictDoNothing()
                : insert.onConflictDoUpdate({
                      target: key.map((name) => columns[name]!) as never,
                      set: set as never,
                  }));
        }
    }

    /** Delete a table's rows by key, a chain of keys per statement. */
    async remove(table: Table, rows: readonly Row[]): Promise<void> {
        for (let start = 0; start < rows.length; start += CHAIN_TERMS) {
            const matches = rows
                .slice(start, start + CHAIN_TERMS)
                .map((row) => Key.match(table, row));
            await this.delete(table).where(or(...matches)!);
        }
    }

    /** Execute a script of SQL statements in one round trip, without returning rows. */
    async executeScript(script: string): Promise<void> {
        await this.driver.write(
            async (native) => {
                const session = native.database._.session;
                // run SQLite scripts through the session's client
                if (native.dialect === "sqlite" && session instanceof Session) {
                    await session.exec(script);
                }
                // run PostgreSQL scripts as simple queries
                else if (native.dialect === "postgresql") {
                    await (session as PostgresJsSession<Sql, EmptyRelations>).client
                        .unsafe(script)
                        .simple();
                }
                // refuse sessions without a script path
                else {
                    throw new TypeError(`${native.dialect} session cannot run scripts`);
                }
            },
            { isTransaction: true },
        );
    }

    /** Execute explicit SQL and return its driver rows. */
    execute<Row extends Record<string, unknown> = Record<string, unknown>>(
        statement: SQL,
    ): Promise<Row[]> {
        return this.driver.all<Row>(this.driver.render(this.compiler.expression(statement)));
    }

    /** Plan and apply tables at once, for replicas, local stores and tests that own their database. */
    async migrate(tables: readonly Table[], options: DeclareOptions = {}): Promise<TablePlan> {
        const plan = await planMigration(this, declareState(tables, this.dialect, options));
        await this.apply(plan);

        return plan;
    }

    /** Name the tables whose applied state does not hold their declaration, none once a migration applied them. */
    async unapplied(tables: readonly Table[], options: DeclareOptions = {}): Promise<string[]> {
        return unappliedTables(await readState(this), declareState(tables, this.dialect, options));
    }

    /** Plan the union of the desired states of every declaration bound to this database. */
    plan(desired: readonly ResourceState[]): Promise<TablePlan> {
        return planStates(this, desired);
    }

    /** Apply a plan in one transaction and record the declared state. */
    apply(plan: TablePlan): Promise<void> {
        return applyPlan(this, plan);
    }

    /** Commit a callback once, or roll back all its changes on failure. */
    async transaction<Value>(
        operation: (transaction: DatabaseConnection) => Promise<Value>,
        options: TransactionOptions = {},
    ): Promise<Value> {
        // reject use after the enclosing transaction finishes
        this.driver.transaction?.assertActive();

        // propagate cancellation from both the caller and an enclosing transaction
        const signals = [this.driver.transaction?.signal, options.signal].filter(
            (signal): signal is AbortSignal => signal !== undefined,
        );
        const signal = signals.length ? AbortSignal.any(signals) : undefined;
        signal?.throwIfAborted();

        // open the native transaction in the connection's dialect
        const execute = async () => {
            // run a SQLite transaction, or a savepoint within an enclosing one
            if (this.driver.native.dialect === "sqlite") {
                const root = this.driver.native.database;
                const isNested = this.driver.transaction !== undefined;

                return await root.transaction(
                    async (transaction) => {
                        // check foreign keys at commit when asked to
                        if (options.constraints === "deferred") {
                            await transaction.run(sql`PRAGMA defer_foreign_keys = ON`);
                        }

                        // identify the outermost writing transaction to change triggers until just before commit
                        const isMarked =
                            !isNested &&
                            !options.isReadOnly &&
                            (await openTransaction(transaction, this.state));
                        const result = await this.#transact(
                            { dialect: "sqlite", database: transaction },
                            operation,
                            signal,
                        );
                        if (isMarked) {
                            await closeTransaction(transaction);
                        }

                        return result;
                    },
                    { behavior: options.isReadOnly ? "deferred" : "immediate" },
                );
            }
            // run a PostgreSQL transaction at the requested isolation
            else if (this.driver.native.dialect === "postgresql") {
                return await this.driver.native.database.transaction(
                    async (transaction) => {
                        // check deferrable constraints at commit when asked to
                        if (options.constraints === "deferred") {
                            await transaction.execute(sql`SET CONSTRAINTS ALL DEFERRED`);
                        }

                        return this.#transact(
                            { dialect: "postgresql", database: transaction },
                            operation,
                            signal,
                        );
                    },
                    {
                        isolationLevel: options.isolationLevel ?? "repeatable read",
                        accessMode: options.isReadOnly ? "read only" : "read write",
                    },
                );
            }
            // reject other dialects
            else {
                return assertNever(this.driver.native);
            }
        };

        // report a commit that lost to a concurrent transaction as a concurrent update
        try {
            return this.driver.transaction
                ? await this.driver.transaction.run(execute, "report")
                : await this.driver.write(execute, { isTransaction: true });
        } catch (error) {
            throw classifyError(error);
        }
    }

    /** Bind a transaction session and drain its submitted queries before completion. */
    #transact<Value>(
        connection: NativeDatabase,
        operation: (transaction: DatabaseConnection) => Promise<Value>,
        signal?: AbortSignal,
    ): Promise<Value> {
        const state = new TransactionState(signal);
        const transaction = new DatabaseConnection(
            new DatabaseDriver(connection, this.state, state),
            this.compiler,
        );

        return state.execute(() => operation(transaction));
    }
}

/**
 * Where a connection's database runs: in the process, where small queries cost microseconds, or across a network, where each round trip costs more than the query.
 *
 * Callers choose many small queries on an embedded database and one statement on a networked one.
 */
export type Locality = "embedded" | "networked";

/** Submitted operations and shutdown of one physical connection, and the commits it watches for. */
export class ConnectionState {
    /** Where the connection's database runs. */
    readonly locality: Locality;
    /** The commits this connection's readers wait for. */
    readonly commits: CommitWatch;
    /** Whether the database holds a log, which once created never goes away. */
    isLogged = false;
    /** The operations submitted so far: statements and transactions, which measure a workload's round trips. */
    operations = 0;
    /** Operations that must finish before the client closes. */
    readonly #pending = new Set<Promise<void>>();
    /** The shared shutdown operation once closure starts. */
    #closing?: Promise<void>;

    /** Track the work of a new connection to a database running somewhere, exchanging commit notifications with other writers. */
    constructor(locality: Locality, notifier: CommitNotifier) {
        this.locality = locality;
        this.commits = new CommitWatch(notifier);
    }

    /** Submit an operation while this connection accepts work. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // reject work once closing starts
        if (this.#closing) {
            throw new DatabaseError("CONNECTION_CLOSED", "the connection is closing or closed");
        }

        // count the operation, and retain its settlement separately from the result returned to the caller
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

    /** Stop submissions, drain pending work, stop watching commits, and close the client once. */
    close(operation: () => Promise<void>): Promise<void> {
        this.#closing ??= Promise.all(this.#pending)
            .then(() => this.commits.stop())
            .then(operation);

        return this.#closing;
    }
}
