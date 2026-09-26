import type { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";
import type { Query, SQL } from "drizzle-orm";
import { assertNever, classifyError } from "../error/error.ts";

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

    /** Submit a write, notifying readers and other writers once it commits outside a transaction. */
    async write<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        const result = await this.run(operation);
        if (!this.transaction) {
            this.state.commits.notify();
        }

        return result;
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
}

/** A PostgreSQL session client running a statement text. */
interface PostgresClient {
    /** Run a statement text with its parameters, prepared once per connection. */
    unsafe(
        sql: string,
        parameters: unknown[],
        options: { readonly prepare: boolean },
    ): Promise<Iterable<unknown>>;
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
