import { SimulatedClock } from "@destack/environment/simulation";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TestDatabase } from "../test/database.ts";
import { PostgresPool } from "./pool.ts";

/** A start time for the simulated clocks. */
const START = Date.UTC(2026, 0, 1);

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
    const held = await pool.reserve(clock);
    const waiting = pool.reserve(clock);
    clock.advance(50);
    const refused = await refusal(waiting);

    // reserve again once the connection is released
    pool.release(held);
    const again = await pool.reserve(clock);
    pool.release(again);
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
    const held = await pool.reserve(holder);
    const waiting = pool.reserve(waiter);
    const whileWaiting = waiter.hostInFlight;

    // release it to the waiting clock, which then has no host work in flight
    pool.release(held);
    const reserved = await waiting;
    const afterwards = waiter.hostInFlight;
    pool.release(reserved);
    expect([whileWaiting, afterwards]).toEqual([1, 0]);
});

test("refuse releasing a connection the pool did not reserve or already took back", async () => {
    // release a reserved connection twice
    const pool = openPool();
    const clock = new SimulatedClock(START);
    const reserved = await pool.reserve(clock);
    pool.release(reserved);
    expect(() => pool.release(reserved)).toThrow(
        new TypeError("the connection is not reserved from this pool"),
    );
});
