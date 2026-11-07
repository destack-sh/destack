import { expect, test } from "@destack/test";
import { channelHub } from "./channel.ts";

test("deliver a message to the other parties of its channel's name alone", async () => {
    // join two parties on each of two names
    const join = channelHub<string>();
    const received: string[] = [];
    const [commits, others] = [join("log"), join("log")];
    const [tracker, peer] = [join("tracker"), join("tracker")];
    for (const [name, channel] of [
        ["log sender", commits],
        ["log", others],
        ["tracker sender", tracker],
        ["tracker", peer],
    ] as const) {
        channel.listen({ receive: (message) => received.push(`${name}: ${message}`) });
    }

    // notify on each name, after which the hub delivers in the next microtasks
    commits.notify("commit");
    tracker.notify("hello");
    await Promise.resolve();

    expect(received.toSorted()).toEqual(["log: commit", "tracker: hello"]);
});
