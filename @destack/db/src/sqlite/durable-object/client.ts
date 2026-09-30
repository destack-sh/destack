import { WorkQueue, type ConnectionClient, type QueryClient, type Statement } from "../client.ts";
import { SqliteScript } from "../script.ts";

/** The result of running a statement. */
export interface RunResult {
    /** The number of rows changed. */
    readonly changes: number;
    /** The last inserted row identifier. */
    readonly lastInsertRowid: number;
}

/** The rows one statement of a Durable Object's SQL storage yields. */
export interface DurableObjectCursor {
    /** Read every row as a named object. */
    toArray(): Record<string, unknown>[];
    /** Read every row as an array of values. */
    raw(): Iterable<unknown[]>;
    /** The rows the statement wrote. */
    readonly rowsWritten: number;
}

/** The parts of a Durable Object's storage the client uses. */
export interface DurableObjectStorage {
    /** The object's SQLite database. */
    readonly sql: {
        /** Execute one statement with bound parameters. */
        exec(query: string, ...bindings: unknown[]): DurableObjectCursor;
    };
    /** Run work in a transaction, nested in the current one when inside it. */
    transaction<Value>(closure: () => Promise<Value>): Promise<Value>;
}

/** Statements over a Durable Object's SQLite storage. */
export class DurableObjectQuery implements QueryClient<RunResult> {
    /** The storage. */
    readonly storage: DurableObjectStorage;
    /** The work queue, absent within a transaction. */
    readonly queue: WorkQueue | undefined;

    /** Create the client. */
    constructor(storage: DurableObjectStorage, queue?: WorkQueue) {
        this.storage = storage;
        this.queue = queue;
    }

    /** Prepare a statement with its row mode. */
    async prepare(sql: string): Promise<Statement<RunResult>> {
        // track the row mode
        let isRaw = false;
        const run = (method: "run" | "all" | "get", parameters: readonly unknown[]) =>
            this.#schedule(async () => this.#execute(sql, method, parameters, isRaw));

        // set the row mode on each run, integers staying numbers as the storage returns them
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

    /** Run a script one statement at a time. */
    exec(script: string): Promise<unknown> {
        return this.#schedule(async () => {
            for (const statement of SqliteScript.statements(script)) {
                this.storage.sql.exec(statement).toArray();
            }
        });
    }

    /** Run work in a transaction nested in the storage's current one. */
    nest<Value>(
        _depth: number,
        operation: (client: QueryClient<RunResult>) => Promise<Value>,
    ): Promise<Value> {
        return this.storage.transaction(() => operation(this));
    }

    /** Execute one statement. */
    #execute(
        sql: string,
        method: "run" | "all" | "get",
        parameters: readonly unknown[],
        isRaw: boolean,
    ): unknown {
        // run a changing statement, reading its row identifier after it
        const cursor = this.storage.sql.exec(sql, ...parameters);
        if (method === "run") {
            cursor.toArray();
            const [inserted] = this.storage.sql.exec("SELECT last_insert_rowid() AS id").toArray();

            return { changes: cursor.rowsWritten, lastInsertRowid: inserted!.id as number };
        }

        // read rows as arrays or objects
        const rows = isRaw ? [...cursor.raw()] : cursor.toArray();

        return method === "all" ? rows : rows[0];
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over a Durable Object's SQLite storage, one transaction at a time. */
export class DurableObjectClient extends DurableObjectQuery implements ConnectionClient<RunResult> {
    /** The work queue. */
    readonly #queue: WorkQueue;

    /** Create the client. */
    constructor(storage: DurableObjectStorage) {
        const queue = new WorkQueue();
        super(storage, queue);
        this.#queue = queue;
    }

    /** Close nothing: the storage belongs to its Durable Object. */
    async close(): Promise<void> {
        await this.#queue.run(async () => undefined);
    }

    /** Run a callback in a storage transaction, whatever its requested locking mode. */
    transactionAsync<Value>(operation: (client: QueryClient<RunResult>) => Promise<Value>) {
        const begin = () =>
            this.#queue.run(() =>
                this.storage.transaction(() => operation(new DurableObjectQuery(this.storage))),
            );

        return { deferred: begin, immediate: begin, exclusive: begin };
    }
}
