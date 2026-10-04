import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverStatement } from "../sql/index.ts";
import type { Session } from "./session.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";
import { type Span, trace } from "@destack/telemetry";

/** The statements each span ran and their summed duration, in milliseconds. */
const TOTALS = new WeakMap<Span, { statements: number; milliseconds: number }>();

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

        // classify failures, and add the statement to the active span's totals
        const reported = async () => {
            const span = trace.getActiveSpan();
            const started = performance.now();
            try {
                return await operation();
            } catch (error) {
                throw classifyError(error);
            } finally {
                if (span !== undefined) {
                    total(span, performance.now() - started);
                }
            }
        };

        return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
    }

    /**
     * Submit a write and notify readers and writers after it commits outside a transaction.
     *
     * On SQLite, a write outside a transaction runs as one identified transaction.
     */
    write<Value>(operation: (session: Session) => Promise<Value>): Promise<Value> {
        return this.commit(() => this.#identified(operation));
    }

    /** Submit work that commits on its own, such as a transaction, and notify readers and writers after it commits. */
    async commit<Value>(operation: () => Promise<Value>): Promise<Value> {
        const result = await this.run(operation);
        if (this.transaction === undefined) {
            this.state.commits.notify();
        }

        return result;
    }

    /** Read every row of a bound statement as an array of values. */
    values(statement: DriverStatement): Promise<unknown[][]> {
        this.transaction?.assertActive();

        return this.run(() =>
            failing(statement, this.session.values(statement.text, statement.parameters)),
        );
    }

    /** Read every row of a bound statement by column name. */
    all(statement: DriverStatement): Promise<Record<string, unknown>[]> {
        this.transaction?.assertActive();

        return this.run(() =>
            failing(statement, this.session.all(statement.text, statement.parameters)),
        );
    }

    /** Run a bound write statement, identified on SQLite outside a transaction. */
    async execute(statement: DriverStatement): Promise<void> {
        this.transaction?.assertActive();
        await this.write((session) =>
            failing(statement, session.run(statement.text, statement.parameters)),
        );
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
                    await closeTransaction(transaction);
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
        const classified = classifyError(error);
        if (classified !== error) {
            throw classified;
        }

        throw new DatabaseError("QUERY_FAILED", `query failed: ${statement.text}`, {
            cause: error,
        });
    }
}

/** Add a statement to a span's totals and stamp them on it. */
function total(span: Span, milliseconds: number): void {
    // add to the running totals
    const totals = TOTALS.get(span) ?? { statements: 0, milliseconds: 0 };
    totals.statements += 1;
    totals.milliseconds += milliseconds;
    TOTALS.set(span, totals);

    // stamp them on the span, replacing the previous totals
    span.setAttributes({
        "destack.db.statements": totals.statements,
        "destack.db.duration": totals.milliseconds,
    });
}
