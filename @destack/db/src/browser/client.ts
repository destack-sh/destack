import type { DatabaseEnvironment } from "../database/connection.ts";
import type { DriverValue } from "../table/column.ts";
import init, { type Database, type SqlValue } from "@sqlite.org/sqlite-wasm";
import {
    SQLITE_LIMITS,
    SqliteSavepoint,
    SqliteWorkQueue,
    MEMORY_POOL,
    type SqliteConnectionClient,
    type SqliteQueryClient,
} from "../sqlite/client.ts";
import { CoalescingVfs } from "./vfs.ts";

/** The most write-ahead pages before a checkpoint, 400 KB, which a reopen replays in ~30 ms. */
const WAL_PAGES = 100;

/** Statements over one SQLite WebAssembly database. */
export class WasmQuery implements SqliteQueryClient {
    /** The most values one statement binds, as SQLite binds by default. */
    readonly limits = SQLITE_LIMITS;
    /** The database. */
    readonly database: Database;
    /** The work queue and the environment each turn waits in, absent within a transaction. */
    readonly queue:
        | { readonly work: SqliteWorkQueue; readonly environment: DatabaseEnvironment }
        | undefined;

    /** Create the client. */
    constructor(
        database: Database,
        queue?: { readonly work: SqliteWorkQueue; readonly environment: DatabaseEnvironment },
    ) {
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

    /** Run a statement for its effect, answering how many rows it changed. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<number> {
        const bind = bindings(parameters);

        return await this.#schedule(async () => {
            this.database.exec({ sql, ...(bind === undefined ? {} : { bind }) });

            return this.database.changes();
        });
    }

    /** Run a script. */
    async exec(script: string): Promise<void> {
        await this.#schedule(async () => {
            this.database.exec(script);
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
        return this.queue === undefined
            ? work()
            : this.queue.work.run(work, this.queue.environment);
    }
}

/** A connection client over one SQLite WebAssembly database, one transaction at a time. */
export class WasmClient extends WasmQuery implements SqliteConnectionClient {
    /** The realm's SQLite runtime, compiled and instantiated once, as a realm loads a library. */
    static #runtime: ReturnType<typeof init> | undefined;

    /** The work queue. */
    readonly #queue: SqliteWorkQueue;
    /** The environment the client's turns wait in. */
    readonly #environment: DatabaseEnvironment;

    /** Create the client, its turns waited for in an environment. */
    constructor(database: Database, environment: DatabaseEnvironment) {
        // queue the database's work, each turn waited for in the client's environment
        const queue = new SqliteWorkQueue(environment.telemetry, MEMORY_POOL);
        super(database, { work: queue, environment });
        this.#queue = queue;
        this.#environment = environment;
    }

    /** Open an in-memory database. */
    static async memory(environment: DatabaseEnvironment): Promise<WasmClient> {
        // open with foreign keys enforced
        const sqlite = await WasmClient.#sqlite(environment);
        const database = new sqlite.oo1.DB(":memory:");
        database.exec("PRAGMA foreign_keys = ON;");

        return new WasmClient(database, environment);
    }

    /** Open a file of the origin's private file system, which one worker holds at a time. */
    static async opfs(name: string, environment: DatabaseEnvironment): Promise<WasmClient> {
        // open the file through the database's pool of access handles, coalescing each commit's writes
        const sqlite = await WasmClient.#sqlite(environment);
        const pool = await sqlite.installOpfsSAHPoolVfs({ name: `destack-${name}` });
        const coalescing = new CoalescingVfs(sqlite, pool.vfsName, environment.telemetry);
        const database = new sqlite.oo1.DB({ filename: `/${name}`, vfs: coalescing.name });

        // write ahead without shared memory, as the worker holds the file alone, durable at each commit
        database.exec(
            `PRAGMA foreign_keys = ON; PRAGMA locking_mode = EXCLUSIVE; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA wal_autocheckpoint = ${WAL_PAGES};`,
        );

        return new WasmClient(database, environment);
    }

    /** Read the realm's SQLite runtime, loading it at the first database. */
    static #sqlite(environment: DatabaseEnvironment): ReturnType<typeof init> {
        WasmClient.#runtime ??= environment.scheduler.run(() => init());

        return WasmClient.#runtime;
    }

    /** Close the database after its work. */
    async close(): Promise<void> {
        await this.#queue.run(async () => this.database.close(), this.#environment);
    }

    /** Run a callback in a transaction. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>) {
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
            }, this.#environment);

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
