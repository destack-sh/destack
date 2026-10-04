import type { DriverValue } from "../table/column.ts";
import init, { type Database, type SqlValue } from "@sqlite.org/sqlite-wasm";
import { Savepoint, WorkQueue, type ConnectionClient, type QueryClient } from "../sqlite/client.ts";

/** Statements over one SQLite WebAssembly database. */
export class WasmQuery implements QueryClient {
    /** The database. */
    readonly database: Database;
    /** The work queue, absent within a transaction. */
    readonly queue: WorkQueue | undefined;

    /** Create the client. */
    constructor(database: Database, queue?: WorkQueue) {
        this.database = database;
        this.queue = queue;
    }

    /** Read every row as an array of values. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#schedule(async () => this.database.selectArrays(sql, bindings(parameters)));
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(async () => this.database.selectObjects(sql, bindings(parameters)));
    }

    /** Run a statement for its effect. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<void> {
        const bind = bindings(parameters);
        await this.#schedule(async () => {
            this.database.exec({ sql, ...(bind === undefined ? {} : { bind }) });
        });
    }

    /** Run a script. */
    async exec(script: string): Promise<void> {
        await this.#schedule(async () => {
            this.database.exec(script);
        });
    }

    /** Run work in a savepoint at a depth. */
    nest<Value>(depth: number, operation: (client: QueryClient) => Promise<Value>): Promise<Value> {
        return Savepoint.run(this, depth, operation);
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over one SQLite WebAssembly database, one transaction at a time. */
export class WasmClient extends WasmQuery implements ConnectionClient {
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
    transactionAsync<Value>(operation: (client: QueryClient) => Promise<Value>) {
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

/** Read parameters as bindings, booleans as SQLite integers, none when empty. */
function bindings(parameters: readonly DriverValue[]): SqlValue[] | undefined {
    return parameters.length === 0
        ? undefined
        : parameters.map((value) => (typeof value === "boolean" ? Number(value) : value));
}
