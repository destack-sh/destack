import type { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";
import { classifyError } from "../error/error.ts";

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
