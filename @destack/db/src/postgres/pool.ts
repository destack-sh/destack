import type { DatabaseEnvironment } from "../database/connection.ts";
import type { Environment, Scheduler } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import postgres from "postgres";
import { quote } from "../dialect/quote.ts";
import { DatabaseError } from "../error/error.ts";

/** The connections of one pool: five, within the six a Worker opens per Hyperdrive. */
const POOL_CONNECTIONS = 5;

/** How long a reservation waits for an idle connection before failing: 30 s, HikariCP's default. */
const RESERVE_WAIT_MILLISECONDS = 30_000;

/** The search path of a new connection: none, so a statement outside a schema fails. */
const NO_SCHEMA = '""';

/** The PostgreSQL type identifiers of json and jsonb. */
const JSON_TYPES = [114, 3802] as const;

/** A connection reserved from a pool, and its search path change sent ahead of the work. */
export interface PostgresReservation {
    /** The reserved connection. */
    readonly sql: postgres.Sql;
    /** The search path change, settled at once when the connection is in the schema already. */
    readonly ready: Promise<unknown>;
}

/** A PostgreSQL pool of single connections, each in the schema it last ran in. */
export class PostgresPool {
    /** The connection listening for every database on the pool. */
    readonly listener: postgres.Sql;
    /** The parties listening by channel name, with the tokens each waits to receive. */
    readonly parties = new Map<string, Set<Map<string, () => void>>>();
    /** How long a reservation waits for an idle connection before failing, in milliseconds. */
    readonly #wait: number;
    /** The pool's connections. */
    readonly #connections: readonly postgres.Sql[];
    /** The connections no work has reserved. */
    readonly #idle: postgres.Sql[];
    /** The reservations waiting for a connection, oldest first. */
    readonly #waiters: ((sql: postgres.Sql) => void)[] = [];
    /** The search path each connection has set, unknown after a change failed. */
    readonly #schemas = new Map<postgres.Sql, string | undefined>();
    /** The world each reserved connection serves, by its scheduler, until released. */
    readonly #holders = new Map<postgres.Sql, Scheduler>();
    /** The callbacks of each listening party, called whenever the listening connection closes. */
    readonly #closings = new Set<() => void>();
    /** The pool's spans and durations. */
    readonly #scope: TelemetryScope;
    /** The pool's name, as OpenTelemetry's `db.client.connection.pool.name` reports it. */
    readonly #pool: string;
    /** The connections idle and in use. */
    readonly #connectionStates;
    /** The requests waiting for a connection. */
    readonly #pending;
    /** The waits for a connection that ran out. */
    readonly #timeouts;

    /** Open a pool of a server, passing JSON as text and reporting the server's warnings. */
    constructor(
        address: string,
        environment: Pick<Environment, "telemetry">,
        options: {
            /** The connections, five by default. */
            readonly connections?: number;
            /** The server settings of each connection, besides its empty search path. */
            readonly settings?: Readonly<Record<string, string>>;
            /** How long a reservation waits for an idle connection, 30 s by default. */
            readonly wait?: number;
        } = {},
    ) {
        // open the connections lazily, each starting without a schema
        const open = (closed?: () => void) => {
            const sql: postgres.Sql = postgres(address, {
                max: 1,
                prepare: false,
                connection: { ...options.settings, search_path: NO_SCHEMA },
                onnotice: reportNotices(environment.telemetry),
                onclose: () => {
                    this.#schemas.set(sql, NO_SCHEMA);
                    closed?.();
                },
            });

            // pass JSON columns' text to the server as it is, since columns encode JSON themselves
            for (const type of JSON_TYPES) {
                sql.options.serializers[type] = (value: unknown) => value;
            }

            return sql;
        };
        const connections = options.connections ?? POOL_CONNECTIONS;
        this.#connections = Array.from({ length: connections }, open);
        this.#pool = poolName(address);
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.#connectionStates = this.#scope.metric.upDownCounter("db.client.connection.count", {
            unit: "{connection}",
            attributes: {
                "db.client.connection.pool.name": [this.#pool],
                "db.client.connection.state": ["idle", "used"],
            },
        });
        this.#connectionStates.add(connections, {
            "db.client.connection.pool.name": this.#pool,
            "db.client.connection.state": "idle",
        });

        // declare the pool's size, its waiting requests and its timed out waits
        const named = { "db.client.connection.pool.name": [this.#pool] };
        this.#scope.metric
            .upDownCounter("db.client.connection.max", { unit: "{connection}", attributes: named })
            .add(connections, { "db.client.connection.pool.name": this.#pool });
        this.#pending = this.#scope.metric.upDownCounter("db.client.connection.pending_requests", {
            unit: "{request}",
            attributes: named,
        });
        this.#timeouts = this.#scope.metric.counter("db.client.connection.timeouts", {
            unit: "{timeout}",
            attributes: named,
        });
        this.#idle = [...this.#connections];
        this.listener = open(() => {
            for (const closing of this.#closings) {
                closing();
            }
        });
        this.#wait = options.wait ?? RESERVE_WAIT_MILLISECONDS;
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

    /** Reserve an idle connection for work in a schema, failing once none frees within the wait. */
    async reserve(
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
        schema: string,
    ): Promise<PostgresReservation> {
        // take a connection for the environment's work
        const sql = await this.#take(environment);

        // set the connection's search path once it changes, forgetting it should the change fail
        if (this.#schemas.get(sql) === schema) {
            return { sql, ready: Promise.resolve() };
        }
        this.#schemas.set(sql, schema);
        const ready = sql
            .unsafe(`SET search_path TO ${quote(schema)}`)
            .simple()
            .execute()
            .catch((error: unknown) => {
                this.#schemas.set(sql, undefined);
                throw error;
            });

        return { sql, ready };
    }

    /** Give back a reserved connection to the oldest waiting reservation, else to the idle ones. */
    release(sql: postgres.Sql): void {
        // refuse a connection the pool did not reserve or took back
        if (!this.#holders.delete(sql)) {
            throw new TypeError("the connection is not reserved from this pool");
        }
        this.#release();

        // hand it on
        this.#handOn(sql);
    }

    /** Run work that reads and writes no table, such as a notification, in any schema. */
    async run<Value>(
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
        work: (sql: postgres.Sql) => Promise<Value>,
    ): Promise<Value> {
        const sql = await this.#take(environment);
        try {
            return await work(sql);
        } finally {
            this.release(sql);
        }
    }

    /** Call back whenever the listening connection closes, until stopped. */
    onListenerClose(closing: () => void): () => void {
        this.#closings.add(closing);

        return () => this.#closings.delete(closing);
    }

    /** Close the pool's connections after their queries. */
    async end(): Promise<void> {
        await Promise.all([...this.#connections, this.listener].map((sql) => sql.end()));
    }

    /** Take an idle connection for an environment's work, else wait for a release. */
    async #take(
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
    ): Promise<postgres.Sql> {
        // take an idle connection or wait for one, counting it used
        const sql = await this.#measureWait(
            async () => this.#idle.pop() ?? (await this.#waitReleased(environment)),
        );
        this.#holders.set(sql, environment.scheduler);
        this.#use();

        return sql;
    }

    /** Wait for a released connection, failing once none comes within the wait. */
    async #waitReleased(
        environment: Pick<DatabaseEnvironment, "clock" | "scheduler">,
    ): Promise<postgres.Sql> {
        // queue the wait, keeping the world's time while other worlds' work has connections
        const { clock, scheduler } = environment;
        const released = Promise.withResolvers<postgres.Sql>();
        const ended = Promise.withResolvers<void>();
        const pool = { "db.client.connection.pool.name": this.#pool };
        this.#waiters.push(released.resolve);
        this.#pending.add(1, pool);
        if ([...this.#holders.values()].some((holder) => holder !== scheduler)) {
            void scheduler.waitUntil(ended.promise);
        }
        try {
            // race the release against the wait on the clock
            const stop = new AbortController();
            const sql = await Promise.race([
                released.promise,
                clock.sleep(this.#wait, stop.signal).then(
                    () => undefined,
                    (error: unknown) => {
                        if (!stop.signal.aborted) {
                            throw error;
                        }

                        return undefined;
                    },
                ),
            ]).finally(() => stop.abort());

            // take a connection released before or as the wait ended, else refuse the wait
            const isQueued = this.#waiters.includes(released.resolve);
            if (sql !== undefined || !isQueued) {
                return sql ?? (await released.promise);
            }
            this.#timeouts.add(1, pool);
            throw DatabaseError.of(
                "CONNECTION_UNAVAILABLE",
                `connection is unavailable: every one stayed in a transaction for ${this.#wait} ms`,
            );
        } catch (error) {
            // leave the queue, handing on a connection a release gave the wait meanwhile
            const index = this.#waiters.indexOf(released.resolve);
            if (index === -1) {
                this.#handOn(await released.promise);
            } else {
                this.#waiters.splice(index, 1);
            }
            throw error;
        } finally {
            this.#pending.add(-1, pool);
            ended.resolve();
        }
    }

    /** Give a free connection to the oldest waiting reservation, else to the idle ones. */
    #handOn(sql: postgres.Sql): void {
        const waiter = this.#waiters.shift();
        if (waiter === undefined) {
            this.#idle.push(sql);
        } else {
            waiter(sql);
        }
    }
}

/** Name a pool by its server's host, port and database, leaving out its credentials. */
function poolName(address: string): string {
    const url = new URL(address);

    return `${url.host}${url.pathname}`;
}

/** Make the reporter of a server's warnings on a telemetry, dropping informational notices. */
export function reportNotices(
    telemetry: Environment["telemetry"],
): (notice: postgres.Notice) => void {
    const { log } = TelemetryScope.of(import.meta.destack.package, telemetry);

    return (notice) => {
        if (notice["severity"] === "WARNING") {
            log.warn("destack.postgres.warned", {
                "db.response.status_code": String(notice["code"]),
                "destack.postgres.message": String(notice["message"]),
            });
        }
    };
}
