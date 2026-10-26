import type { Clock, Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import postgres from "postgres";
import { DatabaseError } from "../error/error.ts";

/** The connections of one pool: five, within the six a Worker opens per Hyperdrive. */
const POOL_CONNECTIONS = 5;

/** How long a reservation waits for an idle connection before failing: 30 s, HikariCP's default. */
const RESERVE_WAIT_MILLISECONDS = 30_000;

/** The search path of a pool's connections: none, so a statement outside a schema fails. */
const NO_SCHEMA = '""';

/** The PostgreSQL type identifiers of json and jsonb. */
const JSON_TYPES = [114, 3802] as const;

/** A PostgreSQL pool whose databases each run in a schema, reserving connections a bounded time. */
export class PostgresPool {
    /** The postgres.js pool, which listens for every database on it. */
    readonly sql: postgres.Sql;
    /** How long a reservation waits for an idle connection before failing, in milliseconds. */
    readonly #wait: number;
    /** The clock of the work each reserved connection serves, until released. */
    readonly #holders = new Map<postgres.ReservedSql, Clock>();

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
        // leave each connection without a schema, and plan each statement in its schema
        this.sql = postgres(address, {
            max: options.connections ?? POOL_CONNECTIONS,
            prepare: false,
            connection: { ...options.settings, search_path: NO_SCHEMA },
            onnotice: reportNotices(environment.telemetry),
        });
        this.#wait = options.wait ?? RESERVE_WAIT_MILLISECONDS;

        // pass JSON columns' text to the server as it is, since columns encode JSON themselves
        for (const type of JSON_TYPES) {
            this.sql.options.serializers[type] = (value: unknown) => value;
        }
    }

    /** Reserve an idle connection for work on a clock, failing once none frees within the wait. */
    async reserve(clock: Clock): Promise<postgres.ReservedSql> {
        // count the wait as host work while other clocks' work has connections, as it waits on them
        const reserving = this.sql.reserve();
        if ([...this.#holders.values()].some((holder) => holder !== clock)) {
            void clock.trackHost(reserving);
        }

        // race the reservation against the wait on the clock
        const stop = new AbortController();
        const isReserved = await Promise.race([
            reserving.then(() => true),
            clock.sleep(this.#wait, stop.signal).then(
                () => false,
                (error: unknown) => {
                    if (!stop.signal.aborted) {
                        throw error;
                    }

                    return true;
                },
            ),
        ]).finally(() => stop.abort());

        // refuse a wait no connection ended, giving back a connection that comes after it
        if (!isReserved) {
            void reserving.then(
                (late) => late.release(),
                () => undefined,
            );
            throw DatabaseError.of(
                "CONNECTION_TIMEOUT",
                `waited ${this.#wait} ms for a connection: every one is in a transaction`,
            );
        }

        // record the clock the connection serves until released
        const reserved = await reserving;
        this.#holders.set(reserved, clock);

        return reserved;
    }

    /** Give back a reserved connection, refusing one the pool did not reserve or took back. */
    release(reserved: postgres.ReservedSql): void {
        if (!this.#holders.delete(reserved)) {
            throw new TypeError("the connection is not reserved from this pool");
        }
        reserved.release();
    }

    /** Run work on a connection reserved for it, such as one statement outside every schema. */
    async run<Value>(clock: Clock, work: (sql: postgres.Sql) => Promise<Value>): Promise<Value> {
        const reserved = await this.reserve(clock);
        try {
            return await work(reserved);
        } finally {
            this.release(reserved);
        }
    }

    /** Close the pool's connections after their queries. */
    end(): Promise<void> {
        return this.sql.end();
    }
}

/** Make the reporter of a server's warnings on a telemetry, dropping informational notices. */
export function reportNotices(
    telemetry: Environment["telemetry"],
): (notice: postgres.Notice) => void {
    const { log } = TelemetryScope.of(import.meta.destack.package, telemetry);

    return (notice) => {
        if (notice["severity"] === "WARNING") {
            log.warn("postgresql.warning", { message: String(notice["message"]) });
        }
    };
}
