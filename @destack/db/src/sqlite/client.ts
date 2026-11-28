import type { DatabaseEnvironment } from "../database/connection.ts";
import type { Environment, Scheduler } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { DatabaseError } from "../error/error.ts";
import type { DriverValue } from "../table/column.ts";
import type { SqlLimits } from "../database/session.ts";

/** SQLite's default limits: 32,766 values since 3.32, SQLITE_MAX_COLUMN 2,000. */
export const SQLITE_LIMITS: SqlLimits = { variables: 32_766, columns: 2_000 };

/** How long work waits behind earlier work before failing: 30 s, HikariCP's default. */
const TURN_WAIT_MILLISECONDS = 30_000;

/** The statements of a SQLite connection or one of its transactions. */
export interface SqliteQueryClient {
    /** The limits the engine sets on one statement. */
    readonly limits: SqlLimits;
    /** Read every row as an array of values, integers exact where the engine stores them. */
    readonly values: (sql: string, parameters: readonly DriverValue[]) => Promise<unknown[][]>;
    /** Read every row by column name. */
    readonly all: (sql: string, parameters: readonly DriverValue[]) => Promise<unknown[]>;
    /** Run a statement for its effect, answering how many rows it changed. */
    readonly run: (sql: string, parameters: readonly DriverValue[]) => Promise<number>;
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

/** The pool name of an in-memory database's queue, as SQLite names such a database. */
export const MEMORY_POOL = ":memory:";

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
    /** The turns queued behind earlier work, oldest first. */
    readonly #queued: QueuedTurn[] = [];
    /** The watch refusing queued turns past their deadlines, absent while none waits. */
    #watch: AbortController | undefined;
    /** The queue's spans and durations, as a pool of one connection. */
    readonly #scope: TelemetryScope;
    /** The pool's name, as OpenTelemetry's `db.client.connection.pool.name` reports it. */
    readonly #pool: string;
    /** The connections idle and in use. */
    readonly #connectionStates;

    /** Queue a named file's work, failing work whose turn does not come within the wait. */
    constructor(telemetry: Environment["telemetry"], name: string, wait = TURN_WAIT_MILLISECONDS) {
        // count the file's one connection idle, then store the wait
        this.#pool = name;
        this.#scope = TelemetryScope.of(import.meta.destack.package, telemetry);
        this.#connectionStates = this.#scope.metric.upDownCounter("db.client.connection.count", {
            unit: "{connection}",
            attributes: {
                "db.client.connection.pool.name": [this.#pool],
                "db.client.connection.state": ["idle", "used"],
            },
        });
        this.#connectionStates.add(1, {
            "db.client.connection.pool.name": this.#pool,
            "db.client.connection.state": "idle",
        });
        this.#wait = wait;
    }

    /** Wait for a connection, recording how long the wait took. */
    #measureWait<Value>(run: () => Promise<Value>): Promise<Value> {
        const attributes = { "db.client.connection.pool.name": this.#pool };

        return this.#scope.measure("db.client.connection.wait_time", attributes, run);
    }

    /** Count an idle connection as used. */
    #use(): void {
        const pool = { "db.client.connection.pool.name": this.#pool };
        this.#connectionStates.add(-1, { ...pool, "db.client.connection.state": "idle" });
        this.#connectionStates.add(1, { ...pool, "db.client.connection.state": "used" });
    }

    /** Count a used connection as idle. */
    #release(): void {
        const pool = { "db.client.connection.pool.name": this.#pool };
        this.#connectionStates.add(-1, { ...pool, "db.client.connection.state": "used" });
        this.#connectionStates.add(1, { ...pool, "db.client.connection.state": "idle" });
    }

    /** Run work after the earlier work, refusing it once its turn outlasts the wait. */
    run<Value>(
        work: () => Promise<Value>,
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
    ): Promise<Value> {
        // take the turn, at once on an idle queue and a bounded time behind earlier work, timing every take
        const turn = this.#tail;
        const start = () => this.#start(work, environment.scheduler);
        const queued = this.#measureWait(() =>
            this.#pending === 0 ? turn : this.#turn(turn, environment),
        ).then(start);
        this.#pending += 1;

        // count it done before its caller continues, so the caller's next work finds the queue idle
        const result = queued.finally(() => {
            this.#pending -= 1;
        });

        // queue later work behind both the earlier work and this one
        const settled = result.then(
            () => undefined,
            () => undefined,
        );
        this.#tail = Promise.all([turn, settled]);

        return result;
    }

    /** Start work in its turn, keeping the time of every other world waiting behind it. */
    #start<Value>(work: () => Promise<Value>, scheduler: Scheduler): Promise<Value> {
        // run the work, remembering its world and counting the connection used until it ends
        this.#use();
        const running = work();
        const settled = running.then(
            () => undefined,
            () => undefined,
        );
        this.#running = { scheduler, settled };
        void settled.then(() => {
            this.#release();
            if (this.#running?.settled === settled) {
                this.#running = undefined;
            }
        });

        // keep the other worlds waiting for the file from moving time past it
        for (const waiting of this.#waiting.keys()) {
            if (waiting !== scheduler) {
                void waiting.waitUntil(settled);
            }
        }

        return running;
    }

    /** Wait for a turn after earlier work, or refuse it once the wait ends first. */
    #turn(
        turn: Promise<unknown>,
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
    ): Promise<void> {
        // keep this world's time while another world's work runs ahead
        const { clock, scheduler } = environment;
        this.#waiting.set(scheduler, (this.#waiting.get(scheduler) ?? 0) + 1);
        const running = this.#running;
        if (running !== undefined && running.scheduler !== scheduler) {
            void scheduler.waitUntil(running.settled);
        }

        // queue the turn with its deadline, refused by the queue's one watch once it passes
        const queued = new Promise<void>((resolve, reject) => {
            // record the turn
            const waiting: QueuedTurn = {
                deadline: clock.now() + this.#wait,
                isDone: false,
                reject,
            };
            this.#queued.push(waiting);

            // take the turn once the earlier work ends, unless refused, stopping the watch once none waits
            void turn.then(() => {
                if (!waiting.isDone) {
                    waiting.isDone = true;
                    resolve();
                }
                this.#settle();
            });

            // watch the deadlines while any turn waits
            this.#watch ??= this.#follow(clock);
        });

        return queued.finally(() => this.#leave(scheduler));
    }

    /** Refuse each queued turn once its deadline passes, sleeping until the oldest one's. */
    #follow(clock: DatabaseEnvironment["clock"]): AbortController {
        const watch = new AbortController();
        void (async () => {
            // refuse the oldest turn past its deadline, else sleep until its deadline
            for (let oldest = this.#oldest(); oldest !== undefined; oldest = this.#oldest()) {
                const left = oldest.deadline - clock.now();
                if (left > 0) {
                    await clock.sleep(left, watch.signal);
                } else {
                    oldest.isDone = true;
                    oldest.reject(
                        DatabaseError.of(
                            "CONNECTION_UNAVAILABLE",
                            `the connection is unavailable: a transaction still had it after ${this.#wait} ms`,
                        ),
                    );
                }
            }
        })()
            .catch((error: unknown) => {
                // end quietly once the queue drains and stops the watch
                if (!watch.signal.aborted) {
                    throw error;
                }
            })
            .finally(() => {
                if (this.#watch === watch) {
                    this.#watch = undefined;
                }
            });

        return watch;
    }

    /** Read the oldest turn still waiting, dropping the ones done. */
    #oldest(): QueuedTurn | undefined {
        while (this.#queued[0]?.isDone === true) {
            this.#queued.shift();
        }

        return this.#queued[0];
    }

    /** Stop the watch once no turn waits. */
    #settle(): void {
        if (this.#oldest() === undefined) {
            this.#watch?.abort();
            this.#watch = undefined;
        }
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

/** A turn queued behind earlier work. */
interface QueuedTurn {
    /** When the turn is refused, on the queue's clock. */
    readonly deadline: number;
    /** Whether the turn came or was refused. */
    isDone: boolean;
    /** Refuse the turn. */
    readonly reject: (error: unknown) => void;
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
