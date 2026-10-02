import { schema } from "@destack/schema";
import { CHAIN_TERMS, fill, or, render, sql, type SQLWrapper } from "../sql/index.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import { DatabaseDriver } from "./driver.ts";
import type { Session } from "./session.ts";
import { type Insert, type Table, TABLE } from "../table/table.ts";
import { DatabaseTier } from "../declare/tier.ts";
import { SelectBuilder } from "../query/select.ts";
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
import { Commit, typedChannel, type Channel, type OpenChannel } from "../channel/channel.ts";
import { PARAMETER_BUDGET, type Dialect } from "../dialect/dialect.ts";
import { Key } from "../query/key.ts";
import { qualify } from "../table/namespace.ts";
import type { Row } from "../table/row.ts";

/** Queries over one database connection or transaction. */
export class DatabaseConnection {
    /** The session driver. */
    readonly driver: DatabaseDriver;
    /** The tables the database declares, in declaration order. */
    readonly tables: readonly Table[];
    /** The physical connection state. */
    readonly state: ConnectionState;

    /** Bind a driver to the declared tables. */
    constructor(driver: DatabaseDriver, tables: readonly Table[]) {
        this.state = driver.state;
        this.driver = driver;
        this.tables = tables;
    }

    /** The tier of the database, absent for a connection over bare tables. */
    get tier(): DatabaseTier | undefined {
        return this.state.tier;
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
    /** Select records or fields, whose signatures above type the builder by them. */
    select(fields?: Selection): SelectBuilder<Selection | undefined> {
        return new SelectBuilder(this.driver, fields, false);
    }

    /** Select the source table's distinct records. */
    selectDistinct(): SelectBuilder;
    /** Select distinct explicit fields. */
    selectDistinct<Fields extends Selection>(fields: Fields): SelectBuilder<Fields>;
    /** Select distinct records or fields, whose signatures above type the builder by them. */
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

    /** Upsert rows as they are, writing only each row's own columns, a batch of alike rows per statement. */
    upsert<Definition extends Table>(
        table: Definition,
        rows: readonly Insert<Definition>[],
    ): Promise<void>;
    /** Upsert rows, whose signature above types them by their table. */
    async upsert(table: Table, rows: readonly Row[]): Promise<void> {
        // group the rows by the columns they have
        const groups = new Map<string, Row[]>();
        for (const row of rows) {
            const columns = Object.keys(row).toSorted().join();
            groups.set(columns, [...(groups.get(columns) ?? []), row]);
        }

        // write each group in batches within the parameter budget
        const definition = table[TABLE];
        const target = definition.key.map((name) => definition.column(name));
        for (const group of groups.values()) {
            const written = Object.keys(group[0] ?? {});
            const set = Object.fromEntries(
                written
                    .filter((name) => !definition.key.includes(name))
                    .map((name) => [
                        name,
                        sql`excluded.${sql.identifier(definition.column(name).definition.name)}`,
                    ]),
            );
            const size = Math.floor(PARAMETER_BUDGET / written.length);
            for (let start = 0; start < group.length; start += size) {
                const insert = this.insert(table).values(group.slice(start, start + size));
                await (Object.keys(set).length === 0
                    ? insert.onConflictDoNothing()
                    : insert.onConflictDoUpdate({ target, set }));
            }
        }
    }

    /** Delete rows by key, a chain of keys per statement. */
    remove<Definition extends Table>(
        table: Definition,
        rows: readonly Key<Definition>[],
    ): Promise<void>;
    /** Delete rows by key, whose signature above types the keys by their table. */
    async remove(table: Table, rows: readonly Key[]): Promise<void> {
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
    /** Run SQL and read its rows, parsed by a schema when given. */
    async execute(statement: SQLWrapper, row?: schema.Schema): Promise<unknown[]> {
        const rows = await this.driver.all(fill(render(statement, this.dialect)));

        return row === undefined ? rows : rows.map((value) => row.parse(value));
    }

    /** Run SQL and read its rows as an array of values. */
    values(statement: SQLWrapper): Promise<unknown[][]> {
        return this.driver.values(fill(render(statement, this.dialect)));
    }

    /** Run a SQL write. */
    run(statement: SQLWrapper): Promise<void> {
        return this.driver.execute(fill(render(statement, this.dialect)));
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
                        await closeTransaction(session);
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
        operation: (transaction: DatabaseConnection) => Promise<Value>,
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
        operation: (transaction: DatabaseConnection) => Promise<Value>,
        signal?: AbortSignal,
    ): Promise<Value> {
        const state = new TransactionState(signal);
        const transaction = new DatabaseConnection(
            new DatabaseDriver(session, this.state, state),
            this.tables,
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
        openChannel: OpenChannel | undefined,
        announcer: Announcer,
        tier?: DatabaseTier,
    ) {
        // keep the channels, and watch commits on the log channel
        this.locality = locality;
        this.tier = tier;
        this.openChannel = openChannel;
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
