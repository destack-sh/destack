import { expect, test } from "@destack/test";
import { DatabaseError } from "../error/error.ts";
import { SqliteWorkQueue } from "./client.ts";

test("run queued work in arrival order, one piece at a time", async () => {
    // queue three pieces, the first finishing last
    const queue = new SqliteWorkQueue();
    const order: string[] = [];
    const slow = Promise.withResolvers<void>();
    const first = queue.run(async () => {
        await slow.promise;
        order.push("first");
    });
    const second = queue.run(async () => {
        order.push("second");
    });
    slow.resolve();
    await Promise.all([first, second]);

    expect(order).toEqual(["first", "second"]);
});

test("refuse work whose turn does not come within the wait", async () => {
    // hold the connection, as a transaction whose own code queues a statement outside it does
    const queue = new SqliteWorkQueue(1);
    const held = Promise.withResolvers<void>();
    const holding = queue.run(() => held.promise);
    const refused = queue.run(async () => "never");
    const failure = await refused.then(
        () => undefined,
        (error: unknown) => error,
    );

    // run later work once the holder finishes
    held.resolve();
    await holding;
    const later = await queue.run(async () => "later");

    expect([failure, later]).toEqual([
        new DatabaseError(
            "CONNECTION_TIMEOUT",
            "waited 1 ms for the connection: a transaction still has it",
        ),
        "later",
    ]);
});
