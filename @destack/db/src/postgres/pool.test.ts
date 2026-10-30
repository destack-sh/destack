import { SimulatedClock } from "@destack/environment/simulation";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TestDatabase } from "../test/database.ts";
import { environmentOn } from "../test/environment.ts";
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

test("refuse a reservation while the world's own work keeps every connection past the wait, then reserve the released one", async () => {
    // reserve the one connection, then wait past the wait for another in the same world
    const pool = openPool();
    const environment = environmentOn(new SimulatedClock(START));
    const held = await pool.reserve(environment, SCHEMA);
    const waiting = pool.reserve(environment, SCHEMA);
    environment.clock.advance(50);
    const refused = await refusal(waiting);

    // reserve again once the connection is released
    pool.release(held.sql);
    const again = await pool.reserve(environment, SCHEMA);
    pool.release(again.sql);
    expect(refused).toEqual([
        "SERVICE_UNAVAILABLE",
        "CONNECTION_TIMEOUT",
        "waited 50 ms for a connection: every one is in a transaction",
    ]);
});

test("count a wait behind another world's connection as peer work, reserving it once released", async () => {
    // reserve the one connection in one world, then wait for it in another
    const pool = openPool();
    const holder = environmentOn(new SimulatedClock(START));
    const waiter = environmentOn(new SimulatedClock(START));
    const held = await pool.reserve(holder, SCHEMA);
    const waiting = pool.reserve(waiter, SCHEMA);
    const whileWaiting = kinds(waiter.scheduler.pending());

    // release it to the waiting world, which then has no work in flight
    pool.release(held.sql);
    const reserved = await waiting;
    const afterwards = kinds(waiter.scheduler.pending());
    pool.release(reserved.sql);
    expect([whileWaiting, afterwards]).toEqual([["peer"], []]);
});

test("end a timed-out wait behind another world's connection as peer work, leaving the queue", async () => {
    // wait past the wait for the one connection another world holds
    const pool = openPool();
    const holder = environmentOn(new SimulatedClock(START));
    const waiter = environmentOn(new SimulatedClock(START));
    const held = await pool.reserve(holder, SCHEMA);
    const waiting = pool.reserve(waiter, SCHEMA);
    waiter.clock.advance(50);
    const refused = await refusal(waiting);

    // leave no work in flight, and give the released connection to the next reservation
    const afterwards = kinds(waiter.scheduler.pending());
    pool.release(held.sql);
    const next = await pool.reserve(holder, SCHEMA);
    pool.release(next.sql);
    expect({ refused, afterwards }).toEqual({
        refused: [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_TIMEOUT",
            "waited 50 ms for a connection: every one is in a transaction",
        ],
        afterwards: [],
    });
});

test("refuse releasing a connection the pool did not reserve or already took back", async () => {
    // release a reserved connection twice
    const pool = openPool();
    const environment = environmentOn(new SimulatedClock(START));
    const reserved = await pool.reserve(environment, SCHEMA);
    pool.release(reserved.sql);
    expect(() => pool.release(reserved.sql)).toThrow(
        new TypeError("the connection is not reserved from this pool"),
    );
});

test("set a connection's search path to the schema it is reserved in, and again once the schema changes", async () => {
    // reserve the one connection in two schemas in turn, reading its search path each time
    const pool = openPool();
    const environment = environmentOn(new SimulatedClock(START));
    const searchPath = async (schema: string) => {
        const { sql, ready } = await pool.reserve(environment, schema);
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

/** Read who finishes each work in flight. */
function kinds(pending: readonly string[]): string[] {
    return pending.map((line) => line.split(" ")[0] ?? "");
}
