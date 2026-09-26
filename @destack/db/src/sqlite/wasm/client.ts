import init, { type Database, type SqlValue } from "@sqlite.org/sqlite-wasm";
import type { ConnectionClient, QueryClient, Statement } from "../client.ts";

/** The result of running a statement: the rows it changed and the last inserted row. */
export interface RunResult {
    /** The number of rows changed. */
    readonly changes: number;
    /** The row identifier of the last insertion. */
    readonly lastInsertRowid: number | bigint;
}

/** Statements over one SQLite WebAssembly database, queued behind the connection's earlier work outside a transaction. */
export class WasmQuery implements QueryClient<RunResult> {
    /** The database. */
    readonly database: Database;
    /** The queue of work waiting for the connection, absent within the transaction holding it. */
    readonly queue: WorkQueue | undefined;

    /** Run statements on a database, queued when a queue is given. */
    constructor(database: Database, queue?: WorkQueue) {
        this.database = database;
        this.queue = queue;
    }

    /** Prepare a statement keeping its row mode; integers come back exact, as numbers or big integers. */
    async prepare(sql: string): Promise<Statement<RunResult>> {
        // track the row mode the statement runs in
        let isRaw = false;
        const run = (method: "run" | "all" | "get", parameters: readonly unknown[]) =>
            this.#schedule(async () => execute(this.database, sql, method, parameters, isRaw));

        // change the row mode in place, as other statements do
        const statement: Statement<RunResult> = {
            safeIntegers: () => statement,
            raw: (enabled) => {
                isRaw = enabled;

                return statement;
            },
            run: (...parameters) => run("run", parameters) as Promise<RunResult>,
            all: (...parameters) => run("all", parameters) as Promise<unknown[]>,
            get: (...parameters) => run("get", parameters),
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

    /** Run a script. */
    exec(script: string): Promise<unknown> {
        return this.#schedule(async () => this.database.exec(script));
    }

    /** Run work behind the queue, or at once within the transaction holding the connection. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over one SQLite WebAssembly database, running one transaction at a time. */
export class WasmClient extends WasmQuery implements ConnectionClient<RunResult> {
    /** The queue of work waiting for the connection. */
    readonly #queue: WorkQueue;

    /** Serve one database, queuing its work. */
    constructor(database: Database) {
        const queue = new WorkQueue();
        super(database, queue);
        this.#queue = queue;
    }

    /** Open an in-memory database, for tests and short-lived work. */
    static async memory(): Promise<WasmClient> {
        // open the database, enforcing foreign keys as every connection does
        const sqlite = await init();
        const database = new sqlite.oo1.DB(":memory:");
        database.exec("PRAGMA foreign_keys = ON;");

        return new WasmClient(database);
    }

    /** Close the database once its work finished. */
    async close(): Promise<void> {
        await this.#queue.run(async () => this.database.close());
    }

    /** Run a callback in a transaction holding the connection until it ends. */
    transactionAsync<Value>(operation: (client: QueryClient<RunResult>) => Promise<Value>) {
        const begin = (mode: "DEFERRED" | "IMMEDIATE" | "EXCLUSIVE") =>
            this.#queue.run(async () => {
                // run the callback against the connection itself, committing or rolling back by its outcome
                this.database.exec(`BEGIN ${mode}`);
                try {
                    const value = await operation(new WasmQuery(this.database));
                    this.database.exec("COMMIT");

                    return value;
                } catch (error) {
                    this.database.exec("ROLLBACK");
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

/** Work run one piece at a time, in the order it arrived. */
export class WorkQueue {
    /** The tail of the waiting work. */
    #tail: Promise<unknown> = Promise.resolve();

    /** Run work once the earlier work finished. */
    run<Value>(work: () => Promise<Value>): Promise<Value> {
        const result = this.#tail.then(work);
        this.#tail = result.catch(() => undefined);

        return result;
    }
}

/** Execute one statement, returning its change count, its rows or its first row. */
function execute(
    database: Database,
    sql: string,
    method: "run" | "all" | "get",
    parameters: readonly unknown[],
    isRaw: boolean,
): unknown {
    // run a changing statement, reporting what it changed
    const bind = parameters.length === 0 ? undefined : (parameters as SqlValue[]);
    if (method === "run") {
        database.exec({ sql, ...(bind === undefined ? {} : { bind }) });

        return {
            changes: database.changes(),
            lastInsertRowid: database.selectValue("SELECT last_insert_rowid()") as number | bigint,
        };
    }

    // read the rows as arrays or objects
    const rows = isRaw ? database.selectArrays(sql, bind) : database.selectObjects(sql, bind);

    return method === "all" ? rows : rows[0];
}
