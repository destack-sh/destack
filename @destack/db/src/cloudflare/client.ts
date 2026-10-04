import type { DriverValue } from "../table/column.ts";
import { WorkQueue, type ConnectionClient, type QueryClient } from "../sqlite/client.ts";
import { SqliteScript } from "../sqlite/script.ts";

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
export class DurableObjectQuery implements QueryClient {
    /** The storage. */
    readonly storage: DurableObjectStorage;
    /** The work queue, absent within a transaction. */
    readonly queue: WorkQueue | undefined;

    /** Create the client. */
    constructor(storage: DurableObjectStorage, queue?: WorkQueue) {
        this.storage = storage;
        this.queue = queue;
    }

    /** Read every row as an array of values, integers as the storage returns them. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#schedule(async () => [...this.storage.sql.exec(sql, ...parameters).raw()]);
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(async () => this.storage.sql.exec(sql, ...parameters).toArray());
    }

    /** Run a statement for its effect. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#schedule(async () => {
            this.storage.sql.exec(sql, ...parameters).toArray();
        });
    }

    /** Run a script one statement at a time. */
    async exec(script: string): Promise<void> {
        await this.#schedule(async () => {
            for (const statement of SqliteScript.statements(script)) {
                this.storage.sql.exec(statement).toArray();
            }
        });
    }

    /** Run work in a transaction nested in the storage's current one. */
    nest<Value>(
        _depth: number,
        operation: (client: QueryClient) => Promise<Value>,
    ): Promise<Value> {
        return this.storage.transaction(() => operation(this));
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over a Durable Object's SQLite storage, one transaction at a time. */
export class DurableObjectClient extends DurableObjectQuery implements ConnectionClient {
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
    transactionAsync<Value>(operation: (client: QueryClient) => Promise<Value>) {
        const begin = () =>
            this.#queue.run(() =>
                this.storage.transaction(() => operation(new DurableObjectQuery(this.storage))),
            );

        return { deferred: begin, immediate: begin, exclusive: begin };
    }
}
