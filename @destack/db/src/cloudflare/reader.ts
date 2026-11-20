import type { DatabaseEnvironment } from "../database/connection.ts";
import { schema } from "@destack/schema";
import type { DriverValue } from "../table/column.ts";
import type { SqliteConnectionClient, SqliteQueryClient } from "../sqlite/client.ts";
import { SqliteDatabase } from "../sqlite/database.ts";
import { type Channel, DELIVERED } from "../channel/channel.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import type { Table } from "../table/table.ts";
import { DURABLE_OBJECT_LIMITS } from "./client.ts";
import type { FamilyError } from "@destack/error";
import { DatabaseError } from "../error/error.ts";
import { SQLValue } from "../sql/json.ts";
import {
    type CommitPosition,
    CommitWait,
    DATABASE_PATH,
    type DurableObjectDatabaseOperation,
    type DurableObjectStub,
    STORAGE_NAME,
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

/** How long a reader waits before asking an object that did not answer for its commits again. */
const COMMIT_RETRY_MILLISECONDS = 1000;

/** Reads of a database another Durable Object stores, one request per statement. */
export class DurableObjectReader implements SqliteConnectionClient {
    /** The limits Durable Object storage sets on one statement. */
    readonly limits = DURABLE_OBJECT_LIMITS;
    /** The object that stores the database. */
    readonly #object: DurableObjectStub;

    /** Read through the object that stores the database. */
    constructor(object: DurableObjectStub) {
        this.#object = object;
    }

    /** Connect read-only to another object's database in an environment, in a given namespace. */
    static connect(
        object: DurableObjectStub,
        tables: readonly Table[],
        environment: DatabaseEnvironment,
        namespace?: string,
    ) {
        // hear the database's commits from its object, refusing any other channel
        const name = namespace ?? STORAGE_NAME;
        const reader = new DurableObjectReader(object);
        const openChannel = (topic: string): Channel<unknown> => {
            if (topic !== LOG_TOPIC) {
                throw DatabaseError.of(
                    "CHANNEL_BROKEN",
                    `channel '${topic}' is broken: a reader of another object hears its commits alone`,
                );
            }

            return reader.#commits(name, environment);
        };

        return new SqliteDatabase(
            reader,
            tables,
            "networked",
            "reader",
            namespace === undefined
                ? { environment, name, openChannel }
                : { environment, namespace, name, openChannel },
        );
    }

    /** Hear a database's commits and their tables as its object answers each wait for the next. */
    #commits(name: string, environment: DatabaseEnvironment): Channel<unknown> {
        const stops = new Set<AbortController>();

        return {
            delivery: () => DELIVERED,
            // send nothing, as a reader commits nothing
            notify: () => {},
            listen: ({ receive, resume, pause, fail }) => {
                // ask the object to wait for each next commit until stopped
                const stop = new AbortController();
                stops.add(stop);
                void (async () => {
                    let after: CommitPosition | undefined;
                    while (!stop.signal.aborted) {
                        const answer = await this.#wait(name, after, stop.signal).catch(
                            (error: unknown) => ({ error }),
                        );
                        // end at the stop
                        if (stop.signal.aborted) {
                            break;
                        }
                        // trust no silence while the object is unreachable, asking again after a pause
                        else if ("error" in answer && isRetryable(answer.error)) {
                            pause?.();
                            await environment.clock
                                .sleep(COMMIT_RETRY_MILLISECONDS, stop.signal)
                                .catch(() => {});
                            after = undefined;
                        }
                        // fail the channel on a refusal
                        else if ("error" in answer) {
                            fail?.(answer.error);
                            break;
                        }
                        // recheck once the object answers again
                        else if (after === undefined) {
                            resume?.();
                            after = answer.position;
                        }
                        // pass each commit's tables on
                        else {
                            if (answer.tables.length > 0) {
                                receive({ kind: "commit", tables: answer.tables });
                            }
                            after = answer.position;
                        }
                    }
                })();

                return () => {
                    stop.abort();
                    stops.delete(stop);
                };
            },
            close: async () => {
                for (const stop of stops) {
                    stop.abort();
                }
                stops.clear();
            },
        };
    }

    /** Ask the object to wait for a commit past a position, answering the position then. */
    async #wait(
        name: string,
        after: CommitPosition | undefined,
        signal: AbortSignal,
    ): Promise<CommitWait> {
        // post the wait, refusing an answer of a failure
        const operation: DurableObjectDatabaseOperation = {
            operation: "wait",
            name,
            ...(after === undefined ? {} : { after }),
        };
        const response = await this.#object.fetch(
            new Request(`${OBJECT_ORIGIN}${DATABASE_PATH}`, {
                method: "POST",
                body: JSON.stringify(operation),
                signal,
            }),
        );
        if (!response.ok) {
            throw DatabaseError.of(
                "QUERY_BROKEN",
                `the object responded with ${response.status}: '${await response.text()}'`,
            );
        }

        return CommitWait.parse(await response.json());
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
    run(): Promise<number> {
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

    /** Refuse a transaction, which reads across requests cannot span. */
    transactionAsync<Value>(_operation: (client: SqliteQueryClient) => Promise<Value>) {
        const refuse = () => Promise.reject<Value>(DurableObjectReader.#refusal());

        return { deferred: refuse, immediate: refuse, exclusive: refuse };
    }

    /** Release nothing: the object stores the database. */
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
            throw DatabaseError.of(
                "QUERY_BROKEN",
                `the object responded with ${response.status}: '${await response.text()}'`,
            );
        }

        return await response.json();
    }

    /** Build the refusal of anything but a read. */
    static #refusal(): FamilyError<"QUERY_INVALID"> {
        return DatabaseError.of(
            "QUERY_INVALID",
            "query is invalid: a reader of another object's database can only run reads",
        );
    }
}

/** Whether a stub's error is one Cloudflare marks as worth retrying, such as a reset object. */
function isRetryable(error: unknown): boolean {
    return (
        typeof error === "object" &&
        error !== null &&
        "retryable" in error &&
        error.retryable === true
    );
}
