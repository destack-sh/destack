import type postgres from "postgres";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverValue } from "../table/column.ts";
import type { ConnectionClient, QueryClient } from "../sqlite/client.ts";

/** How a transaction isolates its reads and whether it writes. */
export interface SessionTransaction {
    /** Whether the transaction only reads. */
    readonly isReadOnly: boolean;
    /** The PostgreSQL isolation level. */
    readonly isolationLevel: "read committed" | "repeatable read" | "serializable";
}

/** Statements over one SQLite or PostgreSQL connection, or one of its transactions. */
export interface Session {
    /** The SQL dialect. */
    readonly dialect: Dialect;
    /** Read every row as an array of values, integers exact. */
    values(text: string, parameters: readonly DriverValue[]): Promise<unknown[][]>;
    /** Read every row by column name. */
    all(text: string, parameters: readonly DriverValue[]): Promise<Record<string, unknown>[]>;
    /** Run a statement for its effect. */
    run(text: string, parameters: readonly DriverValue[]): Promise<void>;
    /** Run a script of statements in one round trip. */
    exec(script: string): Promise<void>;
    /** Commit work in a transaction, or in a savepoint within one, rolling back when it fails. */
    transaction<Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ): Promise<Value>;
}

/** Statements over a SQLite connection client, or one of its transactions at a savepoint depth. */
export class SqliteSession implements Session {
    /** The SQL dialect. */
    readonly dialect = "sqlite";
    /** The client running statements. */
    readonly client: QueryClient;
    /** The connection that starts transactions, absent inside one. */
    readonly #connection: ConnectionClient | undefined;
    /** The savepoint depth inside a transaction. */
    readonly #depth: number;

    /** Run statements on a connection, or in a transaction at a depth. */
    constructor(client: ConnectionClient | QueryClient, depth?: number) {
        this.client = client;
        this.#connection = depth === undefined && "transactionAsync" in client ? client : undefined;
        this.#depth = depth ?? 0;
    }

    /** Read every row as an array of values, integers exact. */
    values(text: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.client.values(text, parameters);
    }

    /** Read every row by column name. */
    async all(
        text: string,
        parameters: readonly DriverValue[],
    ): Promise<Record<string, unknown>[]> {
        const rows = await this.client.all(text, parameters);

        return rows.map(requireRecord);
    }

    /** Run a statement for its effect. */
    async run(text: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.client.run(text, parameters);
    }

    /** Run a script of statements in one round trip. */
    async exec(script: string): Promise<void> {
        await this.client.exec(script);
    }

    /** Commit work in a transaction, written immediately unless it only reads, or nest it in a savepoint. */
    transaction<Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ): Promise<Value> {
        // nest inside an open transaction
        if (this.#connection === undefined) {
            return this.client.nest(this.#depth, (client) =>
                operation(new SqliteSession(client, this.#depth + 1)),
            );
        }

        // start a transaction on the connection
        const transaction = this.#connection.transactionAsync((client) =>
            operation(new SqliteSession(client, 0)),
        );

        return options.isReadOnly ? transaction.deferred() : transaction.immediate();
    }
}

/** Statements over a PostgreSQL pool, or one of its transactions. */
export class PostgresSession implements Session {
    /** The SQL dialect. */
    readonly dialect = "postgresql";
    /** The pool or transaction running statements. */
    readonly client: postgres.Sql | postgres.TransactionSql;

    /** Run statements on a pool or in a transaction. */
    constructor(client: postgres.Sql | postgres.TransactionSql) {
        this.client = client;
    }

    /** Read every row as an array of values, prepared once per connection. */
    async values(text: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        const rows = await this.client
            .unsafe(text, parameters.map(postgresParameter), { prepare: true })
            .values();

        return [...rows].map(requireArray);
    }

    /** Read every row by column name. */
    async all(
        text: string,
        parameters: readonly DriverValue[],
    ): Promise<Record<string, unknown>[]> {
        const rows = await this.client.unsafe(text, parameters.map(postgresParameter), {
            prepare: true,
        });

        return [...rows].map(requireRecord);
    }

    /** Run a statement for its effect. */
    async run(text: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.client.unsafe(text, parameters.map(postgresParameter), { prepare: true });
    }

    /** Run a script of statements as one simple query. */
    async exec(script: string): Promise<void> {
        await this.client.unsafe(script).simple();
    }

    /** Commit work in a transaction, or nest it in a savepoint. */
    async transaction<Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ): Promise<Value> {
        // nest inside an open transaction
        const client = this.client;
        if ("savepoint" in client) {
            const nested = await client.savepoint(async (transaction) => ({
                value: await operation(new PostgresSession(transaction)),
            }));

            return nested.value;
        }

        // begin a transaction with the isolation level and access mode
        const mode = `isolation level ${options.isolationLevel} ${options.isReadOnly ? "read only" : "read write"}`;
        const begun = await client.begin(mode, async (transaction) => ({
            value: await operation(new PostgresSession(transaction)),
        }));

        return begun.value;
    }
}

/** Require a driver row to be an array of values. */
function requireArray(row: unknown): unknown[] {
    if (!Array.isArray(row)) {
        throw new TypeError(
            "the database driver returned a record where an array of values was asked",
        );
    }

    return row;
}

/** Require a driver row to be a record of values by column name. */
function requireRecord(row: unknown): Record<string, unknown> {
    if (!isRecord(row)) {
        throw new TypeError(
            "the database driver returned an array of values where a record was asked",
        );
    }

    return row;
}

/** Report whether a driver row is a record. */
function isRecord(row: unknown): row is Record<string, unknown> {
    return typeof row === "object" && row !== null && !Array.isArray(row);
}

/** Pass a driver value to PostgreSQL, a 64-bit integer as its decimal text, which int8 columns read exactly. */
function postgresParameter(value: DriverValue): postgres.ParameterOrJSON<never> {
    return typeof value === "bigint" ? value.toString() : value;
}
