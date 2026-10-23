import type { TransactionState } from "./transaction.ts";
import { present } from "@destack/schema";
import { LogColumn } from "../log/position.ts";
import type { ReadCache } from "../log/cache.ts";
import type { ConnectionState } from "./connection.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverStatement } from "../sql/index.ts";
import type { Session } from "./session.ts";
import { TransactionIdentity } from "../log/transaction.ts";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The duration of each statement, in seconds, as OpenTelemetry's database conventions name it. */
const operationDuration = telemetry
    .scope(import.meta.destack.package)
    .metric.histogram("db.client.operation.duration", {
        unit: "s",
        attributes: { "db.system.name": ["sqlite", "postgresql"] },
    });

/** A session and its statements' lifetime, counting and failures. */
export class DatabaseDriver {
    /** The session running statements. */
    readonly session: Session;
    /** The shared connection lifecycle. */
    readonly state: ConnectionState;
    /** The active transaction state, when present. */
    readonly transaction: TransactionState | undefined;
    /** The operations submitted outside a transaction, shared by a connection's sharing driver. */
    readonly meter: Meter;
    /** Whether reads of shared tables outside a transaction go through the shared caches. */
    readonly isSharing: boolean;
    /** The shared tables' version at the first shared read, below which no shared read goes. */
    #version: Promise<Version> | undefined;

    /** Create the driver. */
    constructor(
        session: Session,
        state: ConnectionState,
        transaction?: TransactionState,
        options: DriverOptions = {},
    ) {
        // store the session, the connection's state, the transaction, the meter and read sharing
        this.session = session;
        this.state = state;
        this.transaction = transaction;
        this.meter = options.meter ?? { operations: 0 };
        this.isSharing = options.isSharing ?? false;
    }

    /** The operations submitted through this driver outside a transaction. */
    get operations(): number {
        return this.meter.operations;
    }

    /** The SQL dialect. */
    get dialect(): Dialect {
        return this.session.dialect;
    }

    /** Submit work through the transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // count the statement, and the operation the connection submits outside a transaction
        this.state.statements += 1;
        if (this.transaction === undefined) {
            this.meter.operations += 1;
        }

        // classify failures, and record the statement's duration
        const reported = async () => {
            const started = performance.now();
            try {
                return await operation();
            } catch (error) {
                throw classifyError(error, this.state.declared);
            } finally {
                const seconds = (performance.now() - started) / MILLISECONDS_PER_SECOND;
                operationDuration.record(seconds, { "db.system.name": this.dialect });
            }
        };

        return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
    }

    /** Submit a write of the listed tables. */
    write<Value>(
        operation: (session: Session) => Promise<Value>,
        tables: readonly string[],
    ): Promise<Value> {
        const written =
            tables.length === 0 ? new Set(this.state.cascades.keys()) : this.#cascade(tables);

        // stamp a write that may log, of a table with triggers or of unknown tables
        const isLogging = tables.length === 0 || this.state.isLogging(written);

        return this.commit(() => this.#identified(operation, isLogging), written);
    }

    /** Submit work that commits by itself and writes some tables, such as a transaction. */
    async commit<Value>(
        operation: () => Promise<Value>,
        tables: ReadonlySet<string>,
    ): Promise<Value> {
        // record the tables in the enclosing transaction
        const transaction = this.transaction;
        if (transaction !== undefined) {
            const result = await this.run(operation);
            for (const table of tables) {
                transaction.written.add(table);
            }

            return result;
        }

        // commit
        const result = await this.run(operation);

        // announce a commit that wrote tables
        if (tables.size > 0) {
            this.state.commits.notify([...tables]);
        }

        return result;
    }

    /** Read every row of a bound statement as an array of values. */
    values(statement: DriverStatement): Promise<unknown[][]> {
        const read = () =>
            this.#query(statement, (session) =>
                session.values(statement.text, statement.parameters),
            );

        return this.#isShared(statement)
            ? this.#share(statement, this.state.valueCache, read)
            : read();
    }

    /** Read every row of a bound statement by column name. */
    all(statement: DriverStatement): Promise<Record<string, unknown>[]> {
        const read = () =>
            this.#query(statement, (session) => session.all(statement.text, statement.parameters));

        return this.#isShared(statement)
            ? this.#share(statement, this.state.recordCache, read)
            : read();
    }

    /** Run a bound write of the listed tables. */
    async execute(statement: DriverStatement): Promise<void> {
        this.transaction?.assertActive();
        await this.write(
            (session) =>
                failing(statement, session.run(statement.text, statement.parameters), this.state),
            statement.tables,
        );
    }

    /** Run a statement returning rows, as a read or as a write of the tables it references. */
    #query<Value>(
        statement: DriverStatement,
        query: (session: Session) => Promise<Value>,
    ): Promise<Value> {
        this.transaction?.assertActive();

        return statement.isRead
            ? this.run(() => failing(statement, query(this.session), this.state))
            : this.write(
                  (session) => failing(statement, query(session), this.state),
                  statement.tables,
              );
    }

    /** Decide whether a sharing driver's statement reads only shared tables. */
    #isShared(statement: DriverStatement): boolean {
        const { shared } = this.state;

        return (
            this.isSharing &&
            statement.isRead &&
            this.transaction === undefined &&
            statement.tables.length > 0 &&
            statement.tables.every((table) => shared.has(table))
        );
    }

    /** Read a statement once per log position of its tables, answering each caller a copy. */
    async #share<Row>(
        statement: DriverStatement,
        cache: ReadCache<Row[]>,
        read: () => Promise<Row[]>,
    ): Promise<Row[]> {
        // read the shared tables' latest sequences once, with the log, the epoch and the horizon
        const { log, epoch, horizon, latest, applied } = await (this.#version ??
            this.#readVersion());

        // share the rows at the tables' latest change, or past compaction at the horizon
        const sequence = Math.max(
            horizon,
            ...statement.tables.map((table) =>
                present(latest.get(table), `the version of ${table}`),
            ),
        );
        // key the read by its tables' migrations, which change predefined rows without the log
        const migrations = statement.tables.map((table) =>
            present(applied.get(table), `the migration of ${table}`),
        );
        const key = `${migrations.join(",")} ${statement.text} ${JSON.stringify(statement.parameters, encodeKey)}`;
        const rows = await cache.get(key, read, { log, epoch, sequence });

        return structuredClone(rows);
    }

    /** Read the shared tables' version for this driver's shared reads, again after a failure. */
    #readVersion(): Promise<Version> {
        // read the latest sequence of each shared table, with the log, the epoch and the horizon
        const text = present(this.state.versionStatement, "the shared tables' version statement");
        const reading = this.run(() => this.session.values(text, [])).then(([row = []]) => ({
            log: LogColumn.text(row[0]),
            epoch: LogColumn.text(row[1]),
            horizon: LogColumn.integer(row[2]),
            latest: new Map(
                [...this.state.shared].map((table, index) => [
                    table,
                    LogColumn.nullableInteger(row[index + 3]) ?? 0,
                ]),
            ),
            applied: new Map(
                [...this.state.shared].map((table, index) => [
                    table,
                    present(
                        LogColumn.nullableInteger(row[index + 3 + this.state.shared.size]),
                        `the migration state of ${table}`,
                    ),
                ]),
            ),
        }));
        this.#version = reading;

        // forget a failed read so the next shared read tries again
        reading.catch(() => {
            if (this.#version === reading) {
                this.#version = undefined;
            }
        });

        return reading;
    }

    /** List the tables writes to some tables change, through triggers and referential actions. */
    #cascade(tables: readonly string[]): Set<string> {
        const written = new Set<string>();
        for (const table of tables) {
            // add a declared table's cascade, and only an undeclared table
            const cascade = this.state.cascades.get(table);
            if (cascade === undefined) {
                written.add(table);
            } else {
                for (const changed of cascade) {
                    written.add(changed);
                }
            }
        }

        return written;
    }

    /** Run a write after stamping its transaction's log identity. */
    async #identified<Value>(
        operation: (session: Session) => Promise<Value>,
        isLogging: boolean,
    ): Promise<Value> {
        // run an unlogged write, or a PostgreSQL write outside a transaction, as it is
        if (!isLogging || (this.transaction === undefined && this.session.dialect !== "sqlite")) {
            return await operation(this.session);
        }
        // stamp the enclosing transaction ahead of its first logged write, in statement order
        else if (this.transaction !== undefined) {
            await this.transaction.identity.stamp();

            return await operation(this.session);
        }
        // run a SQLite write in a separate stamped transaction
        else {
            return await this.session.transaction(
                async (transaction) => {
                    // stamp, write and clear the transaction
                    const identity = new TransactionIdentity(transaction, this.state);
                    await identity.stamp();
                    const result = await operation(transaction);
                    await identity.close();

                    return result;
                },
                { isReadOnly: false, isolationLevel: "serializable" },
            );
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

/** How a driver counts and shares its reads. */
export interface DriverOptions {
    /** Whether reads of shared tables outside a transaction go through the shared caches. */
    readonly isSharing?: boolean;
    /** The count of operations to add to, a new one when absent. */
    readonly meter?: Meter;
}

/** A count of the operations drivers submit outside a transaction. */
interface Meter {
    /** The operations submitted. */
    operations: number;
}

/** The shared tables' log version: the log, epoch, horizon and each table's latest sequence. */
interface Version {
    /** The log's identifier. */
    readonly log: string;
    /** The log's epoch. */
    readonly epoch: string;
    /** The compaction horizon, past every compacted change. */
    readonly horizon: number;
    /** The latest sequence each shared table was written at, by SQL name. */
    readonly latest: ReadonlyMap<string, number>;
    /** Each shared table's migration version, its last apply's time, by SQL name. */
    readonly applied: ReadonlyMap<string, number>;
}

/** Encode a shared read's parameters as a key, bigints apart from strings. */
function encodeKey(_key: string, value: unknown): unknown {
    return typeof value === "bigint" ? { bigint: String(value) } : value;
}
