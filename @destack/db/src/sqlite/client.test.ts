import { TEST_START } from "@destack/test/world";
import { expect, test, refusal } from "@destack/test";
import { SimulatedClock } from "@destack/environment/simulation";
import { environmentOn } from "../test/environment.ts";
import { SqliteWorkQueue } from "./client.ts";

test("run queued work in arrival order, one piece at a time", async () => {
    // queue three pieces, the first finishing last
    const environment = environmentOn(new SimulatedClock(TEST_START));
    const queue = new SqliteWorkQueue();
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

test("refuse work whose turn does not come within the wait", async () => {
    // occupy the connection, as a transaction does whose code queues a statement outside it
    const environment = environmentOn(new SimulatedClock(TEST_START));
    const queue = new SqliteWorkQueue(1);
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
            "CONNECTION_TIMEOUT",
            "waited 1 ms for the connection: a transaction still has it",
        ],
        "later",
    ]);
});
