import { expect, test } from "@destack/test";
import { RetryPolicy } from "./retry.ts";
import { SimulatedClock } from "../simulation/index.ts";

test("grow waits by the coefficient up to the maximum, spread below the interval with full jitter", () => {
    // double up to four seconds and jitter
    const policy = RetryPolicy.of({ maximumInterval: 4000 });
    const jittered = { ...policy, jitter: "full" as const };
    expect([
        [1, 2, 3, 4].map((failures) => RetryPolicy.interval(policy, failures, () => 0.5)),
        [1, 2, 3, 4].map((failures) => RetryPolicy.interval(jittered, failures, () => 0.5)),
    ]).toEqual([
        [1000, 2000, 4000, 4000],
        [500, 1000, 2000, 2000],
    ]);
});

test("retry until stopped by default, or up to the maximum attempts", () => {
    const bounded = RetryPolicy.of({ maximumAttempts: 3 });
    expect([
        RetryPolicy.isRetried(RetryPolicy.of(), 1000),
        [1, 2, 3].map((failures) => RetryPolicy.isRetried(bounded, failures)),
    ]).toEqual([true, [true, true, false]]);
});

test("pause on the clock until it passes the interval, ending quietly once the signal aborts", async () => {
    // wait a second of a stopped clock, and end on abort
    const clock = new SimulatedClock(0);
    const policy = RetryPolicy.of();
    const running = new AbortController();
    const aborted = new AbortController();
    aborted.abort(new Error("stopped"));
    const paused = RetryPolicy.pause(policy, 1, clock, running.signal, () => 0.5);

    // release the pause once the clock passes the interval
    clock.advance(1000);
    expect([
        await paused,
        await RetryPolicy.pause(policy, 1, clock, aborted.signal, () => 0.5),
    ]).toEqual([true, false]);
});

test("wait whole milliseconds, rounding a jittered or grown fraction up", () => {
    // jitter a second by a third, and grow 1001 ms by half
    const jittered = { ...RetryPolicy.of(), jitter: "full" as const };
    const grown = RetryPolicy.of({ initialInterval: 1001, backoffCoefficient: 1.5 });
    expect([
        RetryPolicy.interval(jittered, 1, () => 1 / 3),
        RetryPolicy.interval(grown, 2, () => 0.5),
    ]).toEqual([334, 1502]);
});

test("draw the same fraction from the same seed and another from another seed", () => {
    // draw twice from one seed and once from another
    const drawn = ["connection-1 1", "connection-1 1", "connection-1 2"].map((seed) =>
        RetryPolicy.draw(seed),
    );

    expect(drawn).toEqual([
        5683553230138032 / 2 ** 53,
        5683553230138032 / 2 ** 53,
        1860537867773066 / 2 ** 53,
    ]);
});
