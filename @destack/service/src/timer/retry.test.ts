import { expect, test } from "@destack/test";
import { RetryPolicy } from "./retry.ts";

test("grow waits by the coefficient up to the maximum, spread below the interval with full jitter", () => {
    // double up to four seconds, then jitter
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

test("pause before the next attempt, ending quietly once the signal aborts", async () => {
    // wait a short interval and end on abort
    const policy = RetryPolicy.of({ initialInterval: 5 });
    const running = new AbortController();
    const aborted = new AbortController();
    aborted.abort(new Error("stopped"));
    expect([
        await RetryPolicy.pause(policy, 1, running.signal),
        await RetryPolicy.pause(policy, 1, aborted.signal),
    ]).toEqual([true, false]);
});
