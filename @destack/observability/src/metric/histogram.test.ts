import { expect, test } from "@destack/test";
import type { Histogram } from "../event/index.ts";
import { merge, quantile } from "./histogram.ts";

/** Values 3, 5, 6 and 12 at scale 0, where bucket i counts (2^i, 2^(i+1)]. */
const coarse: Histogram = {
    count: 4,
    sum: 26,
    min: 3,
    max: 12,
    scale: 0,
    zeroCount: 0,
    positive: { offset: 1, counts: [1, 2, 1] },
    negative: { offset: 0, counts: [] },
};

/** The value 20 at scale 1, where bucket i counts (2^(i/2), 2^((i+1)/2)]: bucket 8 counts (16, 22.6]. */
const fine: Histogram = {
    count: 1,
    sum: 20,
    min: 20,
    max: 20,
    scale: 1,
    zeroCount: 0,
    positive: { offset: 8, counts: [1] },
    negative: { offset: 0, counts: [] },
};

test("merge histograms at the coarser scale and read quantiles from the bucket with each rank", () => {
    const merged = merge(coarse, fine);

    // fold bucket 8 at scale 1 into bucket 4 at scale 0 and keep every count, the sum and the range
    expect(merged).toEqual({
        count: 5,
        sum: 46,
        min: 3,
        max: 20,
        scale: 0,
        zeroCount: 0,
        positive: { offset: 1, counts: [1, 2, 1, 1] },
        negative: { offset: 0, counts: [] },
    });

    // estimate each quantile as its bucket's geometric middle: 2^2.5 for rank 2, 2^3.5 for rank 3.96
    expect([quantile(merged, 0.5), quantile(merged, 0.99), quantile(fine, 0.5)]).toEqual([
        2 ** 2.5,
        2 ** 3.5,
        20,
    ]);
});
