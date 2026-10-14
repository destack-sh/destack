import { DatabaseError } from "../error/error.ts";
import type { DriverValue } from "../table/column.ts";

/** How long work waits behind earlier work before failing: 30 s, HikariCP's default. */
const TURN_WAIT_MILLISECONDS = 30_000;

/** The statements of a SQLite connection or one of its transactions. */
export interface SqliteQueryClient {
    /** Read every row as an array of values, integers exact where the engine keeps them. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]>;
    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]>;
    /** Run a statement for its effect. */
    run(sql: string, parameters: readonly DriverValue[]): Promise<void>;
    /** Run a script of statements. */
    exec(script: string): Promise<void>;
    /** Run work in a nested transaction at a depth, undoing only its writes when it fails. */
    nest<Value>(
        depth: number,
        operation: (client: SqliteQueryClient) => Promise<Value>,
    ): Promise<Value>;
}

/** A SQLite connection with dedicated transactions. */
export interface SqliteConnectionClient extends SqliteQueryClient {
    /** Close the physical connection. */
    close(): Promise<void>;
    /** Run a callback with exclusive transaction access. */
    transactionAsync<Value>(operation: (client: SqliteQueryClient) => Promise<Value>): {
        /** Begin a deferred transaction. */
        deferred(): Promise<Value>;
        /** Begin a write transaction. */
        immediate(): Promise<Value>;
        /** Begin an exclusive transaction. */
        exclusive(): Promise<Value>;
    };
}

/** Work run one piece at a time, in arrival order, each waiting for its turn a bounded time. */
export class SqliteWorkQueue {
    /** How long work waits for its turn before failing, in milliseconds. */
    readonly #wait: number;
    /** The tail of the waiting work. */
    #tail: Promise<unknown> = Promise.resolve();
    /** The work queued or running now. */
    #pending = 0;

    /** Queue work, failing work whose turn does not come within the wait. */
    constructor(wait = TURN_WAIT_MILLISECONDS) {
        this.#wait = wait;
    }

    /** Run work after the earlier work, refusing it once its turn takes longer than the wait. */
    run<Value>(work: () => Promise<Value>): Promise<Value> {
        // run work at once on an idle queue, and wait a bounded time behind earlier work
        const turn = this.#tail;
        const result = this.#pending === 0 ? turn.then(work) : this.#waited(turn, work);
        this.#pending += 1;

        // queue later work behind both the earlier work and this one
        const settled = result.then(
            () => undefined,
            () => undefined,
        );
        this.#tail = Promise.all([turn, settled]);
        void settled.then(() => {
            this.#pending -= 1;
        });

        return result;
    }

    /** Run work in its turn after earlier work, or refuse it once the wait ends first. */
    #waited<Value>(turn: Promise<unknown>, work: () => Promise<Value>): Promise<Value> {
        // wait for the earlier work or the end of the wait, whichever comes first
        let timer: ReturnType<typeof setTimeout> | undefined;
        const reached = new Promise<boolean>((resolve) => {
            timer = setTimeout(() => resolve(false), this.#wait);
            void turn.then(() => resolve(true));
        });

        // run the work in its turn, or refuse it
        return reached.then((isTurn) => {
            clearTimeout(timer);
            if (!isTurn) {
                throw new DatabaseError(
                    "CONNECTION_TIMEOUT",
                    `waited ${this.#wait} ms for the connection: a transaction still has it`,
                );
            }

            return work();
        });
    }
}

/** Nested transactions as SQL savepoints, for clients that accept savepoint statements. */
export const SqliteSavepoint = {
    /** Run work in a savepoint at a depth, rolling back to it when the work fails. */
    async run<Value>(
        client: SqliteQueryClient,
        depth: number,
        operation: (client: SqliteQueryClient) => Promise<Value>,
    ): Promise<Value> {
        const name = `"destack_savepoint_${depth}"`;
        await client.run(`SAVEPOINT ${name}`, []);
        try {
            const result = await operation(client);
            await client.run(`RELEASE SAVEPOINT ${name}`, []);

            return result;
        } catch (error) {
            // keep both failures when the savepoint cannot be restored
            try {
                await client.run(`ROLLBACK TO SAVEPOINT ${name}`, []);
                await client.run(`RELEASE SAVEPOINT ${name}`, []);
            } catch (rollback) {
                throw new AggregateError(
                    [error, rollback],
                    "SQLite savepoint and rollback failed",
                    {
                        cause: rollback,
                    },
                );
            }

            throw error;
        }
    },
};
