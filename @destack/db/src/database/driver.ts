import type { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";
import type { Query, SQL } from "drizzle-orm";
import { assertNever, classifyError } from "../error/error.ts";
import { closeTransaction, openTransaction } from "../log/transaction.ts";

/** A native Drizzle database and its query lifetime. */
export class DatabaseDriver {
    /** The native database connection. */
    readonly native: NativeDatabase;
    /** The shared connection lifecycle. */
    readonly state: ConnectionState;
    /** The active transaction state, when present. */
    readonly transaction?: TransactionState;

    /** Retain the native database, connection state, and optional transaction state. */
    constructor(native: NativeDatabase, state: ConnectionState, transaction?: TransactionState) {
        this.native = native;
        this.state = state;
        this.transaction = transaction;
    }

    /** Submit work through the active transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        // report concurrent updates before the transaction records the failure
        const reported = async () => {
            try {
                return await operation();
            } catch (error) {
                throw classifyError(error);
            }
        };

        return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
    }

    /**
     * Submit a write on the native database it must run on, notifying readers and other writers once it commits outside a transaction.
     *
     * Outside a transaction on SQLite, the write runs as one transaction the log identifies, so that its changes commit as one.
     * A write that manages its own transaction, such as a transaction or a script, runs as it is.
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

    /** Run a SQLite write outside a transaction as one transaction the log's triggers identify, and others as they are. */
    async #identified<Value>(
        operation: (native: NativeDatabase) => PromiseLike<Value>,
    ): Promise<Value> {
        // run writes inside transactions, and PostgreSQL writes, as they are
        if (this.native.dialect !== "sqlite" || this.transaction !== undefined) {
            return await operation(this.native);
        }

        // run the write on the transaction holding the connection
        return await this.native.database.transaction(
            async (transaction) => {
                // mark the transaction for the log, write, then clear the mark before it commits
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

    /** Render a native statement to its text and parameters in the connection's dialect. */
    render(statement: SQL): Query {
        return (this.native.database as unknown as NativeInternals<never>).dialect.sqlToQuery(
            statement,
        );
    }

    /** Read every row of rendered text with its parameters on the native connection or transaction, as driver rows. */
    all<Row>(query: Query): Promise<Row[]> {
        return this.run(async () => {
            // read through the SQLite session's client, which reuses its prepared statement for the text
            if (this.native.dialect === "sqlite") {
                const { client } = (
                    this.native.database as unknown as NativeInternals<SqliteClient>
                ).session;

                return (await client.all(query.sql, ...query.params)) as Row[];
            }
            // read through the PostgreSQL session's client, prepared once per connection
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

    /** Read every row of rendered text with its parameters on the native connection or transaction, as arrays of values in selected order. */
    values(query: Query): Promise<unknown[][]> {
        return this.run(async () => {
            // read positional rows through the SQLite session's client, which reuses its prepared statement for the text and mode
            if (this.native.dialect === "sqlite") {
                const { client } = (
                    this.native.database as unknown as NativeInternals<SqliteClient>
                ).session;
                const statement = await client.prepare(query.sql);

                return (await statement.raw(true).all(...query.params)) as unknown[][];
            }
            // read positional rows through the PostgreSQL session's client, prepared once per connection
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

/** A Drizzle database or transaction: its dialect renders statements and its session's client runs them. */
interface NativeInternals<Client> {
    /** The dialect rendering statements to text and parameters. */
    readonly dialect: { sqlToQuery(statement: SQL): Query };
    /** The session running the database's statements. */
    readonly session: { readonly client: Client };
}

/** A SQLite session client reading the rows of a statement text. */
interface SqliteClient {
    /** Read every row of a statement text with its parameters. */
    all(sql: string, ...parameters: unknown[]): Promise<unknown[]>;
    /** Prepare a statement text, whose row mode selects named or positional rows. */
    prepare(sql: string): Promise<{
        raw(enabled: boolean): { all(...parameters: unknown[]): Promise<unknown[]> };
    }>;
}

/** A PostgreSQL session client running a statement text. */
interface PostgresClient {
    /** Run a statement text with its parameters, prepared once per connection. */
    unsafe(
        sql: string,
        parameters: unknown[],
        options: { readonly prepare: boolean },
    ): Promise<Iterable<unknown>> & {
        /** Read the rows as arrays of values in selected order. */
        values(): Promise<Iterable<unknown[]>>;
    };
}

/** The selected dialect and its native Drizzle database. */
export type NativeDatabase =
    | {
          /** SQLite queries through embedded or hosted Turso. */
          readonly dialect: "sqlite";
          /** The native query connection. */
          readonly database: SQLiteAsyncDatabase<"async", unknown>;
      }
    | {
          /** PostgreSQL queries through a connection pool. */
          readonly dialect: "postgresql";
          /** The native query connection. */
          readonly database: PostgresJsDatabase;
      };
