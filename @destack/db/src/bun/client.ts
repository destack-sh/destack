import type { DatabaseEnvironment } from "../database/connection.ts";
/// <reference types="bun" />
import * as bun from "bun:sqlite";
import type { DriverValue } from "../table/column.ts";
import {
    SQLITE_LIMITS,
    SqliteSavepoint,
    SqliteWorkQueue,
    MEMORY_POOL,
    type SqliteConnectionClient,
    type SqliteQueryClient,
} from "../sqlite/client.ts";
import { SqliteScript } from "../sqlite/script.ts";
import { DriverStatement } from "../sql/render.ts";
import { present } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { Cache } from "../cache/cache.ts";
import type {} from "@destack/package/import-meta";

/** The most prepared statement texts per connection: a package runs 400 to 600 of 2 to 10 KB. */
const PREPARED_TEXTS = 512;

/** The work queue of each database file by its real path: one writer per file in the process. */
const FILE_QUEUES = new Map<string, SqliteWorkQueue>();

/** The wait for a WAL checkpoint in milliseconds, the writer's own busy timeout. */
const READER_BUSY_TIMEOUT_MILLISECONDS = 5000;

/** Statements on one SQLite database, prepared once per text. */
export class BunQuery implements SqliteQueryClient {
    /** The most values one statement binds, as SQLite binds by default. */
    readonly limits = SQLITE_LIMITS;
    /** The database. */
    readonly database: bun.Database;
    /** The prepared statements. */
    readonly statements: BunStatementCache;
    /** The work queue and the environment each turn waits in, absent within a transaction. */
    readonly queue:
        | { readonly work: SqliteWorkQueue; readonly environment: DatabaseEnvironment }
        | undefined;

    /** Create the client. */
    constructor(
        database: bun.Database,
        statements: BunStatementCache,
        queue?: { readonly work: SqliteWorkQueue; readonly environment: DatabaseEnvironment },
    ) {
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

    /** Run a statement for its effect, answering how many rows it changed. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<number> {
        return await this.#schedule(
            async () => this.statements.take(sql).run(...parameters).changes,
        );
    }

    /** Run a script statement by statement, since Bun reports only a script's last failure. */
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
        return this.queue === undefined
            ? work()
            : this.queue.work.run(work, this.queue.environment);
    }
}

/** A connection client over one SQLite database, one transaction at a time per file and process. */
export class BunClient extends BunQuery implements SqliteConnectionClient {
    /** The work queue of the database file. */
    readonly #queue: SqliteWorkQueue;
    /** The environment the client's turns wait in. */
    readonly #environment: DatabaseEnvironment;
    /** The database's file, empty in memory. */
    readonly #file: string;
    /** The query-only connection to the file, opened on first use. */
    #reader: BunQuery | undefined;

    /** Create the client, its turns waited for in an environment. */
    constructor(database: bun.Database, environment: DatabaseEnvironment) {
        // share the file's queue, each turn waited for in the client's environment
        const queue = BunClient.queue(database, environment);
        super(database, new BunStatementCache(database, environment), {
            work: queue,
            environment,
        });
        this.#queue = queue;
        this.#environment = environment;

        // remember the file, which a reader opens beside the writer
        this.#file = BunClient.file(database);
    }

    /** Read every row as an array of values, a read beside the writer. */
    override values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        const reader = DriverStatement.isRead(sql) ? this.#readerMaybe() : undefined;

        return reader === undefined
            ? super.values(sql, parameters)
            : reader.values(sql, parameters);
    }

    /** Read every row by column name, a read beside the writer. */
    override all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        const reader = DriverStatement.isRead(sql) ? this.#readerMaybe() : undefined;

        return reader === undefined ? super.all(sql, parameters) : reader.all(sql, parameters);
    }

    /** Open the query-only connection on first use, none in memory. */
    #readerMaybe(): BunQuery | undefined {
        // open the reader on the first read of a file
        if (this.#reader === undefined && this.#file !== "") {
            // open it writable for the WAL index, refusing writes
            const reader = new bun.Database(this.#file, { readwrite: true, create: false });
            reader.run(`PRAGMA busy_timeout = ${READER_BUSY_TIMEOUT_MILLISECONDS}`);
            reader.run("PRAGMA query_only = ON");
            this.#reader = new BunQuery(reader, new BunStatementCache(reader, this.#environment));
        }

        return this.#reader;
    }

    /** Read the work queue of a database's file by its real path, or a separate one in memory. */
    static queue(database: bun.Database, environment: DatabaseEnvironment): SqliteWorkQueue {
        // give each memory database a separate queue
        const file = BunClient.file(database);
        if (file === "") {
            return new SqliteWorkQueue(environment.telemetry, MEMORY_POOL);
        }

        // share one queue per file, however its path was spelled
        const known = FILE_QUEUES.get(file);
        if (known !== undefined) {
            return known;
        }
        const name = file.slice(file.lastIndexOf("/") + 1);
        const queue = new SqliteWorkQueue(environment.telemetry, name);
        FILE_QUEUES.set(file, queue);

        return queue;
    }

    /** Read the real path SQLite resolved for a database's main file, empty in memory. */
    static file(database: bun.Database): string {
        const listed = database
            .query<{ name: string; file: string }, []>("PRAGMA database_list")
            .all();

        return present(
            listed.find((entry) => entry.name === "main"),
            "the database's main file",
        ).file;
    }

    /** Commit the transaction, adding the failing row to a foreign key error. */
    #commit(): void {
        try {
            this.database.run("COMMIT");
        } catch (error) {
            // read the first row with a missing parent
            const failing = this.database
                .query<{ table: string; rowid: number; parent: string }, []>(
                    "PRAGMA foreign_key_check",
                )
                .get();

            // pass any other failure on
            if (failing === null) {
                throw error;
            }
            const { table, rowid, parent } = failing;

            throw new Error(
                `FOREIGN KEY constraint failed: '${table}' row '${rowid}' lacks its '${parent}'`,
                { cause: error },
            );
        }
    }

    /** Close the database after its work. */
    async close(): Promise<void> {
        await this.#queue.run(async () => {
            // close the writer, then the reader
            this.statements.close();
            this.database.close();
            this.#reader?.statements.close();
            this.#reader?.database.close();
        }, this.#environment);
    }

    /** Run a callback in a transaction. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>) {
        const begin = (mode: "DEFERRED" | "IMMEDIATE" | "EXCLUSIVE") =>
            this.#queue.run(async () => {
                // commit or roll back by the callback's outcome
                this.database.run(`BEGIN ${mode}`);
                try {
                    const value = await operation(new BunQuery(this.database, this.statements));
                    this.#commit();

                    return value;
                } catch (error) {
                    this.database.run("ROLLBACK");
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

/** The prepared statements of one connection by text, least recently used first. */
export class BunStatementCache {
    /** The database preparing the statements. */
    readonly #database: bun.Database;
    /** The statements by text, finalized once dropped past the bound. */
    readonly #statements: Cache<bun.Statement<unknown, bun.SQLQueryBindings[]>>;

    /** Create the cache, counting its takes and evictions as the statement cache. */
    constructor(database: bun.Database, environment: DatabaseEnvironment) {
        this.#database = database;
        this.#statements = new Cache({
            name: "statement",
            capacity: PREPARED_TEXTS,
            scope: TelemetryScope.of(import.meta.destack.package, environment.telemetry),
            evicted: (statement) => statement.finalize(),
        });
    }

    /** Take a text's statement, preparing it the first time. */
    take(sql: string): bun.Statement<unknown, bun.SQLQueryBindings[]> {
        return this.#statements.take(sql, () =>
            this.#database.prepare<unknown, bun.SQLQueryBindings[]>(sql),
        );
    }

    /** Finalize every statement. */
    close(): void {
        for (const statement of this.#statements.values()) {
            statement.finalize();
        }
        this.#statements.clear();
    }
}
