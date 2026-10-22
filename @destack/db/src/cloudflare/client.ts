import type { DriverValue } from "../table/column.ts";
import { type SqliteConnectionClient, type SqliteQueryClient } from "../sqlite/client.ts";
import { SqliteScript } from "../sqlite/script.ts";
import type { DurableObjectDatabaseHost, StorageTransaction } from "./host.ts";

/** The rows one statement of a Durable Object's SQL storage yields. */
export interface DurableObjectCursor {
    /** Read every row as a named object. */
    readonly toArray: () => Record<string, unknown>[];
    /** Read every row as an array of values. */
    readonly raw: () => Iterable<unknown[]>;
    /** The rows the statement read so far, all of them once its rows are read. */
    readonly rowsRead: number;
    /** The rows the statement wrote. */
    readonly rowsWritten: number;
}

/** The Durable Object storage APIs the client uses. */
export interface DurableObjectStorage {
    /** The object's SQLite database. */
    readonly sql: {
        /** Execute one statement with bound parameters. */
        readonly exec: (query: string, ...bindings: unknown[]) => DurableObjectCursor;
        /** The bytes the object's SQLite database takes. */
        readonly databaseSize: number;
    };
    /** Run work in a transaction, nested in the current one when inside it. */
    readonly transaction: <Value>(closure: () => Promise<Value>) => Promise<Value>;
    /** Delete everything the object stores. */
    readonly deleteAll: () => Promise<void>;
}

/** The most parameters Durable Object SQLite storage binds to a statement, as Cloudflare limits. */
const MAX_PARAMETERS = 100;

/** The largest integer a number represents exactly, 2^53 - 1. */
const MAX_EXACT = BigInt(Number.MAX_SAFE_INTEGER);

/** The rows a statement returned and the rows it wrote. */
export interface StatementRows<Rows extends unknown[] = unknown[]> {
    /** The returned rows. */
    readonly rows: Rows;
    /** The rows the statement wrote. */
    readonly rowsWritten: number;
}

/** How a client's statements run: behind the storage's queue, or in one open transaction. */
export type DurableObjectMode =
    | { readonly kind: "queued" }
    | { readonly kind: "joined"; readonly transaction: StorageTransaction };

/** Statements over a Durable Object's SQLite storage. */
export class DurableObjectQuery implements SqliteQueryClient {
    /** The object's one storage host, which the object's clients share. */
    readonly host: DurableObjectDatabaseHost;
    /** How the statements run. */
    readonly mode: DurableObjectMode;

    /** Create the client over the object's host, behind its queue or in its open transaction. */
    constructor(host: DurableObjectDatabaseHost, mode: DurableObjectMode) {
        this.host = host;
        this.mode = mode;
    }

    /** The object's storage. */
    get storage(): DurableObjectStorage {
        return this.host.storage;
    }

    /** Read every row as an array of values, integers as the storage returns them. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        return this.#schedule(
            async () => this.#read(this.#exec(sql, parameters), (cursor) => [...cursor.raw()]).rows,
        );
    }

    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        return this.#schedule(
            async () => this.#read(this.#exec(sql, parameters), (cursor) => cursor.toArray()).rows,
        );
    }

    /** Read every row in a form, with the rows the statement wrote. */
    read(
        sql: string,
        parameters: readonly DriverValue[],
        rows: "values" | "objects",
    ): Promise<StatementRows> {
        const read: (cursor: DurableObjectCursor) => unknown[] =
            rows === "values" ? (cursor) => [...cursor.raw()] : (cursor) => cursor.toArray();

        return this.#schedule(async () => this.#read(this.#exec(sql, parameters), read));
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

    /** Execute a statement, inlining its values when the storage cannot bind them all. */
    #exec(sql: string, parameters: readonly DriverValue[]): DurableObjectCursor {
        // bind exact integers as numbers, since the storage binds no bigints
        const values = parameters.map((value) =>
            typeof value === "bigint" && value >= -MAX_EXACT && value <= MAX_EXACT
                ? Number(value)
                : value,
        );

        // inline more values than the storage binds, or an integer past a number's exact range
        const isBound =
            values.length <= MAX_PARAMETERS && values.every((value) => typeof value !== "bigint");

        return isBound
            ? this.storage.sql.exec(sql, ...values)
            : this.storage.sql.exec(inlined(sql, values));
    }

    /** Read a statement's rows, then count the rows it read and wrote. */
    #read<Rows extends unknown[]>(
        cursor: DurableObjectCursor,
        read: (cursor: DurableObjectCursor) => Rows,
    ): StatementRows<Rows> {
        const rows = read(cursor);
        this.host.count(cursor);

        return { rows, rowsWritten: cursor.rowsWritten };
    }

    /** Run work behind the queue, or at once in the open transaction of a joined client. */
    #schedule<Value>(work: () => Promise<Value>): Promise<Value> {
        // queue the work of a queued client
        if (this.mode.kind === "queued") {
            return this.host.queue.run(work);
        }
        // run a joined client's work within its open transaction
        else {
            return this.mode.transaction.within(work);
        }
    }
}

/** A connection client over Durable Object SQLite storage, one transaction at a time. */
export class DurableObjectClient extends DurableObjectQuery implements SqliteConnectionClient {
    /** Wait for a queued client's work, leaving the storage to its Durable Object. */
    async close(): Promise<void> {
        // wait for the queued work of a queued client
        if (this.mode.kind === "queued") {
            await this.host.queue.run(async () => undefined);
        }
    }

    /** Run a callback in a storage transaction, ignoring its requested locking mode. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>) {
        const { mode } = this;
        const begin = () =>
            mode.kind === "queued"
                ? this.host.transaction((transaction) =>
                      operation(new DurableObjectQuery(this.host, { kind: "joined", transaction })),
                  )
                : mode.transaction.within(() => this.storage.transaction(() => operation(this)));

        return { deferred: begin, immediate: begin, exclusive: begin };
    }
}

/** Inline a statement's values in place of its numbered markers, skipping quotes and names. */
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

    throw new TypeError("a statement has a numbered marker without a bound value");
}
