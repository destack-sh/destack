import { TEST_START } from "@destack/test/world";
import { expect, refusal, testOn } from "@destack/test";
import { SimulatedClock } from "@destack/environment/simulation";
import { environmentOn } from "../test/environment.ts";
import { SqliteWorkQueue } from "./client.ts";

/** Test against SQLite alone, as the file tests its own connector or host. */
const onSqlite = testOn("sqlite");

onSqlite("run queued work in arrival order, one piece at a time", async () => {
    // queue three pieces, the first finishing last
    const environment = environmentOn(new SimulatedClock(TEST_START));
    const queue = new SqliteWorkQueue(environment.telemetry, "memory");
    const order: string[] = [];
    const slow = Promise.withResolvers<void>();
    const first = queue.run(async () => {
        await slow.promise;
        order.push("first");
    }, environment);
    const second = queue.run(async () => {
        order.push("second");
    }, environment);
    slow.resolve();
    await Promise.all([first, second]);

    expect(order).toEqual(["first", "second"]);
});

onSqlite(
    "run work awaited one piece after another at once, arming no wait for its turn",
    async () => {
        // await one piece, then queue the next as a caller's following statement does
        const environment = environmentOn(new SimulatedClock(TEST_START));
        const queue = new SqliteWorkQueue(environment.telemetry, "memory");
        await queue.run(async () => "first", environment);
        const second = queue.run(async () => "second", environment);
        const armed = environment.clock.next;

        // find the queue idle, with no timer armed for the turn
        expect([armed, await second]).toEqual([undefined, "second"]);
    },
);

onSqlite("refuse each queued turn at its own deadline, keeping later turns waiting", async () => {
    // queue one turn, then another halfway through the first one's wait
    const environment = environmentOn(new SimulatedClock(TEST_START));
    const queue = new SqliteWorkQueue(environment.telemetry, "memory", 10);
    const held = Promise.withResolvers<void>();
    const holding = queue.run(() => held.promise, environment);
    const early = queue.run(async () => "early", environment);
    environment.clock.advance(6);
    const late = queue.run(async () => "late", environment);

    // pass the first deadline alone, then let the holder finish before the second
    environment.clock.advance(4);
    const failure = await refusal(early);
    held.resolve();
    await holding;

    expect([failure, await late]).toEqual([
        [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_UNAVAILABLE",
            "the connection is unavailable: a transaction still had it after 10 ms",
        ],
        "late",
    ]);
});

onSqlite("refuse work whose turn does not come within the wait", async () => {
    // occupy the connection, as a transaction does whose code queues a statement outside it
    const environment = environmentOn(new SimulatedClock(TEST_START));
    const queue = new SqliteWorkQueue(environment.telemetry, "memory", 1);
    const held = Promise.withResolvers<void>();
    const holding = queue.run(() => held.promise, environment);
    const refused = queue.run(async () => "never", environment);
    environment.clock.advance(1);
    const failure = await refusal(refused);

    // run later work once the holder finishes
    held.resolve();
    await holding;
    const later = await queue.run(async () => "later", environment);

    expect([failure, later]).toEqual([
        [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_UNAVAILABLE",
            "the connection is unavailable: a transaction still had it after 1 ms",
        ],
        "later",
    ]);
});
