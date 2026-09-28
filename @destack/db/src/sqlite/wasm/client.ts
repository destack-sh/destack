import init, { type Database, type SqlValue } from "@sqlite.org/sqlite-wasm";
import { WorkQueue, type ConnectionClient, type QueryClient, type Statement } from "../client.ts";

/** The result of running a statement. */
export interface RunResult {
    /** The number of rows changed. */
    readonly changes: number;
    /** The last inserted row identifier. */
    readonly lastInsertRowid: number | bigint;
}

/** Statements over one SQLite WebAssembly database. */
export class WasmQuery implements QueryClient<RunResult> {
    /** The database. */
    readonly database: Database;
    /** The work queue, absent within a transaction. */
    readonly queue: WorkQueue | undefined;

    /** Create the client. */
    constructor(database: Database, queue?: WorkQueue) {
        this.database = database;
        this.queue = queue;
    }

    /** Prepare a statement with its row mode. */
    async prepare(sql: string): Promise<Statement<RunResult>> {
        // track the row mode
        let isRaw = false;
        const run = (method: "run" | "all" | "get", parameters: readonly unknown[]) =>
            this.#schedule(async () => execute(this.database, sql, method, parameters, isRaw));

        // set the row mode on each run
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

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over one SQLite WebAssembly database, one transaction at a time. */
export class WasmClient extends WasmQuery implements ConnectionClient<RunResult> {
    /** The work queue. */
    readonly #queue: WorkQueue;

    /** Create the client. */
    constructor(database: Database) {
        const queue = new WorkQueue();
        super(database, queue);
        this.#queue = queue;
    }

    /** Open an in-memory database. */
    static async memory(): Promise<WasmClient> {
        // open with foreign keys enforced
        const sqlite = await init();
        const database = new sqlite.oo1.DB(":memory:");
        database.exec("PRAGMA foreign_keys = ON;");

        return new WasmClient(database);
    }

    /** Close the database after its work. */
    async close(): Promise<void> {
        await this.#queue.run(async () => this.database.close());
    }

    /** Run a callback in a transaction. */
    transactionAsync<Value>(operation: (client: QueryClient<RunResult>) => Promise<Value>) {
        const begin = (mode: "DEFERRED" | "IMMEDIATE" | "EXCLUSIVE") =>
            this.#queue.run(async () => {
                // commit or roll back by the callback's outcome
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

/** Execute one statement. */
function execute(
    database: Database,
    sql: string,
    method: "run" | "all" | "get",
    parameters: readonly unknown[],
    isRaw: boolean,
): unknown {
    // run a changing statement
    const bind = parameters.length === 0 ? undefined : (parameters as SqlValue[]);
    if (method === "run") {
        database.exec({ sql, ...(bind === undefined ? {} : { bind }) });

        return {
            changes: database.changes(),
            lastInsertRowid: database.selectValue("SELECT last_insert_rowid()") as number | bigint,
        };
    }

    // read rows as arrays or objects
    const rows = isRaw ? database.selectArrays(sql, bind) : database.selectObjects(sql, bind);

    return method === "all" ? rows : rows[0];
}
