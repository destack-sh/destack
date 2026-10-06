import { AsyncLocalStorage } from "node:async_hooks";
import type { DriverValue } from "../table/column.ts";
import {
    SqliteWorkQueue,
    type SqliteConnectionClient,
    type SqliteQueryClient,
} from "../sqlite/client.ts";
import { SqliteScript } from "../sqlite/script.ts";

/** The rows one statement of a Durable Object's SQL storage yields. */
export interface DurableObjectCursor {
    /** Read every row as a named object. */
    toArray(): Record<string, unknown>[];
    /** Read every row as an array of values. */
    raw(): Iterable<unknown[]>;
    /** The rows the statement read so far, all of them once its rows are read. */
    readonly rowsRead: number;
    /** The rows the statement wrote. */
    readonly rowsWritten: number;
}

/** The rows an object's SQLite storage read and wrote through its clients since they were last taken, as Cloudflare bills them. */
export interface RowCount {
    /** The rows read. */
    readonly rowsRead: number;
    /** The rows written. */
    readonly rowsWritten: number;
}

/** The parts of a Durable Object's storage the client uses. */
export interface DurableObjectStorage {
    /** The object's SQLite database. */
    readonly sql: {
        /** Execute one statement with bound parameters. */
        exec(query: string, ...bindings: unknown[]): DurableObjectCursor;
        /** The bytes the object's SQLite database takes. */
        readonly databaseSize: number;
    };
    /** Run work in a transaction, nested in the current one when inside it. */
    transaction<Value>(closure: () => Promise<Value>): Promise<Value>;
}

/** The most parameters a Durable Object's SQLite storage binds to one statement, as Cloudflare limits it. */
const MAX_PARAMETERS = 100;

/** The work queue of each object's storage, shared by the databases it keeps since its one SQLite connection runs one transaction at a time. */
const QUEUES = new WeakMap<DurableObjectStorage, SqliteWorkQueue>();

/** The storage transaction the current work runs in, which the work of the storage's other databases joins. */
const TRANSACTIONS = new AsyncLocalStorage<StorageTransaction>();

/** The rows each object's storage read and wrote through its clients since the last take, added to in place. */
const ROWS = new WeakMap<DurableObjectStorage, { rowsRead: number; rowsWritten: number }>();

/** The rows each object's storage read and wrote, counted in memory per object until the meters take them. */
export const RowCount = {
    /** Add the rows a statement read and wrote, once its rows are read, to its storage's count. */
    add(storage: DurableObjectStorage, cursor: DurableObjectCursor): void {
        // start the storage's count once per take
        let counted = ROWS.get(storage);
        if (counted === undefined) {
            counted = { rowsRead: 0, rowsWritten: 0 };
            ROWS.set(storage, counted);
        }

        // add the statement's rows
        counted.rowsRead += cursor.rowsRead;
        counted.rowsWritten += cursor.rowsWritten;
    },

    /** Take the rows a storage read and wrote since the last take, starting its count again from zero. */
    take(storage: DurableObjectStorage): RowCount {
        const counted = ROWS.get(storage) ?? { rowsRead: 0, rowsWritten: 0 };
        ROWS.delete(storage);

        return counted;
    },
};

/** A transaction of an object's storage, open while its work runs. */
interface StorageTransaction {
    /** The storage. */
    readonly storage: DurableObjectStorage;
    /** Whether the transaction's work still runs. */
    isOpen: boolean;
}

/** Statements over a Durable Object's SQLite storage. */
export class DurableObjectQuery implements SqliteQueryClient {
    /** The storage. */
    readonly storage: DurableObjectStorage;
    /** The work queue, absent within a transaction. */
    readonly queue: SqliteWorkQueue | undefined;

    /** Create the client. */
    constructor(storage: DurableObjectStorage, queue?: SqliteWorkQueue) {
        this.storage = storage;
        this.queue = queue;
    }

    /** Read every row as an array of values, integers as the storage returns them. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#schedule(async () =>
            this.#read(this.#exec(sql, parameters), (cursor) => [...cursor.raw()]),
        );
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(async () =>
            this.#read(this.#exec(sql, parameters), (cursor) => cursor.toArray()),
        );
    }

    /** Run a statement for its effect. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#schedule(async () => {
            this.#read(this.#exec(sql, parameters), (cursor) => cursor.toArray());
        });
    }

    /** Run a script one statement at a time. */
    async exec(script: string): Promise<void> {
        await this.#schedule(async () => {
            for (const statement of SqliteScript.statements(script)) {
                this.#read(this.storage.sql.exec(statement), (cursor) => cursor.toArray());
            }
        });
    }

    /** Run work in a transaction nested in the storage's current one. */
    nest<Value>(
        _depth: number,
        operation: (client: SqliteQueryClient) => Promise<Value>,
    ): Promise<Value> {
        return this.storage.transaction(() => operation(this));
    }

    /** Execute a statement, writing its values into its text when it binds more than the storage takes. */
    #exec(sql: string, parameters: readonly DriverValue[]): DurableObjectCursor {
        return parameters.length <= MAX_PARAMETERS
            ? this.storage.sql.exec(sql, ...parameters)
            : this.storage.sql.exec(inlined(sql, parameters));
    }

    /** Read a statement's rows, then count the rows it read and wrote. */
    #read<Rows>(cursor: DurableObjectCursor, read: (cursor: DurableObjectCursor) => Rows): Rows {
        const rows = read(cursor);
        RowCount.add(this.storage, cursor);

        return rows;
    }

    /** Run work behind the queue, or at once within a transaction. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.queue === undefined ? work() : schedule(this.storage, this.queue, work);
    }
}

/** A connection client over a Durable Object's SQLite storage, one transaction at a time across the databases it keeps. */
export class DurableObjectClient extends DurableObjectQuery implements SqliteConnectionClient {
    /** The storage's work queue. */
    readonly #queue: SqliteWorkQueue;

    /** Create the client, queueing behind the other clients of the storage. */
    constructor(storage: DurableObjectStorage) {
        // share the storage's queue with its other clients
        const queue = QUEUES.get(storage) ?? new SqliteWorkQueue();
        QUEUES.set(storage, queue);
        super(storage, queue);
        this.#queue = queue;
    }

    /** Close nothing: the storage belongs to its Durable Object. */
    async close(): Promise<void> {
        await this.#queue.run(async () => undefined);
    }

    /** Run a callback in a storage transaction, whatever its requested locking mode, nested in the open one it runs in. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>) {
        const begin = () =>
            schedule(this.storage, this.#queue, async () => {
                // mark the transaction open while its work runs
                const transaction = { storage: this.storage, isOpen: true };
                try {
                    return await TRANSACTIONS.run(transaction, () =>
                        this.storage.transaction(() =>
                            operation(new DurableObjectQuery(this.storage)),
                        ),
                    );
                } finally {
                    transaction.isOpen = false;
                }
            });

        return { deferred: begin, immediate: begin, exclusive: begin };
    }
}

/** Run work of a storage's database in the storage's open transaction it runs in, behind the storage's queue otherwise. */
function schedule<Value>(
    storage: DurableObjectStorage,
    queue: SqliteWorkQueue,
    work: () => Promise<Value>,
): Promise<Value> {
    const current = TRANSACTIONS.getStore();

    return current?.storage === storage && current.isOpen ? work() : queue.run(work);
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
