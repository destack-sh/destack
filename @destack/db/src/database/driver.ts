import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverStatement } from "../sql/index.ts";
import type { Session } from "./session.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";
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

    /** Create the driver. */
    constructor(session: Session, state: ConnectionState, transaction?: TransactionState) {
        this.session = session;
        this.state = state;
        this.transaction = transaction;
    }

    /** The SQL dialect. */
    get dialect(): Dialect {
        return this.session.dialect;
    }

    /** Submit work through the transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // count the statement
        this.state.statements += 1;

        // classify failures, and record the statement's duration
        const reported = async () => {
            const started = performance.now();
            try {
                return await operation();
            } catch (error) {
                throw classifyError(error);
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

        return this.commit(() => this.#identified(operation), written);
    }

    /** Submit work that commits on its own and writes some tables, such as a transaction. */
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
        return this.#query(statement, (session) =>
            session.values(statement.text, statement.parameters),
        );
    }

    /** Read every row of a bound statement by column name. */
    all(statement: DriverStatement): Promise<Record<string, unknown>[]> {
        return this.#query(statement, (session) =>
            session.all(statement.text, statement.parameters),
        );
    }

    /** Run a bound write of the listed tables, identified on SQLite outside a transaction. */
    async execute(statement: DriverStatement): Promise<void> {
        this.transaction?.assertActive();
        await this.write(
            (session) => failing(statement, session.run(statement.text, statement.parameters)),
            statement.tables,
        );
    }

    /** Run a statement returning rows, as a read or as a write of the tables it names. */
    #query<Value>(
        statement: DriverStatement,
        query: (session: Session) => Promise<Value>,
    ): Promise<Value> {
        this.transaction?.assertActive();

        return statement.isRead
            ? this.run(() => failing(statement, query(this.session)))
            : this.write((session) => failing(statement, query(session)), statement.tables);
    }

    /** List the tables writes to some tables change, through triggers and referential actions. */
    #cascade(tables: readonly string[]): Set<string> {
        const written = new Set<string>();
        for (const table of tables) {
            // add a declared table's cascade, and an undeclared table alone
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

    /** Run a write outside a transaction as one identified SQLite transaction. */
    async #identified<Value>(operation: (session: Session) => Promise<Value>): Promise<Value> {
        // run other writes as they are
        if (this.session.dialect !== "sqlite" || this.transaction !== undefined) {
            return await operation(this.session);
        }

        // run the write in a transaction
        return await this.session.transaction(
            async (transaction) => {
                // mark, write and unmark the transaction
                const isMarked = await openTransaction(transaction, this.state);
                const result = await operation(transaction);
                if (isMarked) {
                    await closeTransaction(transaction, this.state);
                }

                return result;
            },
            { isReadOnly: false, isolationLevel: "serializable" },
        );
    }
}

/** Report a driver failure with its statement, leaving classified failures as they are. */
async function failing<Value>(statement: DriverStatement, pending: Promise<Value>): Promise<Value> {
    try {
        return await pending;
    } catch (error) {
        // keep a classified failure, and report another with its statement
        const classified = classifyError(error);
        if (classified instanceof DatabaseError) {
            throw classified;
        }

        throw new DatabaseError("QUERY_FAILED", `query failed: ${statement.text}`, {
            cause: error,
        });
    }
}
