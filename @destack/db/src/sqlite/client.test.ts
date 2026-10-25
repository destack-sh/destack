import { expect, test, refusal } from "@destack/test";
import { SimulatedClock } from "@destack/environment/simulation";
import { SqliteWorkQueue } from "./client.ts";

/** The moment the queues' clocks start at: 2026-10-01T00:00:00Z. */
const START = Date.UTC(2026, 9, 1);

test("run queued work in arrival order, one piece at a time", async () => {
    // queue three pieces, the first finishing last
    const clock = new SimulatedClock(START);
    const queue = new SqliteWorkQueue();
    const order: string[] = [];
    const slow = Promise.withResolvers<void>();
    const first = queue.run(async () => {
        await slow.promise;
        order.push("first");
    }, clock);
    const second = queue.run(async () => {
        order.push("second");
    }, clock);
    slow.resolve();
    await Promise.all([first, second]);

    expect(order).toEqual(["first", "second"]);
});

test("refuse work whose turn does not come within the wait", async () => {
    // occupy the connection, as a transaction does whose code queues a statement outside it
    const clock = new SimulatedClock(START);
    const queue = new SqliteWorkQueue(1);
    const held = Promise.withResolvers<void>();
    const holding = queue.run(() => held.promise, clock);
    const refused = queue.run(async () => "never", clock);
    clock.advance(1);
    const failure = await refusal(refused);

    // run later work once the holder finishes
    held.resolve();
    await holding;
    const later = await queue.run(async () => "later", clock);

    expect([failure, later]).toEqual([
        [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_TIMEOUT",
            "waited 1 ms for the connection: a transaction still has it",
        ],
        "later",
    ]);
});
