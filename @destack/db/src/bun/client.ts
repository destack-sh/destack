/// <reference types="bun" />
import type * as bun from "bun:sqlite";
import type { DriverValue } from "../table/column.ts";
import {
    SqliteSavepoint,
    SqliteWorkQueue,
    type SqliteConnectionClient,
    type SqliteQueryClient,
} from "../sqlite/client.ts";
import { SqliteScript } from "../sqlite/script.ts";

/** The most prepared statement texts per connection: a service runs 400 to 600, at 2 to 10 KB each. */
const PREPARED_TEXTS = 512;

/** The work queue of each database file, shared by the process's connections to it. */
const FILE_QUEUES = new Map<string, SqliteWorkQueue>();

/** Bun's SQLite statements, with the integer mode Bun has beside its declared methods. */
declare module "bun:sqlite" {
    /** A Bun statement, with the integer mode Bun has beside its declared methods. */
    interface Statement<ReturnType, ParamsType> {
        /** Read integers as big integers, exactly, or as numbers. */
        safeIntegers(enabled: boolean): Statement<ReturnType, ParamsType>;
    }
}

/** Statements on one SQLite database, prepared once per text. */
export class BunQuery implements SqliteQueryClient {
    /** The database. */
    readonly database: bun.Database;
    /** The prepared statements. */
    readonly statements: BunStatementCache;
    /** The work queue, absent within a transaction. */
    readonly queue: SqliteWorkQueue | undefined;

    /** Create the client. */
    constructor(database: bun.Database, statements: BunStatementCache, queue?: SqliteWorkQueue) {
        this.database = database;
        this.statements = statements;
        this.queue = queue;
    }

    /** Read every row as an array of values, integers exact. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#schedule(async () =>
            this.statements
                .take(sql)
                .safeIntegers(true)
                .values(...parameters),
        );
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(async () =>
            this.statements
                .take(sql)
                .safeIntegers(false)
                .all(...parameters),
        );
    }

    /** Run a statement for its effect. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#schedule(async () => this.statements.take(sql).run(...parameters));
    }

    /** Run a script statement by statement, since Bun reports only the last failure of a whole script. */
    exec(script: string): Promise<void> {
        return this.#schedule(async () => {
            for (const statement of SqliteScript.statements(script)) {
                this.database.run(statement);
            }
        });
    }

    /** Run work in a savepoint at a depth. */
    nest<Value>(
        depth: number,
        operation: (client: SqliteQueryClient) => Promise<Value>,
    ): Promise<Value> {
        return SqliteSavepoint.run(this, depth, operation);
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over one SQLite database, one transaction at a time per file and process. */
export class BunClient extends BunQuery implements SqliteConnectionClient {
    /** The work queue of the database file. */
    readonly #queue: SqliteWorkQueue;

    /** Create the client. */
    constructor(database: bun.Database) {
        const queue = BunClient.queue(database);
        super(database, new BunStatementCache(database), queue);
        this.#queue = queue;
    }

    /** Read the work queue of a database's file, or a queue of its own for a memory database. */
    static queue(database: bun.Database): SqliteWorkQueue {
        // give each memory database its own queue
        const file = database.filename;
        if (file === "" || file === ":memory:") {
            return new SqliteWorkQueue();
        }

        // share one queue per file
        let queue = FILE_QUEUES.get(file);
        if (queue === undefined) {
            queue = new SqliteWorkQueue();
            FILE_QUEUES.set(file, queue);
        }

        return queue;
    }

    /** Close the database after its work. */
    async close(): Promise<void> {
        await this.#queue.run(async () => {
            this.statements.close();
            this.database.close();
        });
    }

    /** Run a callback in a transaction. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>) {
        const begin = (mode: "DEFERRED" | "IMMEDIATE" | "EXCLUSIVE") =>
            this.#queue.run(async () => {
                // commit or roll back by the callback's outcome
                this.database.run(`BEGIN ${mode}`);
                try {
                    const value = await operation(new BunQuery(this.database, this.statements));
                    this.database.run("COMMIT");

                    return value;
                } catch (error) {
                    this.database.run("ROLLBACK");
                    throw error;
                }
            });

        return {
            deferred: () => begin("DEFERRED"),
            immediate: () => begin("IMMEDIATE"),
            exclusive: () => begin("EXCLUSIVE"),
        };
    }
}

/**
 * The prepared statements of one connection by text, least recently used first.
 *
 * Statements run synchronously, and one statement serves each text.
 */
export class BunStatementCache {
    /** The database preparing the statements. */
    readonly #database: bun.Database;
    /** The statements by text, least recently used first. */
    readonly #statements = new Map<string, bun.Statement<unknown, bun.SQLQueryBindings[]>>();

    /** Create the cache. */
    constructor(database: bun.Database) {
        this.#database = database;
    }

    /** Take a text's statement, preparing it the first time. */
    take(sql: string): bun.Statement<unknown, bun.SQLQueryBindings[]> {
        // move the statement to the recent end
        const statement =
            this.#statements.get(sql) ??
            this.#database.prepare<unknown, bun.SQLQueryBindings[]>(sql);
        this.#statements.delete(sql);
        this.#statements.set(sql, statement);

        // finalize the least recently used beyond the bound
        if (this.#statements.size > PREPARED_TEXTS) {
            for (const [oldest, evicted] of this.#statements) {
                this.#statements.delete(oldest);
                evicted.finalize();
                break;
            }
        }

        return statement;
    }

    /** Finalize every statement. */
    close(): void {
        for (const statement of this.#statements.values()) {
            statement.finalize();
        }
        this.#statements.clear();
    }
}
