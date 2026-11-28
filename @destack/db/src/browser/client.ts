import type { DatabaseEnvironment } from "../database/connection.ts";
import type { DriverValue } from "../table/column.ts";
import init, {
    type Database,
    type PreparedStatement,
    type SqlValue,
} from "@sqlite.org/sqlite-wasm";
import {
    SQLITE_LIMITS,
    SqliteStatementCache,
    SqliteSavepoint,
    SqliteWorkQueue,
    MEMORY_POOL,
    type SqliteConnectionClient,
    type SqliteQueryClient,
} from "../sqlite/client.ts";
import { CoalescingVfs } from "./vfs.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The most log pages before a commit checkpoints itself, 4 MB, which a reopen replays in 0.3 s. */
const WAL_PAGES = 1000;

/** How long commits rest before the log moves into the file, past the gaps between keystrokes. */
const CHECKPOINT_REST_MILLISECONDS = 250;

/** Statements over one SQLite WebAssembly database, prepared once per text. */
export class WasmQuery implements SqliteQueryClient {
    /** The most values one statement binds, as SQLite binds by default. */
    readonly limits = SQLITE_LIMITS;
    /** The database. */
    readonly database: Database;
    /** The prepared statements. */
    readonly statements: SqliteStatementCache<PreparedStatement>;
    /** The work queue and the environment each turn waits in, absent within a transaction. */
    readonly queue:
        | { readonly work: SqliteWorkQueue; readonly environment: DatabaseEnvironment }
        | undefined;

    /** Create the client. */
    constructor(
        database: Database,
        statements: SqliteStatementCache<PreparedStatement>,
        queue?: { readonly work: SqliteWorkQueue; readonly environment: DatabaseEnvironment },
    ) {
        this.database = database;
        this.statements = statements;
        this.queue = queue;
    }

    /** Read every row as an array of values. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#schedule(async () =>
            this.#rows(sql, parameters, (statement) => statement.get([])),
        );
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(async () =>
            this.#rows(sql, parameters, (statement) => statement.get({})),
        );
    }

    /** Run a statement for its effect, answering how many rows it changed. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<number> {
        return await this.#schedule(async () => {
            this.#rows(sql, parameters, () => undefined);

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

    /** Run a text's statement, preparing it the first time, and read each row it steps to. */
    #rows<Row>(
        sql: string,
        parameters: readonly DriverValue[],
        read: (statement: PreparedStatement) => Row,
    ): Row[] {
        // bind the values, step through the rows, and reset the statement for its next run
        const statement = this.statements.take(sql);
        const bind = bindings(parameters);
        try {
            if (bind !== undefined) {
                statement.bind(bind);
            }
            const rows: Row[] = [];
            while (statement.step()) {
                rows.push(read(statement));
            }

            return rows;
        } finally {
            statement.reset(true);
        }
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
    /** The timer checkpointing once commits rest, absent while none waits. */
    #checkpoint: number | undefined;

    /** Create the client, its turns waited for in an environment. */
    constructor(database: Database, environment: DatabaseEnvironment) {
        // queue the database's work, each turn waited for in the client's environment
        const queue = new SqliteWorkQueue(environment.telemetry, MEMORY_POOL);
        const statements = new SqliteStatementCache(
            environment,
            (sql) => database.prepare(sql),
            (statement: PreparedStatement) => statement.finalize(),
        );
        super(database, statements, {
            work: queue,
            environment,
        });
        this.#queue = queue;
        this.#environment = environment;
        this.#checkpoint = undefined;
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
        // load the runtime, then take the database's pool of access handles, each step in its span
        const scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        const sqlite = await scope.span("destack.database.runtime", {}, () =>
            WasmClient.#sqlite(environment),
        );
        const pool = await scope.span("destack.database.pool", {}, () =>
            sqlite.installOpfsSAHPoolVfs({ name: `destack-${name}` }),
        );

        // open the file in its span
        return scope.span("destack.database.file", {}, async () => {
            // open the file through the pool, coalescing each commit's writes
            const coalescing = new CoalescingVfs(sqlite, pool.vfsName, environment.telemetry);
            const database = new sqlite.oo1.DB({ filename: `/${name}`, vfs: coalescing.name });

            // write ahead without shared memory, as the worker holds the file alone, durable at each commit
            database.exec(
                `PRAGMA foreign_keys = ON; PRAGMA locking_mode = EXCLUSIVE; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA wal_autocheckpoint = ${WAL_PAGES};`,
            );

            return new WasmClient(database, environment);
        });
    }

    /** Read the realm's SQLite runtime, loading it at the first database. */
    static #sqlite(environment: DatabaseEnvironment): ReturnType<typeof init> {
        WasmClient.#runtime ??= environment.scheduler.run(() => init());

        return WasmClient.#runtime;
    }

    /** Close the database after its work, which checkpoints it. */
    async close(): Promise<void> {
        this.#environment.clock.clearTimeout(this.#checkpoint);
        this.#checkpoint = undefined;
        await this.#queue.run(async () => {
            this.statements.close();
            this.database.close();
        }, this.#environment);
    }

    /** Run a callback in a transaction. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>) {
        const begin = (mode: "DEFERRED" | "IMMEDIATE" | "EXCLUSIVE") =>
            this.#queue.run(async () => {
                // commit or roll back by the callback's outcome
                this.database.exec(`BEGIN ${mode}`);
                try {
                    const value = await operation(new WasmQuery(this.database, this.statements));
                    this.database.exec("COMMIT");
                    this.#checkpointAtRest();

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

    /** Move the write-ahead log into the file once commits rest, off every write's path. */
    #checkpointAtRest(): void {
        // wait again from the latest commit
        const { clock } = this.#environment;
        clock.clearTimeout(this.#checkpoint);

        // checkpoint in its own turn, behind any work queued by then
        this.#checkpoint = clock.setTimeout(() => {
            this.#checkpoint = undefined;
            const checkpoint = async () => {
                this.database.exec("PRAGMA wal_checkpoint(PASSIVE)");
            };
            this.#queue
                .run(checkpoint, this.#environment)
                .catch((error: unknown) => globalThis.reportError(error));
        }, CHECKPOINT_REST_MILLISECONDS);
    }
}

/** Read parameters as bindings, booleans as SQLite integers, none when empty. */
function bindings(parameters: readonly DriverValue[]): SqlValue[] | undefined {
    return parameters.length === 0
        ? undefined
        : parameters.map((value) => (typeof value === "boolean" ? Number(value) : value));
}
