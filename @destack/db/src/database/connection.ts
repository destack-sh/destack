import type { Clock, Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { exists, fill, render, type SQL, sql, type SQLWrapper } from "../sql/index.ts";
import { classifyError, DatabaseError, type DeclaredNames } from "../error/error.ts";
import { DatabaseDriver, OperationDuration } from "./driver.ts";
import type { ColumnCipher } from "./cipher.ts";
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
import { changedTables, planTables, type TablePlan } from "../migration/plan.ts";
import { mergeStates, type Merge } from "../migration/merge.ts";
import type { Selection } from "../query/selection.ts";
import { type TransactionOptions, TransactionState } from "./transaction.ts";
import { TransactionIdentity } from "../log/transaction.ts";
import { Log } from "../log/log.ts";
import { Transaction } from "../log/commit.ts";
import { type Announcer, CommitWatch } from "../log/watch.ts";
import { LOG_TABLES, LOG_TOPIC } from "../log/schema.ts";
import { ReadCache } from "../log/cache.ts";
import { Commit, typedChannel, type Channel } from "../channel/channel.ts";
import { Tracker } from "../tracker/tracker.ts";
import { PARAMETER_BUDGET, type Dialect } from "../dialect/dialect.ts";
import { Key } from "../query/key.ts";
import { elementValue, jsonElements } from "../query/statement.ts";
import { qualify, relation } from "../table/namespace.ts";
import { Relations } from "../query/relation.ts";
import { ModelQueryBuilder, type Queries } from "../query/find.ts";
import type { Model } from "../query/model.ts";
import type { Row } from "../table/row.ts";

/** The channel a database's connections track their live instances and owners on. */
const TRACKER_CHANNEL = "tracker";

/** The weight each read cache of a database stores, in rows: 2 MB of 0.5 KB rows, 4 MB for both. */
const CACHED_ROWS = 4096;

/** The most rows one cached read has: a metadata read has a few dozen, a listing page 100. */
const CACHED_READ_ROWS = 128;

/** The key bytes weighing as much as one row, the half kilobyte a row averages. */
const KEY_BYTES_PER_ROW = 512;

/** The referential actions that write the referencing rows. */
const WRITING_ACTIONS: ReadonlySet<string> = new Set(["cascade", "set null", "set default"]);

/** The write operations whose cascades differ, as statements' leading verbs have them. */
const WRITE_OPERATIONS = ["INSERT", "UPDATE", "DELETE"] as const;

/** A write operation, a delete standing in for an unknown one as it cascades furthest. */
export type WriteOperation = (typeof WRITE_OPERATIONS)[number];

/** A write of one table making a write of another through a trigger or referential action. */
interface CascadeLink {
    /** The written table's SQL name. */
    readonly written: string;
    /** The operations on it that make the write. */
    readonly operations: readonly WriteOperation[];
    /** The changed table's SQL name. */
    readonly changed: string;
    /** The operation the changed table takes. */
    readonly operation: WriteOperation;
}

/** Queries over one database connection or transaction. */
export class DatabaseConnection<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The session driver. */
    readonly driver: DatabaseDriver;
    /** The tables the database declares, in declaration order. */
    readonly tables: readonly Table[];
    /** The relations the database declares, which relational reads follow. */
    readonly relations: Relations<Models>;
    /** The physical connection state. */
    readonly state: ConnectionState;
    /** The relational reads, made on first use. */
    #query: Queries<Models> | undefined;

    /** Bind a driver to the declared tables and relations. */
    constructor(driver: DatabaseDriver, tables: readonly Table[], relations: Relations<Models>) {
        // store the driver's state beside the declaration
        this.state = driver.state;
        this.driver = driver;
        this.tables = tables;
        this.relations = relations;
    }

    /** Open a connection on the same session that counts its own operations. */
    metered(): DatabaseConnection<Models> {
        const driver = new DatabaseDriver(this.driver.session, this.state, this.driver.transaction);

        return new DatabaseConnection(driver, this.tables, this.relations);
    }

    /** Read the tables the relations relate. */
    get query(): Queries<Models> {
        this.#query ??= ModelQueryBuilder.of(this, this.relations);

        return this.#query;
    }

    /** Open a named channel to the database's other connections, refusing a sole writer's. */
    channel(name: string): Channel<unknown> {
        const open = this.state.openChannel;
        if (open === undefined) {
            throw DatabaseError.of(
                "CHANNEL_MISSING",
                `the connection is its database's only writer and has no channel ${name}`,
            );
        }

        return open(name);
    }

    /** Whether other connections write the database beside this one, sharing its channel. */
    get isShared(): boolean {
        return this.state.openChannel !== undefined;
    }

    /** The tracker of the database's live instances and owners. */
    get tracker(): Tracker {
        return this.state.tracker();
    }

    /** Decide whether the database stores a table's rows as a replica, as its declaration lists. */
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

    /** Select explicit fields, or the source table's records without them. */
    select<Fields extends Selection | undefined = undefined>(
        fields?: Fields,
    ): SelectBuilder<Fields> {
        return new SelectBuilder(this.driver, fields, false);
    }

    /** Select distinct explicit fields, or the source table's distinct records without them. */
    selectDistinct<Fields extends Selection | undefined = undefined>(
        fields?: Fields,
    ): SelectBuilder<Fields> {
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
            // leave the stored row when only the key or an unchanged append-only row is written
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

    /** Key the database's sensitive columns once, as SQLCipher's `PRAGMA key` does. */
    key(cipher: ColumnCipher): void {
        // refuse a second key, which would mix the sealing of stored values
        if (this.state.cipher !== undefined && this.state.cipher !== cipher) {
            throw new TypeError("the database already has a key");
        }
        this.state.cipher = cipher;
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

    /** Execute a script in one round trip: DDL and pragmas alone, or a transaction's writes. */
    async executeScript(script: string): Promise<void> {
        // stamp an enclosing transaction, then commit the script as a write of every table
        const every = new Set(this.state.declared.keys());
        await this.driver.transaction?.identity.stamp();
        await this.driver.commit(() => this.driver.session.exec(script), every, every);
    }

    /** Run SQL and parse each row by a schema. */
    async execute<Parsed>(statement: SQLWrapper, row: schema.Schema<Parsed>): Promise<Parsed[]> {
        return (await this.all(statement)).map((value) => row.parse(value));
    }

    /** Run SQL and read its rows by column name, as the database returns them. */
    all(statement: SQLWrapper): Promise<Record<string, unknown>[]> {
        return this.driver.all(
            fill(render(statement, this.dialect, this.state.namespace, this.state.cipher)),
        );
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
        return this.driver.values(
            fill(render(statement, this.dialect, this.state.namespace, this.state.cipher)),
        );
    }

    /** Run a SQL write. */
    run(statement: SQLWrapper): Promise<void> {
        return this.driver.execute(
            fill(render(statement, this.dialect, this.state.namespace, this.state.cipher)),
        );
    }

    /** Plan and apply tables at once, beside declared states, such as a resource's desired ones. */
    async migrate(
        tables: readonly Table[],
        options: DeclareOptions & {
            /** The states stored beside the tables, such as a resource's desired ones. */
            readonly states?: readonly (readonly TableState[])[];
            /** What happens to the applied tables neither the tables nor the states declare. */
            readonly others?: "retain" | "drop";
        } = {},
    ): Promise<TablePlan> {
        // declare the tables, its replicated tables as replicas
        const declared = declareState(tables, this.dialect, {
            replicated: this.state.replicated,
            ...options,
        });

        // retain the other applied tables as they are when asked, as their declarations had them
        const { namespace } = this.state;
        const names = new Set(declared.map((entry) => namespaceState(entry, namespace).table.name));
        const others =
            options.others === "retain"
                ? (await readState(this))
                      .filter((applied) => !names.has(applied.table.name))
                      .map((applied) => localState(applied, namespace))
                : [];

        // plan the declared, given and retained tables, then apply the plan
        const merged = mergeStates([...(options.states ?? []), others, declared]);
        const plan = await this.plan(merged);
        await this.apply(plan);
        this.state.declare(tables);

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
        const changed = changedTables(applied, declared);

        return planTables({
            applied,
            existing: await readTables(this),
            declared,
            changed,
            empty: await readEmpty(this, [...changed]),
            conflicts: (state.conflicts ?? []).map((conflict) => ({
                ...conflict,
                table: relation(conflict.table, namespace),
            })),
            dialect: this.dialect,
            ...(namespace === undefined ? {} : { namespace }),
        });
    }

    /** Apply a plan in one transaction and record the declared state. */
    async apply(plan: TablePlan): Promise<void> {
        await applyPlan(this, plan);
    }

    /** Commit a callback, or roll back on failure or on its rollback, then resolve undefined. */
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

        // open the transaction, or a savepoint sharing the outermost one's writes and identity
        const isNested = this.driver.transaction !== undefined;
        const written = this.driver.transaction?.written ?? new Set<string>();
        const scripted = this.driver.transaction?.scripted ?? new Set<string>();
        let attempt: TransactionIdentity | undefined;
        const execute = () =>
            this.driver.session.transaction(
                async (session) => {
                    // defer constraints to the commit when asked, sent ahead of the work
                    const deferring =
                        options.constraints === "deferred"
                            ? session.run(
                                  session.dialect === "sqlite"
                                      ? "PRAGMA defer_foreign_keys = ON"
                                      : "SET CONSTRAINTS ALL DEFERRED",
                                  [],
                              )
                            : undefined;

                    // give the outermost transaction its log identity, stamped at its first write
                    const identity =
                        this.driver.transaction?.identity ??
                        new TransactionIdentity(session, this.state);
                    attempt = isNested ? undefined : identity;
                    const isStamped = identity.isStamped;

                    // run the work, forgetting a stamp its rolled back savepoint undid
                    const result = await this.#transact(
                        session,
                        operation,
                        written,
                        scripted,
                        identity,
                        signal,
                    ).catch(async (error: unknown) => {
                        await deferring;
                        if (isNested && !isStamped) {
                            identity.unstamp();
                        }
                        throw error;
                    });
                    await deferring;
                    if (!isNested) {
                        await identity.close();
                    }

                    return result;
                },
                {
                    isReadOnly: options.isReadOnly ?? false,
                    isolationLevel: options.isolationLevel ?? "repeatable read",
                },
            );

        // classify a nested failure, which the outermost transaction runs again on a lost race
        const enclosing = this.driver.transaction;
        if (enclosing !== undefined) {
            try {
                return await enclosing.run(execute, "report");
            } catch (error) {
                throw classifyError(error, this.state.declared);
            }
        }

        // forget a lost attempt's writes and token, then announce the written tables once committed
        return Transaction.retry(
            this,
            "SERIALIZATION_FAILURE",
            async () => {
                written.clear();
                scripted.clear();
                try {
                    const committed = await this.driver.commit(execute, written, scripted);
                    attempt?.commit();

                    return committed;
                } catch (error) {
                    attempt?.abandon();
                    throw error;
                }
            },
            signal,
        );
    }

    /** Bind a transaction session and drain its queries. */
    #transact<Value>(
        session: Session,
        operation: (transaction: DatabaseConnection<Models>) => Promise<Value>,
        written: Set<string>,
        scripted: Set<string>,
        identity: TransactionIdentity,
        signal?: AbortSignal,
    ): Promise<Value> {
        const state = new TransactionState(written, scripted, identity, signal);
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

/** The effects a connection runs on: its clock, randomness, crypto and telemetry. */
export type DatabaseEnvironment = Pick<Environment, "clock" | "crypto" | "random" | "telemetry">;

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
    /** The clock, randomness, crypto and telemetry the connection runs on. */
    readonly environment: DatabaseEnvironment;
}

/** The operations, shutdown and commit watch of one physical connection. */
export class ConnectionState {
    /** Where the connection's database runs. */
    readonly locality: Locality;
    /** The clock, randomness, crypto and telemetry the connection runs on. */
    readonly environment: DatabaseEnvironment;
    /** The histogram of each statement's duration on the environment's telemetry. */
    readonly durations: OperationDuration;
    /** The SQL names of the tables the database replicates, empty over bare tables. */
    readonly replicated: ReadonlySet<string>;
    /** The tables each operation on each declared table changes, itself first, through cascades. */
    #cascades: ReadonlyMap<WriteOperation, ReadonlyMap<string, readonly string[]>> = new Map();
    /** The SQL names of the declared tables whose writes fire aggregate triggers. */
    #aggregated: ReadonlySet<string> = new Set();
    /** The SQL names of the declared tables whose deletes fire dependent triggers. */
    #dependent: ReadonlySet<string> = new Set();
    /** The commits this connection's readers wait for. */
    readonly commits: CommitWatch;
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel: ((name: string) => Channel<unknown>) | undefined;
    /** The database's namespace in a shared SQLite store, absent for a separate database. */
    readonly namespace: string | undefined;
    /** The searched tables whose full-text rows SQLite writes update, by SQL name. */
    #searched: ReadonlyMap<string, Table> = new Map();
    /** The declared names of the tables and their columns, by SQL name. */
    #declared: DeclaredNames = new Map();
    /** The SQL names of the tables without change triggers: unlogged ones and the log's tables. */
    readonly unlogged = new Set<string>(LOG_TABLES);
    /** The declared tables, by SQL name. */
    readonly #tables = new Map<string, Table>();
    /** The dialect the tables render in. */
    readonly #dialect: Dialect;
    /** Whether the database announces its logged tables' commits itself. */
    readonly #isAnnouncing: boolean;
    /** The cache of live `values` reads, by statement, each at its tables' commit versions. */
    readonly valueCache = new ReadCache<unknown[][]>(CACHED_ROWS, CACHED_READ_ROWS, weigh);
    /** The cache of live `all` reads, by statement, each at its tables' commit versions. */
    readonly recordCache = new ReadCache<Record<string, unknown>[]>(
        CACHED_ROWS,
        CACHED_READ_ROWS,
        weigh,
    );
    /** The cipher of sensitive columns, absent until the database is keyed. */
    cipher: ColumnCipher | undefined = undefined;
    /** Whether the database has a log. */
    isLogged = false;
    /** The submitted statements and transactions. */
    operations = 0;
    /** The run statements and transactions, including nested ones. */
    statements = 0;
    /** The commits that lost a race on their reads and ran again. */
    lostRaces = 0;
    /** The end of the latest serialized work, which the next one waits for. */
    #serialized: Promise<unknown> = Promise.resolve();
    /** The operations to finish before close. */
    readonly #pending = new Set<Promise<void>>();
    /** The shutdown. */
    #closing?: Promise<void>;
    /** The tracker of the database's live instances and owners, opened on first use. */
    #tracker?: Tracker;

    /** The clock the log stamps changes at, the environment's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Create the state of a new connection over declared tables. */
    constructor(options: ConnectionOptions) {
        // store the channels, the namespace and what writes to the declared tables change
        const { openChannel, tables } = options;
        this.locality = options.locality;
        this.environment = options.environment;
        this.durations = new OperationDuration(options.environment.telemetry, [
            ...tables.map((table) => table[TABLE].sqlName),
            ...LOG_TABLES,
        ]);
        this.replicated = options.replicated;
        this.openChannel = openChannel;
        this.namespace = options.namespace;
        this.#dialect = options.dialect;
        this.#isAnnouncing = options.announcer === "database";

        // watch commits on the log channel, then declare the tables
        this.commits = new CommitWatch(
            openChannel === undefined ? undefined : typedChannel(openChannel(LOG_TOPIC), Commit),
            this.unlogged,
        );
        this.declare(tables);
    }

    /** The declared tables, in declaration order. */
    get tables(): readonly Table[] {
        return [...this.#tables.values()];
    }

    /** The tables each operation on each declared table changes, itself first, through cascades. */
    get cascades(): ReadonlyMap<WriteOperation, ReadonlyMap<string, readonly string[]>> {
        return this.#cascades;
    }

    /** The SQL names of the declared tables whose writes fire aggregate triggers. */
    get aggregated(): ReadonlySet<string> {
        return this.#aggregated;
    }

    /** The SQL names of the declared tables whose deletes fire dependent triggers. */
    get dependent(): ReadonlySet<string> {
        return this.#dependent;
    }

    /** The searched tables whose full-text rows SQLite writes update, by SQL name. */
    get searched(): ReadonlyMap<string, Table> {
        return this.#searched;
    }

    /** The declared names of the tables and their columns, by SQL name. */
    get declared(): DeclaredNames {
        return this.#declared;
    }

    /** Declare more tables the database stores, such as those an installation migrated. */
    declare(tables: readonly Table[]): void {
        // add the new tables
        const added = tables.filter((table) => !this.#tables.has(table[TABLE].sqlName));
        if (added.length === 0) {
            return;
        }
        for (const table of added) {
            this.#tables.set(table[TABLE].sqlName, table);
        }

        // list what writes to every declared table change, derive, search and name
        const declared = this.tables;
        this.#cascades = listCascades(declared, this.#dialect);
        this.#aggregated = new Set(
            declared.flatMap((table) =>
                table[TABLE].aggregates.map(
                    (aggregate) => ("from" in aggregate ? aggregate.from() : table)[TABLE].sqlName,
                ),
            ),
        );
        this.#dependent = new Set(
            declared
                .filter((table) => table[TABLE].dependents.length > 0)
                .map((table) => table[TABLE].sqlName),
        );
        this.#searched = new Map(
            declared
                .filter((table) => table[TABLE].fullText !== undefined)
                .map((table) => [table[TABLE].sqlName, table]),
        );
        this.#declared = new Map(
            declared.map((table) => [
                table[TABLE].sqlName,
                {
                    name: table[TABLE].name,
                    columns: new Map(
                        table[TABLE].entries.map(([property, column]) => [
                            column.definition.name,
                            property,
                        ]),
                    ),
                },
            ]),
        );

        // add the new tables without change triggers to the unlogged ones
        for (const table of added.filter((each) => each[TABLE].retention === "none")) {
            this.unlogged.add(table[TABLE].sqlName);
        }

        // watch the declared tables, the database announcing the logged ones but append-only ones
        const announced = added.filter(
            (table) => table[TABLE].retention !== "none" && !table[TABLE].isAppendOnly,
        );
        this.commits.declare(
            [...this.declared.keys()],
            this.#isAnnouncing ? announced.map((table) => table[TABLE].sqlName) : [],
        );
    }

    /** List the tables an operation on some tables changes through triggers and actions. */
    cascade(tables: readonly string[], operation: WriteOperation): Set<string> {
        // add a declared table's cascade, and only an undeclared table
        const cascades = this.cascades.get(operation);
        const written = new Set<string>();
        for (const table of tables) {
            for (const changed of cascades?.get(table) ?? [table]) {
                written.add(changed);
            }
        }

        return written;
    }

    /** List the written tables with change triggers, in written order. */
    logged(written: ReadonlySet<string>): readonly string[] {
        return [...written].filter((table) => !this.unlogged.has(table));
    }

    /** Submit an operation. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // reject work once closing
        if (this.#closing) {
            throw DatabaseError.of("CONNECTION_CLOSED", "the connection is closing or closed");
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

    /** Run work after the connection's earlier serialized work, one at a time in arrival order. */
    serialize<Value>(work: () => Promise<Value>): Promise<Value> {
        const running = this.#serialized.then(work);
        this.#serialized = running.catch(() => undefined);

        return running;
    }

    /** Whether the connection is closing or closed, refusing new work. */
    get isClosing(): boolean {
        return this.#closing !== undefined;
    }

    /** Open the tracker of the database's live instances once. */
    tracker(): Tracker {
        this.#tracker ??= new Tracker(this.environment, this.openChannel?.(TRACKER_CHANNEL));

        return this.#tracker;
    }

    /** Stop submissions, drain work, and close the client once. */
    close(operation: () => Promise<void>): Promise<void> {
        this.#closing ??= this.clock.trackHost(
            Promise.all(this.#pending)
                .then(() => this.#tracker?.close())
                .then(() => this.commits.stop())
                .then(operation),
        );

        return this.#closing;
    }
}

/** Weigh a cached read by its rows and its key's length, at least one row. */
function weigh(key: string, rows: readonly unknown[]): number {
    return Math.max(1, rows.length) + Math.ceil(key.length / KEY_BYTES_PER_ROW);
}

/** List the tables each operation on each table changes, itself first, through cascades. */
function listCascades(
    tables: readonly Table[],
    dialect: Dialect,
): ReadonlyMap<WriteOperation, ReadonlyMap<string, readonly string[]>> {
    // link each operation's write of a table to the writes its triggers and referential actions make
    const links = new Map<string, CascadeLink[]>();
    for (const link of tables.flatMap((table) => cascadeLinks(table, dialect))) {
        for (const operation of link.operations) {
            const from = `${operation} ${link.written}`;
            links.set(from, [...(links.get(from) ?? []), link]);
        }
    }

    // close each table's writes of each operation over the links, keeping each table once
    const cascades = new Map<WriteOperation, ReadonlyMap<string, readonly string[]>>();
    for (const operation of WRITE_OPERATIONS) {
        const closed = new Map<string, readonly string[]>();
        for (const table of tables) {
            const name = table[TABLE].sqlName;
            const reached = new Set([`${operation} ${name}`]);
            for (const next of reached) {
                for (const link of links.get(next) ?? []) {
                    reached.add(`${link.operation} ${link.changed}`);
                }
            }
            const changed = [...reached].map((each) => each.slice(each.indexOf(" ") + 1));
            closed.set(name, [...new Set(changed)]);
        }
        cascades.set(operation, closed);
    }

    return cascades;
}

/** List a table's links from a write of one table to the write it makes in another. */
function cascadeLinks(table: Table, dialect: Dialect): CascadeLink[] {
    // update an aggregate's holder at every write of its source
    const definition = table[TABLE];
    const aggregates = definition.aggregates.map((aggregate): CascadeLink => ({
        written: ("from" in aggregate ? aggregate.from() : table)[TABLE].sqlName,
        operations: WRITE_OPERATIONS,
        changed: ("into" in aggregate ? aggregate.into() : table)[TABLE].sqlName,
        operation: "UPDATE",
    }));

    // delete the dependents a deletion deletes
    const dependents = definition.dependents
        .filter((dependent) => dependent.onDelete === "cascade")
        .map((dependent): CascadeLink => ({
            written: definition.sqlName,
            operations: ["DELETE"],
            changed: dependent.from()[TABLE].sqlName,
            operation: "DELETE",
        }));

    // write the referencing rows a referenced table's deletes and updates write
    const references = definition.constraints(dialect).flatMap((constraint): CascadeLink[] => {
        if (constraint.kind !== "foreignKey") {
            return [];
        }
        const { onDelete, onUpdate } = constraint.actions;
        const deleting: CascadeLink[] =
            onDelete !== undefined && WRITING_ACTIONS.has(onDelete)
                ? [
                      {
                          written: constraint.references,
                          operations: ["DELETE"],
                          changed: definition.sqlName,
                          operation: onDelete === "cascade" ? "DELETE" : "UPDATE",
                      },
                  ]
                : [];
        const updating: CascadeLink[] =
            onUpdate !== undefined && WRITING_ACTIONS.has(onUpdate)
                ? [
                      {
                          written: constraint.references,
                          operations: ["UPDATE"],
                          changed: definition.sqlName,
                          operation: "UPDATE",
                      },
                  ]
                : [];

        return [...deleting, ...updating];
    });

    // keep a tree's ancestors at every write of its nodes
    const tree: CascadeLink[] =
        definition.tree === undefined
            ? []
            : [
                  {
                      written: definition.sqlName,
                      operations: WRITE_OPERATIONS,
                      changed: definition.tree.ancestors[TABLE].sqlName,
                      operation: "DELETE",
                  },
              ];

    return [...aggregates, ...dependents, ...references, ...tree];
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
