import { SimulatedClock } from "@destack/environment/simulation";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TestDatabase } from "../test/database.ts";
import { PostgresPool } from "./pool.ts";

/** A start time for the simulated clocks. */
const START = Date.UTC(2026, 0, 1);

/** The schema the reservations run in. */
const SCHEMA = "pool_test";

/** Open a pool of one connection whose reservations wait 50 ms, closed after the test. */
function openPool(): PostgresPool {
    const pool = new PostgresPool(TestDatabase.address(), testHost(), { connections: 1, wait: 50 });
    onTestFinished(() => pool.end());

    return pool;
}

test("refuse a reservation while the clock's own work keeps every connection past the wait, then reserve the released one", async () => {
    // reserve the one connection, then wait past the wait for another on the same clock
    const pool = openPool();
    const clock = new SimulatedClock(START);
    const held = await pool.reserve(clock, SCHEMA);
    const waiting = pool.reserve(clock, SCHEMA);
    clock.advance(50);
    const refused = await refusal(waiting);

    // reserve again once the connection is released
    pool.release(held.sql);
    const again = await pool.reserve(clock, SCHEMA);
    pool.release(again.sql);
    expect(refused).toEqual([
        "SERVICE_UNAVAILABLE",
        "CONNECTION_TIMEOUT",
        "waited 50 ms for a connection: every one is in a transaction",
    ]);
});

test("count a wait behind another clock's connection as host work, reserving it once released", async () => {
    // reserve the one connection on one clock, then wait for it on another
    const pool = openPool();
    const holder = new SimulatedClock(START);
    const waiter = new SimulatedClock(START);
    const held = await pool.reserve(holder, SCHEMA);
    const waiting = pool.reserve(waiter, SCHEMA);
    const whileWaiting = waiter.hostInFlight;

    // release it to the waiting clock, which then has no host work in flight
    pool.release(held.sql);
    const reserved = await waiting;
    const afterwards = waiter.hostInFlight;
    pool.release(reserved.sql);
    expect([whileWaiting, afterwards]).toEqual([1, 0]);
});

test("end a timed-out wait behind another clock's connection as host work, leaving the queue", async () => {
    // wait past the wait for the one connection another clock holds
    const pool = openPool();
    const holder = new SimulatedClock(START);
    const waiter = new SimulatedClock(START);
    const held = await pool.reserve(holder, SCHEMA);
    const waiting = pool.reserve(waiter, SCHEMA);
    waiter.advance(50);
    const refused = await refusal(waiting);

    // leave no host work in flight, and give the released connection to the next reservation
    const afterwards = waiter.hostInFlight;
    pool.release(held.sql);
    const next = await pool.reserve(holder, SCHEMA);
    pool.release(next.sql);
    expect({ refused, afterwards }).toEqual({
        refused: [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_TIMEOUT",
            "waited 50 ms for a connection: every one is in a transaction",
        ],
        afterwards: 0,
    });
});

test("refuse releasing a connection the pool did not reserve or already took back", async () => {
    // release a reserved connection twice
    const pool = openPool();
    const clock = new SimulatedClock(START);
    const reserved = await pool.reserve(clock, SCHEMA);
    pool.release(reserved.sql);
    expect(() => pool.release(reserved.sql)).toThrow(
        new TypeError("the connection is not reserved from this pool"),
    );
});

test("set a connection's search path to the schema it is reserved in, and again once the schema changes", async () => {
    // reserve the one connection in two schemas in turn, reading its search path each time
    const pool = openPool();
    const clock = new SimulatedClock(START);
    const searchPath = async (schema: string) => {
        const { sql, ready } = await pool.reserve(clock, schema);
        try {
            await ready;
            const [row] = await sql`SHOW search_path`;

            return String(row?.["search_path"]);
        } finally {
            pool.release(sql);
        }
    };
    expect([
        await searchPath(SCHEMA),
        await searchPath(SCHEMA),
        await searchPath("pool_other"),
    ]).toEqual([SCHEMA, SCHEMA, "pool_other"]);
});
