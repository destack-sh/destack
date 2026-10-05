import { expect, test } from "@destack/test";
import type { Entry, Histogram } from "../entry/index.ts";
import { aggregate, merge, quantile } from "./series.ts";

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

/** Stamp a point of the request metric at a time with a route. */
function point(
    time: number,
    route: string,
    values: Pick<Entry, "metric" | "value" | "histogram">,
): Entry {
    return {
        kind: "point",
        name: "request",
        time,
        duration: 60_000_000,
        source: { name: "@example/notes", version: "2026.9.0" },
        status: "unset",
        attributes: { route, method: "GET" },
        ...values,
    };
}

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

test("aggregate points into steps per attribute group: summed increments, last gauges and merged histograms", () => {
    // aggregate one metric by route in minute steps
    const request = {
        scope: "space-1",
        name: "request",
        from: 0,
        before: 180_000_000,
        step: 60_000_000,
        group: ["route"],
    };

    // sum two steps of route a and one of route b
    const sums = aggregate(
        [
            point(0, "a", { metric: "sum", value: 2 }),
            point(10_000_000, "a", { metric: "sum", value: 3 }),
            point(70_000_000, "a", { metric: "sum", value: 1 }),
            point(5_000_000, "b", { metric: "sum", value: 4 }),
        ],
        request,
    );

    // keep each step's latest gauge value
    const gauges = aggregate(
        [
            point(20_000_000, "a", { metric: "gauge", value: 7 }),
            point(10_000_000, "a", { metric: "gauge", value: 9 }),
        ],
        request,
    );

    // merge one step's histograms from two instances
    const histograms = aggregate(
        [
            point(0, "a", { metric: "histogram", histogram: coarse }),
            point(30_000_000, "a", { metric: "histogram", histogram: fine }),
        ],
        request,
    );

    expect({ sums, gauges, histograms }).toEqual({
        sums: {
            series: [
                {
                    attributes: { route: "a" },
                    steps: [
                        { time: 0, value: 5 },
                        { time: 60_000_000, value: 1 },
                    ],
                },
                { attributes: { route: "b" }, steps: [{ time: 0, value: 4 }] },
            ],
        },
        gauges: { series: [{ attributes: { route: "a" }, steps: [{ time: 0, value: 7 }] }] },
        histograms: {
            series: [
                {
                    attributes: { route: "a" },
                    steps: [
                        { time: 0, count: 5, sum: 46, p50: 2 ** 2.5, p90: 2 ** 3.5, p99: 2 ** 3.5 },
                    ],
                },
            ],
        },
    });
});
