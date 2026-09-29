import type { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";
import type { Query, SQL } from "drizzle-orm";
import { assertNever, classifyError } from "../error/error.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";
import { type Span, trace } from "@destack/telemetry";

/** The statements each span ran and their summed duration, in milliseconds. */
const TOTALS = new WeakMap<Span, { statements: number; milliseconds: number }>();

/** A native Drizzle database and its query lifetime. */
export class DatabaseDriver {
    /** The native database connection. */
    readonly native: NativeDatabase;
    /** The shared connection lifecycle. */
    readonly state: ConnectionState;
    /** The active transaction state, when present. */
    readonly transaction?: TransactionState;

    /** Create the driver. */
    constructor(native: NativeDatabase, state: ConnectionState, transaction?: TransactionState) {
        this.native = native;
        this.state = state;
        this.transaction = transaction;
    }

    /** Submit work through the transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // count the statement
        this.state.statements += 1;

        // report concurrent updates first, and add the statement to the active span's totals
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
     * Submit a write and notify readers and writers once it commits outside a transaction.
     *
     * On SQLite, a write outside a transaction runs as one identified transaction.
     */
    async write<Value>(
        operation: (native: NativeDatabase) => PromiseLike<Value>,
        options: { readonly isTransaction?: boolean } = {},
    ): Promise<Value> {
        const result = await this.run(() =>
            options.isTransaction ? operation(this.native) : this.#identified(operation),
        );
        if (!this.transaction) {
            this.state.commits.notify();
        }

        return result;
    }

    /** Run a SQLite write outside a transaction as one identified transaction. */
    async #identified<Value>(
        operation: (native: NativeDatabase) => PromiseLike<Value>,
    ): Promise<Value> {
        // run other writes as they are
        if (this.native.dialect !== "sqlite" || this.transaction !== undefined) {
            return await operation(this.native);
        }

        // run the write in a transaction
        return await this.native.database.transaction(
            async (transaction) => {
                // mark, write and unmark the transaction
                const isMarked = await openTransaction(transaction, this.state);
                const result = await operation({ dialect: "sqlite", database: transaction });
                if (isMarked) {
                    await closeTransaction(transaction);
                }

                return result;
            },
            { behavior: "immediate" },
        );
    }

    /** Render a statement to its text and parameters. */
    render(statement: SQL): Query {
        return (this.native.database as unknown as NativeInternals<never>).dialect.sqlToQuery(
            statement,
        );
    }

    /** Read every row of a rendered query. */
    all<Row>(query: Query): Promise<Row[]> {
        return this.run(async () => {
            // read through the SQLite client's prepared statement
            if (this.native.dialect === "sqlite") {
                const { client } = (
                    this.native.database as unknown as NativeInternals<SqliteClient>
                ).session;

                return (await client.all(query.sql, ...query.params)) as Row[];
            }
            // read through the PostgreSQL client
            else if (this.native.dialect === "postgresql") {
                const { client } = (
                    this.native.database as unknown as NativeInternals<PostgresClient>
                ).session;

                return [
                    ...(await client.unsafe(query.sql, query.params, { prepare: true })),
                ] as Row[];
            }
            // reject other dialects
            else {
                return assertNever(this.native);
            }
        });
    }

    /** Read every row of a rendered query as value arrays. */
    values(query: Query): Promise<unknown[][]> {
        return this.run(async () => {
            // read positional rows with exact integers through SQLite
            if (this.native.dialect === "sqlite") {
                const { client } = (
                    this.native.database as unknown as NativeInternals<SqliteClient>
                ).session;
                const statement = await client.prepare(query.sql);

                return (await statement
                    .safeIntegers(true)
                    .raw(true)
                    .all(...query.params)) as unknown[][];
            }
            // read positional rows through PostgreSQL
            else if (this.native.dialect === "postgresql") {
                const { client } = (
                    this.native.database as unknown as NativeInternals<PostgresClient>
                ).session;

                return [
                    ...(await client.unsafe(query.sql, query.params, { prepare: true }).values()),
                ];
            }
            // reject other dialects
            else {
                return assertNever(this.native);
            }
        });
    }
}

/** A Drizzle database or transaction's internals. */
interface NativeInternals<Client> {
    /** The dialect rendering statements. */
    readonly dialect: { sqlToQuery(statement: SQL): Query };
    /** The session running statements. */
    readonly session: { readonly client: Client };
}

/** A SQLite session client. */
interface SqliteClient {
    /** Read every row of a statement. */
    all(sql: string, ...parameters: unknown[]): Promise<unknown[]>;
    /** Prepare a statement. */
    prepare(sql: string): Promise<{
        safeIntegers(enabled: boolean): {
            raw(enabled: boolean): { all(...parameters: unknown[]): Promise<unknown[]> };
        };
    }>;
}

/** A PostgreSQL session client. */
interface PostgresClient {
    /** Run a statement, prepared once per connection. */
    unsafe(
        sql: string,
        parameters: unknown[],
        options: { readonly prepare: boolean },
    ): Promise<Iterable<unknown>> & {
        /** Read the rows as value arrays. */
        values(): Promise<Iterable<unknown[]>>;
    };
}

/** The dialect and its native Drizzle database. */
export type NativeDatabase =
    | {
          /** SQLite on a file, in memory, in a browser or in a Durable Object. */
          readonly dialect: "sqlite";
          /** The native query connection. */
          readonly database: SQLiteAsyncDatabase<"async", unknown>;
      }
    | {
          /** PostgreSQL through a connection pool. */
          readonly dialect: "postgresql";
          /** The native query connection. */
          readonly database: PostgresJsDatabase;
      };

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
