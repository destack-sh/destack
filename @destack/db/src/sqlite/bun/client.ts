/// <reference types="bun" />
import type * as bun from "bun:sqlite";
import { WorkQueue, type ConnectionClient, type QueryClient, type Statement } from "../client.ts";
import { SqliteScript } from "../script.ts";

/** The most prepared statement texts per connection: a service runs 400 to 600, at 2 to 10 KB each. */
const PREPARED_TEXTS = 512;

/** The work queue of each database file, shared by the process's connections to it. */
const FILE_QUEUES = new Map<string, WorkQueue>();

/** A Bun statement with its integer mode. */
type NativeStatement = bun.Statement & {
    /** Read integers as big integers, exactly, or as numbers. */
    safeIntegers(enabled: boolean): NativeStatement;
};

/** The result of running a statement. */
export interface RunResult {
    /** The number of rows changed. */
    readonly changes: number;
    /** The last inserted row identifier. */
    readonly lastInsertRowid: number | bigint;
}

/** Statements on one SQLite database, prepared once per text. */
export class BunQuery implements QueryClient<RunResult> {
    /** The database. */
    readonly database: bun.Database;
    /** The prepared statements. */
    readonly statements: StatementCache;
    /** The work queue, absent within a transaction. */
    readonly queue: WorkQueue | undefined;

    /** Create the client. */
    constructor(database: bun.Database, statements: StatementCache, queue?: WorkQueue) {
        this.database = database;
        this.statements = statements;
        this.queue = queue;
    }

    /** Prepare a statement on the cached native statement. */
    async prepare(sql: string): Promise<Statement<RunResult>> {
        // track the statement's modes
        let isRaw = false;
        let isSafe = false;
        const run = <Value>(execute: (native: NativeStatement) => Value) =>
            this.#schedule(async () => execute(this.statements.take(sql).safeIntegers(isSafe)));

        // set the modes on each run
        const statement: Statement<RunResult> = {
            safeIntegers: (enabled) => {
                isSafe = enabled;

                return statement;
            },
            raw: (enabled) => {
                isRaw = enabled;

                return statement;
            },
            run: (...parameters) => run((native) => native.run(...(parameters as never[]))),
            all: (...parameters) =>
                run((native) =>
                    isRaw
                        ? native.values(...(parameters as never[]))
                        : native.all(...(parameters as never[])),
                ),
            get: (...parameters) =>
                run((native) =>
                    isRaw
                        ? native.values(...(parameters as never[]))[0]
                        : native.get(...(parameters as never[])),
                ),
        };

        return statement;
    }

    /** Run a statement. */
    async run(sql: string, ...parameters: unknown[]): Promise<RunResult> {
        return (await this.prepare(sql)).run(...parameters);
    }

    /** Read rows as named objects. */
    async all(sql: string, ...parameters: unknown[]): Promise<unknown[]> {
        return (await this.prepare(sql)).all(...parameters);
    }

    /** Read the first row as a named object. */
    async get(sql: string, ...parameters: unknown[]): Promise<unknown> {
        return (await this.prepare(sql)).get(...parameters);
    }

    /** Run a script statement by statement, since Bun reports only the last failure of a whole script. */
    exec(script: string): Promise<unknown> {
        return this.#schedule(async () => {
            for (const statement of SqliteScript.statements(script)) {
                this.database.run(statement);
            }
        });
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over one SQLite database, one transaction at a time per file and process. */
export class BunClient extends BunQuery implements ConnectionClient<RunResult> {
    /** The work queue of the database file. */
    readonly #queue: WorkQueue;

    /** Create the client. */
    constructor(database: bun.Database) {
        const queue = BunClient.queue(database);
        super(database, new StatementCache(database), queue);
        this.#queue = queue;
    }

    /** Read the work queue of a database's file, or a queue of its own for a memory database. */
    static queue(database: bun.Database): WorkQueue {
        // give each memory database its own queue
        const file = database.filename;
        if (file === "" || file === ":memory:") {
            return new WorkQueue();
        }

        // share one queue per file
        let queue = FILE_QUEUES.get(file);
        if (queue === undefined) {
            queue = new WorkQueue();
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
    transactionAsync<Value>(operation: (client: QueryClient<RunResult>) => Promise<Value>) {
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
export class StatementCache {
    /** The database preparing the statements. */
    readonly #database: bun.Database;
    /** The statements by text, least recently used first. */
    readonly #statements = new Map<string, NativeStatement>();

    /** Create the cache. */
    constructor(database: bun.Database) {
        this.#database = database;
    }

    /** Take a text's statement, preparing it the first time. */
    take(sql: string): NativeStatement {
        // move the statement to the recent end
        const statement =
            this.#statements.get(sql) ?? (this.#database.prepare(sql) as NativeStatement);
        this.#statements.delete(sql);
        this.#statements.set(sql, statement);

        // finalize the least recently used beyond the bound
        if (this.#statements.size > PREPARED_TEXTS) {
            const [oldest, evicted] = this.#statements.entries().next().value!;
            this.#statements.delete(oldest);
            evicted.finalize();
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
