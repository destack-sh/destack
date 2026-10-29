import type { TransactionState } from "./transaction.ts";
import type { ReadCache } from "../log/cache.ts";
import type { ConnectionState, WriteOperation } from "./connection.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverStatement } from "../sql/index.ts";
import type { Session } from "./session.ts";
import { TelemetryScope } from "@destack/telemetry";
import type { Environment } from "@destack/environment";
import type {} from "@destack/package/import-meta";

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The statement verbs the database runs, as OpenTelemetry's `db.operation.name` reports them. */
const OPERATIONS = ["SELECT", "INSERT", "UPDATE", "DELETE", "PRAGMA", "CREATE", "DROP"];

/** The histogram of each statement's duration in seconds, by OpenTelemetry's conventions. */
export class OperationDuration {
    /** The histogram on the connection's telemetry. */
    readonly #histogram;
    /** The SQL names of the tables a statement may report as its collection. */
    readonly #collections: ReadonlySet<string>;

    /** Declare the histogram on a telemetry, over the database's tables. */
    constructor(telemetry: Environment["telemetry"], collections: readonly string[]) {
        this.#collections = new Set(collections);
        this.#histogram = TelemetryScope.of(
            import.meta.destack.package,
            telemetry,
        ).metric.histogram("db.client.operation.duration", {
            unit: "s",
            attributes: {
                "db.system.name": ["sqlite", "postgresql"],
                "db.operation.name": OPERATIONS,
                "db.collection.name": collections,
            },
        });
    }

    /** Record a statement's duration in milliseconds on a dialect. */
    record(milliseconds: number, dialect: Dialect, statement: DriverStatement | undefined): void {
        // report the operation and the first table, when the database declares them
        const verb = statement?.operation;
        const operation = verb !== undefined && OPERATIONS.includes(verb) ? verb : undefined;
        const table = statement?.tables[0];
        const collection = table !== undefined && this.#collections.has(table) ? table : undefined;
        this.#histogram.record(milliseconds / MILLISECONDS_PER_SECOND, {
            "db.system.name": dialect,
            ...(operation === undefined ? {} : { "db.operation.name": operation }),
            ...(collection === undefined ? {} : { "db.collection.name": collection }),
        });
    }
}

/** The tables no script wrote, which a plain write passes. */
const UNSCRIPTED: ReadonlySet<string> = new Set();

/** Whether a write logs: through tables with change triggers, through none, or unknown. */
type Logging = "logged" | "unlogged" | "unknown";

/** A session and its statements' lifetime, counting and failures. */
export class DatabaseDriver {
    /** The session running statements. */
    readonly session: Session;
    /** The shared connection lifecycle. */
    readonly state: ConnectionState;
    /** The active transaction state, when present. */
    readonly transaction: TransactionState | undefined;
    /** The operations submitted through this driver outside a transaction. */
    operations = 0;

    /** Create the driver. */
    constructor(session: Session, state: ConnectionState, transaction?: TransactionState) {
        // store the session, the connection's state and the transaction
        this.session = session;
        this.state = state;
        this.transaction = transaction;
    }

    /** The SQL dialect. */
    get dialect(): Dialect {
        return this.session.dialect;
    }

    /** Submit work through the transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>, statement?: DriverStatement): Promise<Value> {
        // count the statement, and the operation the connection submits outside a transaction
        this.state.statements += 1;
        if (this.transaction === undefined) {
            this.operations += 1;
        }

        // count the statement in flight on the clock, classify failures, and record its duration
        const reported = async () => {
            const started = this.state.clock.now();
            try {
                return await this.state.clock.trackHost(Promise.resolve(operation()));
            } catch (error) {
                throw classifyError(error, this.state.declared);
            } finally {
                this.state.durations.record(
                    this.state.clock.now() - started,
                    this.dialect,
                    statement,
                );
            }
        };

        return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
    }

    /** Submit a statement writing its tables. */
    write<Value>(
        operation: (session: Session) => Promise<Value>,
        statement: DriverStatement,
    ): Promise<Value> {
        // cascade the written tables, every table for unknown ones
        const { tables } = statement;
        const written =
            tables.length === 0
                ? new Set(this.state.declared.keys())
                : this.state.cascade(tables, writeOperation(statement));

        // stamp a write that may log, of tables with triggers or of unknown tables
        const logged = this.state.logged(written);
        let logging: Logging = logged.length > 0 ? "logged" : "unlogged";

        // read a write of unknown tables as unknown
        if (tables.length === 0) {
            logging = "unknown";
        }

        return this.commit(
            () => this.#identified(operation, logging, logged),
            written,
            UNSCRIPTED,
            statement,
        );
    }

    /** Submit work that commits by itself and writes some tables, some of them by scripts. */
    async commit<Value>(
        operation: () => Promise<Value>,
        tables: ReadonlySet<string>,
        scripted: ReadonlySet<string>,
        statement?: DriverStatement,
    ): Promise<Value> {
        // record the tables in the enclosing transaction, marking a replica's write that derives rows first
        const transaction = this.transaction;
        if (transaction !== undefined) {
            const mark = transaction.replicaMark;
            if (mark !== undefined && this.#isDerived([...tables, ...scripted], statement)) {
                transaction.replicaMark = undefined;
                await mark();
            }
            const result = await this.run(operation, statement);
            for (const table of tables) {
                transaction.written.add(table);
            }
            for (const table of scripted) {
                transaction.scripted.add(table);
            }

            return result;
        }

        // commit
        const result = await this.run(operation, statement);

        // announce a commit that wrote tables
        if (tables.size > 0) {
            this.state.commits.notify(tables, scripted);
        }

        return result;
    }

    /** Read every row of a bound statement as an array of values. */
    values(statement: DriverStatement): Promise<unknown[][]> {
        const read = () =>
            this.#query(statement, (session) =>
                session.values(statement.text, statement.parameters),
            );

        return this.#isCacheable(statement)
            ? this.#cached(statement, this.state.valueCache, read)
            : read();
    }

    /** Read every row of a bound statement by column name. */
    all(statement: DriverStatement): Promise<Record<string, unknown>[]> {
        const read = () =>
            this.#query(statement, (session) => session.all(statement.text, statement.parameters));

        return this.#isCacheable(statement)
            ? this.#cached(statement, this.state.recordCache, read)
            : read();
    }

    /** Run a bound statement for its effect: a write of its tables, or a read such as a lock. */
    async execute(statement: DriverStatement): Promise<void> {
        // run a read alone, writing no table
        this.transaction?.assertActive();
        const run = (session: Session) =>
            failing(statement, session.run(statement.text, statement.parameters), this.state);
        if (statement.isRead) {
            await this.run(() => run(this.session), statement);

            return;
        }

        // run a write of its tables
        await this.write(run, statement);
    }

    /** Run a statement returning rows, as a read or as a write of the tables it references. */
    #query<Value>(
        statement: DriverStatement,
        query: (session: Session) => Promise<Value>,
    ): Promise<Value> {
        this.transaction?.assertActive();

        return statement.isRead
            ? this.run(() => failing(statement, query(this.session), this.state), statement)
            : this.write((session) => failing(statement, query(session), this.state), statement);
    }

    /** Decide whether a read may be cached: a live read of versioned tables only. */
    #isCacheable(statement: DriverStatement): boolean {
        const { commits } = this.state;

        return (
            statement.isRead &&
            this.transaction === undefined &&
            statement.tables.length > 0 &&
            statement.tables.every((table) => commits.isVersioned(table))
        );
    }

    /** Read a statement once per version of its tables, every caller sharing its frozen rows. */
    async #cached<Row>(
        statement: DriverStatement,
        cache: ReadCache<Row[]>,
        read: () => Promise<Row[]>,
    ): Promise<Row[]> {
        // read from the database until the watch receives every commit, listening from the first read
        const { commits } = this.state;
        if (!commits.isComplete) {
            void commits.listen();

            return await read();
        }

        // key the read by its text and parameters, at its tables' versions before it runs
        const versions = commits.versions(statement.tables);
        const key = `${statement.text} ${JSON.stringify(statement.parameters, encodeKey)}`;

        return await cache.get(key, versions, async () => freeze(await read()));
    }

    /** Decide whether a write fires derivations, dependents only on deletes. */
    #isDerived(tables: readonly string[], statement: DriverStatement | undefined): boolean {
        const { aggregated, dependent, declared, unlogged } = this.state;
        const isDeleting = writeOperation(statement) === "DELETE";

        return tables.some(
            (table) =>
                aggregated.has(table) ||
                (isDeleting && dependent.has(table)) ||
                (!declared.has(table) && !unlogged.has(table)),
        );
    }

    /** Run a write stamped in its transaction, refusing a logged write outside one. */
    async #identified<Value>(
        operation: (session: Session) => Promise<Value>,
        logging: Logging,
        logged: readonly string[],
    ): Promise<Value> {
        // stamp a write that may log inside a transaction, in statement order
        if (this.transaction !== undefined && logging !== "unlogged") {
            await this.transaction.identity.stamp();

            return await operation(this.session);
        }
        // refuse a known logged write outside a transaction
        else if (this.transaction === undefined && logging === "logged") {
            throw DatabaseError.of(
                "TRANSACTION_REQUIRED",
                `a logged write of ${logged.join(", ")} runs outside a transaction: run it in a transaction`,
            );
        }
        // run any other write as it is, the triggers refusing an unknown table's logged change
        else {
            return await operation(this.session);
        }
    }
}

/** Report a driver failure with its statement, leaving classified failures as they are. */
async function failing<Value>(
    statement: DriverStatement,
    pending: Promise<Value>,
    state: ConnectionState,
): Promise<Value> {
    try {
        return await pending;
    } catch (error) {
        // rethrow a classified failure, and report another with its statement
        const classified = classifyError(error, state.declared);
        if (DatabaseError.is(classified)) {
            throw classified;
        }

        throw DatabaseError.of("QUERY_FAILED", `query failed: ${statement.text}`, {
            cause: error,
        });
    }
}

/** Read a statement's write operation, a delete for an unknown one, as it cascades furthest. */
function writeOperation(statement: DriverStatement | undefined): WriteOperation {
    const operation = statement?.operation;

    return operation === "INSERT" || operation === "UPDATE" ? operation : "DELETE";
}

/** Encode a cached read's parameters as a key, bigints apart from strings. */
function encodeKey(_key: string, value: unknown): unknown {
    return typeof value === "bigint" ? { bigint: String(value) } : value;
}

/** Freeze rows and each row, so callers share them and a write to one fails loudly. */
function freeze<Row>(rows: Row[]): Row[] {
    for (const row of rows) {
        Object.freeze(row);
    }

    Object.freeze(rows);

    return rows;
}
