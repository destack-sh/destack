import type { DatabaseEnvironment } from "../database/connection.ts";
import type { Scheduler } from "@destack/environment";
import { DatabaseError } from "../error/error.ts";
import type { DriverValue } from "../table/column.ts";

/** How long work waits behind earlier work before failing: 30 s, HikariCP's default. */
const TURN_WAIT_MILLISECONDS = 30_000;

/** The statements of a SQLite connection or one of its transactions. */
export interface SqliteQueryClient {
    /** Read every row as an array of values, integers exact where the engine stores them. */
    readonly values: (sql: string, parameters: readonly DriverValue[]) => Promise<unknown[][]>;
    /** Read every row by column name. */
    readonly all: (sql: string, parameters: readonly DriverValue[]) => Promise<unknown[]>;
    /** Run a statement for its effect. */
    readonly run: (sql: string, parameters: readonly DriverValue[]) => Promise<void>;
    /** Run a script of statements. */
    readonly exec: (script: string) => Promise<void>;
    /** Run work in a nested transaction at a depth, undoing only its writes when it fails. */
    readonly nest: <Value>(
        depth: number,
        operation: (client: SqliteQueryClient) => Promise<Value>,
    ) => Promise<Value>;
}

/** A SQLite connection with dedicated transactions. */
export interface SqliteConnectionClient extends SqliteQueryClient {
    /** Close the physical connection. */
    readonly close: () => Promise<void>;
    /** Run a callback with exclusive transaction access. */
    readonly transactionAsync: <Value>(
        operation: (client: SqliteQueryClient) => Promise<Value>,
    ) => {
        /** Begin a deferred transaction. */
        readonly deferred: () => Promise<Value>;
        /** Begin a write transaction. */
        readonly immediate: () => Promise<Value>;
        /** Begin an exclusive transaction. */
        readonly exclusive: () => Promise<Value>;
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
    /** The world of the work running now, by its scheduler, and its end, absent while none runs. */
    #running: { readonly scheduler: Scheduler; readonly settled: Promise<unknown> } | undefined;
    /** The worlds waiting for a turn, by their schedulers, with how much of each waits. */
    readonly #waiting = new Map<Scheduler, number>();

    /** Queue work, failing work whose turn does not come within the wait. */
    constructor(wait = TURN_WAIT_MILLISECONDS) {
        this.#wait = wait;
    }

    /** Run work after the earlier work, refusing it once its turn outlasts the wait. */
    run<Value>(
        work: () => Promise<Value>,
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
    ): Promise<Value> {
        // run work at once on an idle queue, and wait a bounded time behind earlier work
        const turn = this.#tail;
        const start = () => this.#start(work, environment.scheduler);
        const result =
            this.#pending === 0 ? turn.then(start) : this.#waited(turn, start, environment);
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

    /** Start work in its turn, counting it as peer work in every other world waiting behind it. */
    #start<Value>(work: () => Promise<Value>, scheduler: Scheduler): Promise<Value> {
        // run the work, remembering its world until it ends
        const running = work();
        const settled = running.then(
            () => undefined,
            () => undefined,
        );
        this.#running = { scheduler, settled };
        void settled.then(() => {
            if (this.#running?.settled === settled) {
                this.#running = undefined;
            }
        });

        // keep the other worlds waiting for the file from moving time past it
        for (const waiting of this.#waiting.keys()) {
            if (waiting !== scheduler) {
                void waiting.track(settled, "peer");
            }
        }

        return running;
    }

    /** Run work in its turn after earlier work, or refuse it once the wait ends first. */
    #waited<Value>(
        turn: Promise<unknown>,
        start: () => Promise<Value>,
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
    ): Promise<Value> {
        // count the wait as peer work while another world's work runs ahead
        const { clock, scheduler } = environment;
        this.#waiting.set(scheduler, (this.#waiting.get(scheduler) ?? 0) + 1);
        const running = this.#running;
        if (running !== undefined && running.scheduler !== scheduler) {
            void scheduler.track(running.settled, "peer");
        }

        // race the turn against the wait on the clock, which the turn stops
        const stop = new AbortController();
        const settled = Promise.race([
            turn.then(() => true),
            clock.sleep(this.#wait, stop.signal).then(
                () => false,
                (error: unknown) => {
                    if (!stop.signal.aborted) {
                        throw error;
                    }

                    return true;
                },
            ),
        ]).finally(() => {
            stop.abort();
            this.#leave(scheduler);
        });

        // run the work in its turn, or refuse it
        return settled.then((isTurn) => {
            if (!isTurn) {
                throw DatabaseError.of(
                    "CONNECTION_TIMEOUT",
                    `waited ${this.#wait} ms for the connection: a transaction still has it`,
                );
            }

            return start();
        });
    }

    /** Stop counting one piece of a world's waiting work. */
    #leave(scheduler: Scheduler): void {
        const count = (this.#waiting.get(scheduler) ?? 0) - 1;
        if (count === 0) {
            this.#waiting.delete(scheduler);
        } else {
            this.#waiting.set(scheduler, count);
        }
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
            // report both failures when the savepoint cannot be restored
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
