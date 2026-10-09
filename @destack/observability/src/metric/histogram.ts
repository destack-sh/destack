import type { Histogram } from "../event/index.ts";

/** Merge two histograms at the coarser of their scales. */
export function merge(first: Histogram, second: Histogram): Histogram {
    // bring both to the coarser scale and keep the recorded range
    const scale = Math.min(first.scale, second.scale);
    const lowest = [first.min, second.min].filter((value) => value !== undefined);
    const highest = [first.max, second.max].filter((value) => value !== undefined);

    return {
        count: first.count + second.count,
        sum: first.sum + second.sum,
        ...(lowest.length === 0 ? {} : { min: Math.min(...lowest) }),
        ...(highest.length === 0 ? {} : { max: Math.max(...highest) }),
        scale,
        zeroCount: first.zeroCount + second.zeroCount,
        positive: add(
            downscale(first.positive, first.scale - scale),
            downscale(second.positive, second.scale - scale),
        ),
        negative: add(
            downscale(first.negative, first.scale - scale),
            downscale(second.negative, second.scale - scale),
        ),
    };
}

/** Estimate a quantile from a histogram: the geometric middle of the bucket with its rank. */
export function quantile(histogram: Histogram, fraction: number): number | undefined {
    // find the rank among the values, lowest first
    if (histogram.count === 0) {
        return undefined;
    }
    const rank = fraction * (histogram.count - 1);
    const base = 2 ** (2 ** -histogram.scale);

    // order the negative buckets from the largest magnitude, followed by zero and the positive ones
    const { negative, positive } = histogram;
    const buckets = [
        ...negative.counts
            .map((count, index) => ({
                count,
                middle: -(base ** (negative.offset + index + 0.5)),
            }))
            .toReversed(),
        { count: histogram.zeroCount, middle: 0 },
        ...positive.counts.map((count, index) => ({
            count,
            middle: base ** (positive.offset + index + 0.5),
        })),
    ];

    // keep the middle of the bucket with the rank within the recorded range
    const low = histogram.min ?? -Infinity;
    const high = histogram.max ?? Infinity;
    let seen = 0;
    for (const bucket of buckets) {
        seen += bucket.count;
        if (seen > rank) {
            return Math.min(high, Math.max(low, bucket.middle));
        }
    }

    return undefined;
}

/** Coarsen buckets by a number of scale steps: each step halves the bucket count. */
function downscale(buckets: Histogram["positive"], steps: number): Histogram["positive"] {
    // keep buckets already at the scale
    if (steps === 0 || buckets.counts.length === 0) {
        return buckets;
    }

    // fold each bucket into its coarser bucket
    const divisor = 2 ** steps;
    const offset = Math.floor(buckets.offset / divisor);
    const counts: number[] = [];
    for (const [position, count] of buckets.counts.entries()) {
        const index = Math.floor((buckets.offset + position) / divisor) - offset;
        counts[index] = (counts[index] ?? 0) + count;
    }

    return { offset, counts: Array.from(counts, (count) => count ?? 0) };
}

/** Add two sides of buckets at the same scale. */
function add(first: Histogram["positive"], second: Histogram["positive"]): Histogram["positive"] {
    // keep one side when the other is empty
    if (first.counts.length === 0) {
        return second;
    } else if (second.counts.length === 0) {
        return first;
    }

    // add the counts over the union of both ranges
    const offset = Math.min(first.offset, second.offset);
    const end = Math.max(first.offset + first.counts.length, second.offset + second.counts.length);
    const counts = Array.from({ length: end - offset }, () => 0);
    for (const side of [first, second]) {
        for (const [position, count] of side.counts.entries()) {
            const index = side.offset + position - offset;
            counts[index] = (counts[index] ?? 0) + count;
        }
    }

    return { offset, counts };
}
