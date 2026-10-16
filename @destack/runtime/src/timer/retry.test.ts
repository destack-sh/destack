import { expect, test } from "@destack/test";
import { RetryPolicy } from "./retry.ts";
import { VirtualClock } from "@destack/environment/simulation";

test("grow waits by the coefficient up to the maximum, spread below the interval with full jitter", () => {
    // double up to four seconds and jitter
    const policy = RetryPolicy.of({ maximumInterval: 4000 });
    const jittered = { ...policy, jitter: "full" as const };
    expect([
        [1, 2, 3, 4].map((failures) => RetryPolicy.interval(policy, failures)),
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
    const clock = new VirtualClock(0);
    const policy = RetryPolicy.of();
    const running = new AbortController();
    const aborted = new AbortController();
    aborted.abort(new Error("stopped"));
    const paused = RetryPolicy.pause(policy, 1, clock, running.signal);

    // release the pause once the clock passes the interval
    clock.advance(1000);
    expect([await paused, await RetryPolicy.pause(policy, 1, clock, aborted.signal)]).toEqual([
        true,
        false,
    ]);
});
