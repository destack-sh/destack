import type { Clock } from "@destack/environment";
import type postgres from "postgres";
import type { Dialect } from "../dialect/dialect.ts";
import { quote } from "../dialect/quote.ts";
import { DatabaseError } from "../error/error.ts";
import type { PostgresPool, PostgresReservation } from "../postgres/pool.ts";
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

/** Where a PostgreSQL session runs: on a pool, or in a transaction at a savepoint depth. */
export type PostgresConnection =
    | { readonly pool: PostgresPool }
    | { readonly transaction: postgres.Sql; readonly depth: number };

/** Statements over a PostgreSQL pool, or one of its transactions on a reserved connection. */
export class PostgresSession implements Session {
    /** The SQL dialect. */
    readonly dialect = "postgresql";
    /** The pool, or the transaction's reserved connection and savepoint depth. */
    readonly connection: PostgresConnection;
    /** The schema every statement runs in. */
    readonly schema: string;
    /** The clock counting each statement, opening and commit in flight. */
    readonly #clock: Clock;
    /** Whether a transaction nested in this one is open, which no sibling may join. */
    #isNesting = false;

    /** Run statements on a pool or in a transaction, in a schema, on a clock. */
    constructor(connection: PostgresConnection, schema: string, clock: Clock) {
        this.connection = connection;
        this.schema = schema;
        this.#clock = clock;
    }

    /** Read every row as an array of values, planned per run, as schemas share statements' text. */
    async values(text: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        const rows = await this.#statement((client) =>
            client
                .unsafe(text, parameters.map(postgresParameter), { prepare: false })
                .values()
                .execute(),
        );

        return [...rows].map(requireArray);
    }

    /** Read every row by column name. */
    async all(
        text: string,
        parameters: readonly DriverValue[],
    ): Promise<Record<string, unknown>[]> {
        const rows = await this.#statement((client) =>
            client.unsafe(text, parameters.map(postgresParameter), { prepare: false }).execute(),
        );

        return [...rows].map(requireRecord);
    }

    /** Run a statement for its effect. */
    async run(text: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#statement((client) =>
            client.unsafe(text, parameters.map(postgresParameter), { prepare: false }).execute(),
        );
    }

    /** Run a script of statements as one simple query, one implicit transaction outside one. */
    async exec(script: string): Promise<void> {
        await this.#statement((client) => client.unsafe(script).simple().execute());
    }

    /** Commit work in a transaction on a reserved connection, or nest it in a savepoint. */
    async transaction<Value>(
        operation: (session: Session) => Promise<Value>,
        options: SessionTransaction,
    ): Promise<Value> {
        // reserve a connection in the schema, then begin on it
        const { connection, schema } = this;
        const clock = this.#clock;
        if ("pool" in connection) {
            const { pool } = connection;
            const access = options.isReadOnly ? "READ ONLY" : "READ WRITE";
            const open = `BEGIN ISOLATION LEVEL ${options.isolationLevel} ${access}`;
            const reserved = await pool.reserve(clock, schema);
            const transaction = reserved.sql;
            try {
                return await transact(clock, operation, (body) =>
                    block(reserved, { open, commit: "COMMIT", rollback: "ROLLBACK" }, () =>
                        body(new PostgresSession({ transaction, depth: 0 }, schema, clock)),
                    ),
                );
            } finally {
                pool.release(transaction);
            }
        }

        // refuse a second nested transaction, whose statements would mix with its sibling's
        if (this.#isNesting) {
            throw DatabaseError.of(
                "QUERY_INVALID",
                "a nested transaction began while its sibling is open: run them in turn",
            );
        }

        // nest at the depth's savepoint, alone until it ends
        const { transaction, depth } = connection;
        const savepoint = quote(`savepoint_${depth}`);
        this.#isNesting = true;

        return await transact(clock, operation, (body) =>
            block(
                { sql: transaction, ready: Promise.resolve() },
                {
                    open: `SAVEPOINT ${savepoint}`,
                    commit: `RELEASE SAVEPOINT ${savepoint}`,
                    rollback: `ROLLBACK TO SAVEPOINT ${savepoint}`,
                },
                () => body(new PostgresSession({ transaction, depth: depth + 1 }, schema, clock)),
            ),
        ).finally(() => {
            this.#isNesting = false;
        });
    }

    /** Run one statement, in the open transaction or on its own in one round trip. */
    async #statement<Result>(run: (client: postgres.Sql) => Promise<Result>): Promise<Result> {
        // run it in the open transaction, nested ones included, as the log identity stamps there
        const { connection, schema } = this;
        if (!("pool" in connection)) {
            return await this.#clock.trackHost(run(connection.transaction));
        }

        // send the statement after any search path change on a connection reserved in the schema
        const { pool } = connection;
        const clock = this.#clock;
        const { sql, ready } = await pool.reserve(clock, schema);
        try {
            const settled = await clock.trackHost(Promise.allSettled([ready, run(sql)]));

            // answer the statement's result, or fail with the first failure
            const [changed, ran] = settled;
            if (changed.status === "fulfilled" && ran.status === "fulfilled") {
                return ran.value;
            }
            throw firstFailure(settled);
        } finally {
            pool.release(sql);
        }
    }
}

/** Run a body between the statements opening and committing a block, rolling back on failure. */
async function block<Value>(
    reserved: PostgresReservation,
    statements: { readonly open: string; readonly commit: string; readonly rollback: string },
    body: () => Promise<Value>,
): Promise<Value> {
    // send the opening before the body's statements, after the connection's search path
    const client = reserved.sql;
    const settled = await Promise.allSettled([
        reserved.ready,
        client.unsafe(statements.open).simple().execute(),
        body(),
    ]);

    // commit a block whose opening and body succeeded, refusing a commit the server rolled back
    const [changed, opened, ran] = settled;
    if (
        changed.status === "fulfilled" &&
        opened.status === "fulfilled" &&
        ran.status === "fulfilled"
    ) {
        const committed = await client.unsafe(statements.commit).simple().execute();
        if (committed.command === "ROLLBACK") {
            throw DatabaseError.of(
                "QUERY_FAILED",
                "the transaction rolled back at its commit, after a statement in it failed",
            );
        }

        return ran.value;
    }

    // roll back a failed block, then fail with its first failure
    const failure = firstFailure(settled);
    try {
        await client.unsafe(statements.rollback).simple().execute();
    } catch (cleanup) {
        throw new AggregateError([failure, cleanup], "transaction and rollback failed", {
            cause: cleanup,
        });
    }
    throw failure;
}

/** Read the first failure of settled work that failed, refusing work that did not. */
function firstFailure(settled: readonly PromiseSettledResult<unknown>[]): unknown {
    for (const each of settled) {
        if (each.status === "rejected") {
            const reason: unknown = each.reason;

            return reason;
        }
    }

    throw new TypeError("the settled work has no failure");
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
