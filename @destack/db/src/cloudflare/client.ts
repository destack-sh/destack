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

/** The most parameters a Durable Object's SQLite storage binds to one statement, as Cloudflare limits it. */
const MAX_PARAMETERS = 100;

/** The work queue of each object's storage, shared by the databases it keeps since its one SQLite connection runs one transaction at a time. */
const QUEUES = new WeakMap<DurableObjectStorage, WorkQueue>();

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
        return this.#schedule(async () => [...this.#exec(sql, parameters).raw()]);
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(async () => this.#exec(sql, parameters).toArray());
    }

    /** Run a statement for its effect. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#schedule(async () => {
            this.#exec(sql, parameters).toArray();
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

    /** Execute a statement, writing its values into its text when it binds more than the storage takes. */
    #exec(sql: string, parameters: readonly DriverValue[]): DurableObjectCursor {
        return parameters.length <= MAX_PARAMETERS
            ? this.storage.sql.exec(sql, ...parameters)
            : this.storage.sql.exec(inlined(sql, parameters));
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : this.queue.run(work);
    }
}

/** A connection client over a Durable Object's SQLite storage, one transaction at a time across the databases it keeps. */
export class DurableObjectClient extends DurableObjectQuery implements ConnectionClient {
    /** The storage's work queue. */
    readonly #queue: WorkQueue;

    /** Create the client, queueing behind the other clients of the storage. */
    constructor(storage: DurableObjectStorage) {
        // share the storage's queue with its other clients
        const queue = QUEUES.get(storage) ?? new WorkQueue();
        QUEUES.set(storage, queue);
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

/** Write a statement's values into its text in place of its numbered parameter markers, skipping quoted text and names. */
function inlined(sql: string, parameters: readonly DriverValue[]): string {
    // copy the text, replacing each numbered marker outside quotes with its value
    let text = "";
    let quote: string | undefined;
    for (let index = 0; index < sql.length; index += 1) {
        const character = sql.charAt(index);
        const number = /^\?(\d+)/u.exec(sql.slice(index, index + 8))?.[1];

        // track the quoted text and names the markers stay out of
        if (quote !== undefined) {
            quote = character === quote ? undefined : quote;
            text += character;
        } else if (character === "'" || character === '"') {
            quote = character;
            text += character;
        }
        // write the value a marker numbers
        else if (number !== undefined) {
            text += literalOf(parameters[Number(number) - 1]);
            index += number.length;
        } else {
            text += character;
        }
    }

    return text;
}

/** Write a driver value as a SQLite literal. */
function literalOf(value: DriverValue | undefined): string {
    // write scalars
    if (value === null) {
        return "NULL";
    } else if (typeof value === "string") {
        return `'${value.replaceAll("'", "''")}'`;
    } else if (typeof value === "number" || typeof value === "bigint") {
        return value.toString();
    } else if (typeof value === "boolean") {
        return value ? "1" : "0";
    }
    // write bytes as a blob
    else if (value instanceof Uint8Array) {
        return `X'${value.toHex()}'`;
    }

    throw new TypeError("a statement numbers a marker it binds no value for");
}
