import { expect, test } from "@destack/test";
import { RateLimit, type RateLimitState } from "./rate.ts";

test("admit a burst, then one request per refilled token, answering the wait of a refused one", () => {
    // take requests in order, keeping the bucket between them
    const limit = new RateLimit({ rate: 2, burst: 3 });
    let state: RateLimitState | undefined;
    const take = (now: number) => {
        const admission = limit.admit(state, now);
        state = admission.state;

        return [admission.isAdmitted, admission.wait];
    };

    // spend the burst, wait for a refill, and cap a long quiet spell at the burst
    expect({
        burst: [0, 0, 0, 0].map(take),
        refilled: [take(499), take(500)],
        capped: [10_000, 10_000, 10_000, 10_000].map(take),
    }).toEqual({
        burst: [
            [true, 0],
            [true, 0],
            [true, 0],
            [false, 500],
        ],
        refilled: [
            [false, 2],
            [true, 0],
        ],
        capped: [
            [true, 0],
            [true, 0],
            [true, 0],
            [false, 500],
        ],
    });
});
