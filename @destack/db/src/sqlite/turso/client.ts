import type * as turso from "@tursodatabase/database";
import { WorkQueue, type ConnectionClient, type QueryClient, type Statement } from "../client.ts";

/** A statement Turso prepared on one connection. */
type NativeStatement = Awaited<ReturnType<turso.Database["prepare"]>>;

/** The most statement texts a connection keeps prepared, each a few kilobytes of engine state. */
const PREPARED_TEXTS = 512;

/** The result of running a statement: the rows it changed and the last inserted row. */
export type RunResult = Awaited<ReturnType<NativeStatement["run"]>>;

/** Statements on one Turso database, prepared once per text and row mode, queued behind a transaction holding the connection. */
export class TursoQuery implements QueryClient<RunResult> {
    /** The database. */
    readonly database: turso.Database;
    /** The prepared statements the connection keeps. */
    readonly statements: StatementPool;
    /** The queue of work waiting for the connection, absent within the transaction holding it. */
    readonly queue: WorkQueue | undefined;

    /** Run statements on a database, queued when a queue is given. */
    constructor(database: turso.Database, statements: StatementPool, queue?: WorkQueue) {
        this.database = database;
        this.statements = statements;
        this.queue = queue;
    }

    /** Prepare a statement that runs on a prepared statement of its text and current row modes. */
    async prepare(sql: string): Promise<Statement<RunResult>> {
        // track the modes the statement runs in
        let isRaw = false;
        let isSafe = false;
        const run = <Value>(execute: (native: NativeStatement) => Promise<Value>) =>
            this.#schedule(() => this.statements.use(sql, isRaw, isSafe, execute));

        // change modes in place, as other statements do
        const statement: Statement<RunResult> = {
            safeIntegers: (enabled) => {
                isSafe = enabled;

                return statement;
            },
            raw: (enabled) => {
                isRaw = enabled;

                return statement;
            },
            run: (...parameters) => run((native) => native.run(...parameters)),
            all: (...parameters) => run((native) => native.all(...parameters)),
            get: (...parameters) => run((native) => native.get(...parameters)),
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
        return this.#schedule(() => this.database.exec(script));
    }

    /** Run work behind the queue, or at once within the transaction holding the connection. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over one Turso database: one transaction at a time, statements prepared once and reused across transactions. */
export class TursoClient extends TursoQuery implements ConnectionClient<RunResult> {
    /** The queue of work waiting for the connection. */
    readonly #queue: WorkQueue;

    /** Serve one database, queuing its work. */
    constructor(database: turso.Database) {
        const queue = new WorkQueue();
        super(database, new StatementPool(database), queue);
        this.#queue = queue;
    }

    /** Close the database once its work finished. */
    async close(): Promise<void> {
        await this.#queue.run(async () => {
            this.statements.close();
            await this.database.close();
        });
    }

    /** Run a callback in a transaction holding the connection until it ends. */
    transactionAsync<Value>(operation: (client: QueryClient<RunResult>) => Promise<Value>) {
        const begin = (mode: "DEFERRED" | "IMMEDIATE" | "EXCLUSIVE") =>
            this.#queue.run(async () => {
                // run the callback on the connection's own statements, committing or rolling back by its outcome
                await this.database.exec(`BEGIN ${mode}`);
                try {
                    const value = await operation(new TursoQuery(this.database, this.statements));
                    await this.database.exec("COMMIT");

                    return value;
                } catch (error) {
                    await this.database.exec("ROLLBACK");
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
 * The prepared statements of one connection, by text and row mode, each lent to one execution at a time.
 *
 * Turso binds parameters before it takes the connection's lock, so two executions never share one statement.
 */
export class StatementPool {
    /** The database preparing the statements. */
    readonly #database: turso.Database;
    /** The idle statements by row mode and text, least recently used first. */
    readonly #idle = new Map<string, NativeStatement[]>();

    /** Keep statements prepared on one database. */
    constructor(database: turso.Database) {
        this.#database = database;
    }

    /** Lend an idle statement of a text and row mode, or prepare one, and keep it for reuse after the work. */
    async use<Value>(
        sql: string,
        isRaw: boolean,
        isSafe: boolean,
        work: (statement: NativeStatement) => Promise<Value>,
    ): Promise<Value> {
        // take an idle statement, or prepare one in the requested modes
        const key = `${isRaw ? "r" : "o"}${isSafe ? "s" : "n"}${sql}`;
        const idle = this.#idle.get(key);
        const statement =
            idle?.pop() ?? (await this.#database.prepare(sql)).raw(isRaw).safeIntegers(isSafe);

        // return it once the work is done, keeping the most recently used texts
        try {
            return await work(statement);
        } finally {
            const kept = this.#idle.get(key) ?? [];
            this.#idle.delete(key);
            this.#idle.set(key, [...kept, statement]);
            if (this.#idle.size > PREPARED_TEXTS) {
                const [oldest] = this.#idle.keys();
                this.#idle.delete(oldest!);
            }
        }
    }

    /** Forget every statement before the connection closes. */
    close(): void {
        this.#idle.clear();
    }
}
