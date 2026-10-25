import type { Clock } from "@destack/environment";
import type postgres from "postgres";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverValue } from "../table/column.ts";
import type { SqliteConnectionClient, SqliteQueryClient } from "../sqlite/client.ts";

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
    readonly values: (text: string, parameters: readonly DriverValue[]) => Promise<unknown[][]>;
    /** Read every row by column name. */
    readonly all: (
        text: string,
        parameters: readonly DriverValue[],
    ) => Promise<Record<string, unknown>[]>;
    /** Run a statement for its effect. */
    readonly run: (text: string, parameters: readonly DriverValue[]) => Promise<void>;
    /** Run a script of statements in one round trip. */
    readonly exec: (script: string) => Promise<void>;
    /** Commit work in a transaction, or in a savepoint within one, rolling back when it fails. */
    readonly transaction: <Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ) => Promise<Value>;
}

/** Statements over a SQLite connection client, or one of its transactions at a savepoint depth. */
export class SqliteSession implements Session {
    /** The SQL dialect. */
    readonly dialect = "sqlite";
    /** The client running statements. */
    readonly client: SqliteQueryClient;
    /** The connection that starts transactions, absent inside one. */
    readonly #connection: SqliteConnectionClient | undefined;
    /** The savepoint depth inside a transaction. */
    readonly #depth: number;
    /** The clock counting each statement, opening and commit in flight. */
    readonly #clock: Clock;

    /** Run statements on a connection, or in a transaction at a depth, counted on a clock. */
    constructor(client: SqliteConnectionClient | SqliteQueryClient, clock: Clock, depth?: number) {
        // keep the client, the connection starting transactions outside one, the depth and the clock
        this.client = client;
        this.#connection = depth === undefined && "transactionAsync" in client ? client : undefined;
        this.#depth = depth ?? 0;
        this.#clock = clock;
    }

    /** Read every row as an array of values, integers exact. */
    values(text: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#clock.trackHost(this.client.values(text, parameters));
    }

    /** Read every row by column name. */
    async all(
        text: string,
        parameters: readonly DriverValue[],
    ): Promise<Record<string, unknown>[]> {
        const rows = await this.#clock.trackHost(this.client.all(text, parameters));

        return rows.map(requireRecord);
    }

    /** Run a statement for its effect. */
    async run(text: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#clock.trackHost(this.client.run(text, parameters));
    }

    /** Run a script of statements in one round trip. */
    async exec(script: string): Promise<void> {
        await this.#clock.trackHost(this.client.exec(script));
    }

    /** Commit work in a transaction, immediate unless read-only, or nest it in a savepoint. */
    transaction<Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ): Promise<Value> {
        // nest inside an open transaction
        const clock = this.#clock;
        const connection = this.#connection;
        if (connection === undefined) {
            return transact(clock, operation, (body) =>
                this.client.nest(this.#depth, (client) =>
                    body(new SqliteSession(client, clock, this.#depth + 1)),
                ),
            );
        }

        // start a transaction on the connection
        return transact(clock, operation, (body) => {
            const transaction = connection.transactionAsync((client) =>
                body(new SqliteSession(client, clock, 0)),
            );

            return options.isReadOnly ? transaction.deferred() : transaction.immediate();
        });
    }
}

/** Statements over a PostgreSQL pool, or one of its transactions. */
export class PostgresSession implements Session {
    /** The SQL dialect. */
    readonly dialect = "postgresql";
    /** The pool or transaction running statements. */
    readonly client: postgres.Sql | postgres.TransactionSql;
    /** The clock counting each statement, opening and commit in flight. */
    readonly #clock: Clock;

    /** Run statements on a pool or in a transaction, counted on a clock. */
    constructor(client: postgres.Sql | postgres.TransactionSql, clock: Clock) {
        this.client = client;
        this.#clock = clock;
    }

    /** Read every row as an array of values, prepared once per connection. */
    async values(text: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        const rows = await this.#clock.trackHost(
            this.client.unsafe(text, parameters.map(postgresParameter), { prepare: true }).values(),
        );

        return [...rows].map(requireArray);
    }

    /** Read every row by column name. */
    async all(
        text: string,
        parameters: readonly DriverValue[],
    ): Promise<Record<string, unknown>[]> {
        const rows = await this.#clock.trackHost(
            this.client
                .unsafe(text, parameters.map(postgresParameter), { prepare: true })
                .execute(),
        );

        return [...rows].map(requireRecord);
    }

    /** Run a statement for its effect. */
    async run(text: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#clock.trackHost(
            this.client
                .unsafe(text, parameters.map(postgresParameter), { prepare: true })
                .execute(),
        );
    }

    /** Run a script of statements as one simple query. */
    async exec(script: string): Promise<void> {
        await this.#clock.trackHost(this.client.unsafe(script).simple().execute());
    }

    /** Commit work in a transaction, or nest it in a savepoint. */
    async transaction<Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ): Promise<Value> {
        // nest inside an open transaction
        const clock = this.#clock;
        const client = this.client;
        if ("savepoint" in client) {
            return transact(clock, operation, async (body) => {
                const nested = await client.savepoint(async (transaction) => ({
                    value: await body(new PostgresSession(transaction, clock)),
                }));

                return nested.value;
            });
        }

        // begin a transaction with the isolation level and access mode
        const mode = `isolation level ${options.isolationLevel} ${options.isReadOnly ? "read only" : "read write"}`;

        return transact(clock, operation, async (body) => {
            const begun = await client.begin(mode, async (transaction) => ({
                value: await body(new PostgresSession(transaction, clock)),
            }));

            return begun.value;
        });
    }
}

/** Run a transaction's body, counting its opening and its end in flight on a clock. */
async function transact<Value>(
    clock: Clock,
    operation: (session: Session) => Promise<Value>,
    open: (body: (session: Session) => Promise<Value>) => Promise<Value>,
): Promise<Value> {
    // count the opening until the body starts, and the commit or rollback once it ends
    const opening = Promise.withResolvers<void>();
    const ending = Promise.withResolvers<void>();
    void clock.trackHost(opening.promise);
    try {
        return await open(async (session) => {
            opening.resolve();
            try {
                return await operation(session);
            } finally {
                void clock.trackHost(ending.promise);
            }
        });
    } finally {
        opening.resolve();
        ending.resolve();
    }
}

/** Require a driver row to be an array of values. */
function requireArray(row: unknown): unknown[] {
    if (!Array.isArray(row)) {
        throw new TypeError("the database driver returned a record instead of an array of values");
    }

    return row;
}

/** Require a driver row to be a record of values by column name. */
function requireRecord(row: unknown): Record<string, unknown> {
    if (!isRecord(row)) {
        throw new TypeError("the database driver returned an array of values instead of a record");
    }

    return row;
}

/** Report whether a driver row is a record. */
function isRecord(row: unknown): row is Record<string, unknown> {
    return typeof row === "object" && row !== null && !Array.isArray(row);
}

/** Pass a value to PostgreSQL, a 64-bit integer as decimal text for exact int8 columns. */
function postgresParameter(value: DriverValue): postgres.ParameterOrJSON<never> {
    return typeof value === "bigint" ? value.toString() : value;
}
