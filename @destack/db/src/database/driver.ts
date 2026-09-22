import type { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { TransactionState } from "./transaction.ts";
import type { ConnectionState } from "./connection.ts";

/** A native Drizzle database and its query lifetime. */
export class DatabaseDriver {
    /** Native database connection. */
    readonly native: NativeDatabase;
    /** Shared connection lifecycle. */
    readonly state: ConnectionState;
    /** Active transaction state, when present. */
    readonly transaction?: TransactionState;

    /** Retain the native database, connection state, and optional transaction state. */
    constructor(native: NativeDatabase, state: ConnectionState, transaction?: TransactionState) {
        this.native = native;
        this.state = state;
        this.transaction = transaction;
    }

    /** Submit work through the active transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        return this.transaction ? this.transaction.run(operation) : this.state.run(operation);
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
