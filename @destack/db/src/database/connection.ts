import { assertNever, classifyError, DatabaseError } from "../error/error.ts";
import { DatabaseDriver, type NativeDatabase } from "./driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { DrizzleDatabase } from "../dialect/drizzle.ts";
import type { Table } from "../table/table.ts";
import { DatabaseTier } from "../declare/tier.ts";
import { SelectBuilder, type SelectedSubquery, type SelectQuery } from "../query/select.ts";
import { or, sql, type SQL, type WithSubquery } from "drizzle-orm";
import { MutationQuery } from "../query/mutation.ts";
import {
    declareState,
    readState,
    readTables,
    unappliedTables,
    type DeclareOptions,
    type TableState,
} from "../migration/state.ts";
import { applyPlan } from "../migration/apply.ts";
import { planTables, type TablePlan } from "../migration/plan.ts";
import { mergeStates, type MergedState } from "../migration/merge.ts";
import type { Selection } from "../query/selection.ts";
import { type TransactionOptions, TransactionState } from "./transaction.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";
import { Log } from "../log/log.ts";
import { type Announcer, CommitWatch } from "../log/watch.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import type { Channel, OpenChannel } from "../channel/channel.ts";
import { PARAMETER_BUDGET, type Dialect } from "../dialect/dialect.ts";
import { Key } from "../query/key.ts";
import { CHAIN_TERMS } from "../query/predicate.ts";
import type { Row } from "../table/row.ts";
import { TABLE } from "../table/table.ts";
import { Session } from "../sqlite/session.ts";
import type { PostgresJsSession } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { Sql } from "postgres";

/** Queries over one database connection or transaction. */
export class DatabaseConnection<Driver extends Dialect = Dialect> {
    /** The native Drizzle database. */
    readonly driver: DatabaseDriver;
    /** The table compiler. */
    readonly compiler: SchemaCompiler<Driver>;
    /** The physical connection state. */
    readonly state: ConnectionState;

    /** Bind a driver and compiler. */
    constructor(driver: DatabaseDriver, compiler: SchemaCompiler<Driver>) {
        this.state = driver.state;
        this.driver = driver;
        this.compiler = compiler;
    }

    /** The tier of the database, absent for a connection over bare tables. */
    get tier(): DatabaseTier | undefined {
        return this.state.tier;
    }

    /** Open a channel of a name to the database's other connections, refusing a sole writer's. */
    channel<Message>(name: string): Channel<Message> {
        const open = this.state.openChannel;
        if (open === undefined) {
            throw new DatabaseError(
                "NO_CHANNEL",
                `the connection is its database's sole writer, without a channel ${name}`,
            );
        }

        return open(name);
    }

    /** The tables the database declares, in declaration order. */
    get tables(): readonly Table[] {
        return this.compiler.declared;
    }

    /** Decide whether the database keeps a table's rows as copies from their home: the tables of a wider tier than its own. */
    copies(table: Table): boolean {
        const tiers = DatabaseTier.options;
        const declared = table[TABLE].tier;

        return (
            this.tier !== undefined &&
            declared !== undefined &&
            tiers.indexOf(declared) < tiers.indexOf(this.tier)
        );
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

    /** Declare a named common table expression. */
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
                // build the CTE on the native database
                const database = this.driver.native.database as unknown as DrizzleDatabase;

                return database.$with(alias).as(query) as unknown as SelectedSubquery<
                    Result,
                    Alias
                >;
            },
        };
    }

    /** Include common table expressions in the next selection. */
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

    /** Upsert rows, a batch per statement. */
    async upsert(table: Table, rows: readonly Row[]): Promise<void> {
        // update every written column but the key
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

        // write batches within the parameter budget
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

    /** Delete rows by key, a chain of keys per statement. */
    async remove(table: Table, rows: readonly Row[]): Promise<void> {
        for (let start = 0; start < rows.length; start += CHAIN_TERMS) {
            const matches = rows
                .slice(start, start + CHAIN_TERMS)
                .map((row) => Key.match(table, row));
            await this.delete(table).where(or(...matches)!);
        }
    }

    /** Execute a SQL script in one round trip. */
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
                // refuse other sessions
                else {
                    throw new TypeError(`${native.dialect} session cannot run scripts`);
                }
            },
            { isTransaction: true },
        );
    }

    /** Execute SQL and return its rows. */
    execute<Row extends Record<string, unknown> = Record<string, unknown>>(
        statement: SQL,
    ): Promise<Row[]> {
        return this.driver.all<Row>(this.driver.render(this.compiler.expression(statement)));
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

    /** List the tables with an unapplied declaration. */
    async unapplied(tables: readonly Table[], options: DeclareOptions = {}): Promise<string[]> {
        return unappliedTables(await readState(this), declareState(tables, this.dialect, options));
    }

    /** Plan the migration from the applied tables to declared ones. */
    async plan(state: Pick<MergedState, "declared"> & Partial<MergedState>): Promise<TablePlan> {
        return planTables({
            applied: await readState(this),
            existing: await readTables(this),
            declared: state.declared,
            conflicts: state.conflicts ?? [],
            dialect: this.dialect,
        });
    }

    /** Apply a plan in one transaction and record the declared state. */
    apply(plan: TablePlan): Promise<void> {
        return applyPlan(this, plan);
    }

    /** Commit a callback, or roll back on failure or once the callback rolls back, then resolving undefined. */
    async transaction<Value>(
        operation: (transaction: DatabaseConnection) => Promise<Value>,
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

        // open the native transaction
        const execute = async () => {
            // run a SQLite transaction or savepoint
            if (this.driver.native.dialect === "sqlite") {
                const root = this.driver.native.database;
                const isNested = this.driver.transaction !== undefined;

                return await root.transaction(
                    async (transaction) => {
                        // defer foreign keys when asked
                        if (options.constraints === "deferred") {
                            await transaction.run(sql`PRAGMA defer_foreign_keys = ON`);
                        }

                        // mark the outermost writing transaction for the log
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
            // run a PostgreSQL transaction
            else if (this.driver.native.dialect === "postgresql") {
                return await this.driver.native.database.transaction(
                    async (transaction) => {
                        // defer constraints when asked
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

        // report a lost commit as a concurrent update, and resolve nothing once the callback rolled back
        try {
            return this.driver.transaction
                ? await this.driver.transaction.run(execute, "report")
                : await this.driver.write(execute, { isTransaction: true });
        } catch (error) {
            if (error instanceof Rollback) {
                return undefined as Value;
            }
            throw classifyError(error);
        }
    }

    /** Roll back the transaction this connection runs, ending its callback. */
    rollback(): never {
        if (this.driver.transaction === undefined) {
            throw new TypeError("only a transaction rolls back");
        }

        throw new Rollback();
    }

    /** Bind a transaction session and drain its queries. */
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

/** Where a connection's database runs: in the process, or across a network. */
export type Locality = "embedded" | "networked";

/** The operations, shutdown and commit watch of one physical connection. */
export class ConnectionState {
    /** Where the connection's database runs. */
    readonly locality: Locality;
    /** The tier of the database, absent for a connection over bare tables. */
    readonly tier: DatabaseTier | undefined;
    /** The commits this connection's readers wait for. */
    readonly commits: CommitWatch;
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel: OpenChannel | undefined;
    /** Whether the database holds a log. */
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
        openChannel: OpenChannel | undefined,
        announcer: Announcer,
        tier?: DatabaseTier,
    ) {
        // keep the channels, and watch commits on the log channel
        this.locality = locality;
        this.tier = tier;
        this.openChannel = openChannel;
        this.commits = new CommitWatch(openChannel?.(LOG_TOPIC), announcer);
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

/** The unwinding of a transaction its callback rolls back. */
class Rollback extends Error {}
