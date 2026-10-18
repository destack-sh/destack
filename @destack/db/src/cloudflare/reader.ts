import { schema } from "@destack/schema";
import type { Table } from "../table/table.ts";
import type { DriverValue } from "../table/column.ts";
import type { SqliteConnectionClient, SqliteQueryClient } from "../sqlite/client.ts";
import { SqliteDatabase } from "../sqlite/database.ts";
import { DatabaseError } from "../error/error.ts";
import { SQLValue } from "../sql/json.ts";
import {
    DATABASE_PATH,
    type DurableObjectDatabaseOperation,
    type DurableObjectStub,
} from "./host.ts";

/** The origin of the requests to another object, which its stub routes by path alone. */
const OBJECT_ORIGIN = "https://object.invalid";

/** The rows a read answers as arrays of values. */
const ValueRows = schema.object({
    /** The rows. */
    rows: schema.array(schema.array(SQLValue)),
});

/** The rows a read answers as objects by column name. */
const ObjectRows = schema.object({
    /** The rows. */
    rows: schema.array(schema.record(schema.string(), SQLValue)),
});

/** Reads of a database another Durable Object keeps, one request per statement. */
export class DurableObjectReader implements SqliteConnectionClient {
    /** The object keeping the database. */
    readonly #object: DurableObjectStub;

    /** Read through the object keeping the database. */
    constructor(object: DurableObjectStub) {
        this.#object = object;
    }

    /** Connect read-only to the database another object keeps, in a namespace of it when given. */
    static connect(object: DurableObjectStub, tables: readonly Table[], namespace?: string) {
        return new SqliteDatabase(
            new DurableObjectReader(object),
            tables,
            "networked",
            undefined,
            namespace,
        );
    }

    /** Read every row as an array of values. */
    async values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        const { rows } = ValueRows.parse(await this.#read(sql, parameters, "values"));

        return rows.map((row) => row.map((value) => SQLValue.toDriver(value)));
    }

    /** Read every row by column name. */
    async all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        const { rows } = ObjectRows.parse(await this.#read(sql, parameters, "objects"));

        return rows.map((row) =>
            Object.fromEntries(
                Object.entries(row).map(([column, value]) => [column, SQLValue.toDriver(value)]),
            ),
        );
    }

    /** Refuse a statement run for its effect. */
    run(): Promise<void> {
        return Promise.reject(DurableObjectReader.#refusal());
    }

    /** Refuse a script. */
    exec(): Promise<void> {
        return Promise.reject(DurableObjectReader.#refusal());
    }

    /** Refuse a nested transaction. */
    nest<Value>(): Promise<Value> {
        return Promise.reject(DurableObjectReader.#refusal());
    }

    /** Refuse a transaction, which reads across requests cannot keep. */
    transactionAsync<Value>(_operation: (client: SqliteQueryClient) => Promise<Value>) {
        const refuse = () => Promise.reject<Value>(DurableObjectReader.#refusal());

        return { deferred: refuse, immediate: refuse, exclusive: refuse };
    }

    /** Release nothing: the object keeps the database. */
    async close(): Promise<void> {}

    /** Send a read to the object, returning its answer's body and refusing one it fails. */
    async #read(
        sql: string,
        parameters: readonly DriverValue[],
        rows: "values" | "objects",
    ): Promise<unknown> {
        // post the statement with its parameters in JSON form
        const operation: DurableObjectDatabaseOperation = {
            operation: "read",
            sql,
            parameters: parameters.map((value) => SQLValue.fromDriver(value)),
            rows,
        };
        const response = await this.#object.fetch(
            new Request(`${OBJECT_ORIGIN}${DATABASE_PATH}`, {
                method: "POST",
                body: JSON.stringify(operation),
            }),
        );
        if (!response.ok) {
            throw new DatabaseError(
                "QUERY_FAILED",
                `the object answered ${response.status}: ${await response.text()}`,
            );
        }

        return await response.json();
    }

    /** Build the refusal of anything but a read. */
    static #refusal(): DatabaseError {
        return new DatabaseError(
            "INVALID_QUERY",
            "a reader of another object's database only runs reads",
        );
    }
}
