import { SimulatedClock } from "@destack/environment/simulation";
import { expect, onTestFinished, refusal, testOn } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TestDatabase } from "../test/database.ts";
import { environmentOn } from "../test/environment.ts";
import { PostgresPool } from "./pool.ts";

/** Test against PostgreSQL alone, as the file tests its own connector or host. */
const onPostgresql = testOn("postgresql");

/** A start time for the simulated clocks. */
const START = Date.UTC(2026, 0, 1);

/** Open a pool of one connection whose reservations wait 50 ms, closed after the test. */
function openPool(): PostgresPool {
    const pool = new PostgresPool(TestDatabase.address(), testHost(), { connections: 1, wait: 50 });
    onTestFinished(() => pool.end());

    return pool;
}

onPostgresql(
    "refuse a reservation while the world's own work keeps every connection past the wait, then reserve the released one",
    async () => {
        // reserve the one connection, then wait past the wait for another in the same world
        const pool = openPool();
        const environment = environmentOn(new SimulatedClock(START));
        const held = await pool.reserve(environment);
        const waiting = pool.reserve(environment);
        environment.clock.advance(50);
        const refused = await refusal(waiting);

        // reserve again once the connection is released
        pool.release(held);
        const again = await pool.reserve(environment);
        pool.release(again);
        expect(refused).toEqual([
            "SERVICE_UNAVAILABLE",
            "CONNECTION_UNAVAILABLE",
            "connection is unavailable: every one stayed in a transaction for 50 ms",
        ]);
    },
);

onPostgresql(
    "keep a world's time while it waits behind another world's connection, reserving it once released",
    async () => {
        // reserve the one connection in one world, then wait for it in another
        const pool = openPool();
        const holder = environmentOn(new SimulatedClock(START));
        const waiter = environmentOn(new SimulatedClock(START));
        const held = await pool.reserve(holder);
        const waiting = pool.reserve(waiter);
        const whileWaiting = kinds(waiter.scheduler.pending());

        // release it to the waiting world, which then has no work in flight
        pool.release(held);
        const reserved = await waiting;
        const afterwards = kinds(waiter.scheduler.pending());
        pool.release(reserved);
        expect([whileWaiting, afterwards]).toEqual([["operation"], []]);
    },
);

onPostgresql(
    "end a timed-out wait behind another world's connection as peer work, leaving the queue",
    async () => {
        // wait past the wait for the one connection another world holds
        const pool = openPool();
        const holder = environmentOn(new SimulatedClock(START));
        const waiter = environmentOn(new SimulatedClock(START));
        const held = await pool.reserve(holder);
        const waiting = pool.reserve(waiter);
        waiter.clock.advance(50);
        const refused = await refusal(waiting);

        // leave no work in flight, and give the released connection to the next reservation
        const afterwards = kinds(waiter.scheduler.pending());
        pool.release(held);
        const next = await pool.reserve(holder);
        pool.release(next);
        expect({ refused, afterwards }).toEqual({
            refused: [
                "SERVICE_UNAVAILABLE",
                "CONNECTION_UNAVAILABLE",
                "connection is unavailable: every one stayed in a transaction for 50 ms",
            ],
            afterwards: [],
        });
    },
);

onPostgresql(
    "refuse releasing a connection the pool did not reserve or already took back",
    async () => {
        // release a reserved connection twice
        const pool = openPool();
        const environment = environmentOn(new SimulatedClock(START));
        const reserved = await pool.reserve(environment);
        pool.release(reserved);
        expect(() => pool.release(reserved)).toThrow(
            new TypeError("the connection is not reserved from this pool"),
        );
    },
);

onPostgresql(
    "keep every connection's search path empty, so a statement naming no schema fails",
    async () => {
        // reserve the one connection, reading its search path and an unqualified table
        const pool = openPool();
        const environment = environmentOn(new SimulatedClock(START));
        const sql = await pool.reserve(environment);
        try {
            const [row] = await sql`SHOW search_path`;
            const unqualified = await sql`SELECT 1 FROM pg_tables_unqualified`.then(
                () => "found",
                (error: unknown) => (error instanceof Error ? error.message : String(error)),
            );
            expect([String(row?.["search_path"]), unqualified]).toEqual([
                '""',
                'relation "pg_tables_unqualified" does not exist',
            ]);
        } finally {
            pool.release(sql);
        }
    },
);

/** Read who finishes each work in flight. */
function kinds(pending: readonly string[]): string[] {
    return pending.map((line) => line.split(" ")[0] ?? "");
}
