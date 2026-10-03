import { ServiceError } from "@destack/service/error";
import type {
    AttributeValue,
    Entry,
    Histogram,
    PointSeries,
    Series,
    SeriesStep,
} from "../entry/index.ts";

/** The buckets of one side of a histogram. */
type Buckets = Histogram["positive"];

/** One step's aggregate of one attribute group while collecting. */
interface Step {
    /** The step's start, in Unix microseconds. */
    readonly time: number;
    /** The summed increments of a sum. */
    total: number;
    /** The latest gauge value and its time. */
    last?: { readonly time: number; readonly value: number };
    /** The merged histogram. */
    histogram?: Histogram;
}

/** Aggregate a metric's points into steps per attribute group, steps oldest first. */
export function aggregate(points: readonly Entry[], request: PointSeries): Series {
    // refuse a metric its emitters report as different instruments
    const instruments = [...new Set(points.map((point) => point.metric))];
    const [metric] = instruments;
    if (instruments.length > 1) {
        throw new ServiceError("CONFLICT", {
            message: `metric ${request.name} is reported as ${instruments.join(" and ")}`,
        });
    }

    // group each point, then fold it into its step
    const groups = new Map<
        string,
        { attributes: Record<string, AttributeValue>; steps: Map<number, Step> }
    >();
    for (const point of points) {
        const attributes = Object.fromEntries(
            request.group.flatMap((key) => {
                const value = point.attributes[key];

                return value === undefined ? [] : [[key, value]];
            }),
        );
        const key = JSON.stringify(attributes);
        let group = groups.get(key);
        if (group === undefined) {
            group = { attributes, steps: new Map() };
            groups.set(key, group);
        }
        const index = Math.floor((point.time - request.from) / request.step);
        let step = group.steps.get(index);
        if (step === undefined) {
            step = { time: request.from + index * request.step, total: 0 };
            group.steps.set(index, step);
        }
        fold(step, point);
    }

    // emit each group's steps oldest first
    return {
        series: [...groups.values()].map((group) => ({
            attributes: group.attributes,
            steps: [...group.steps.values()]
                .toSorted((first, second) => first.time - second.time)
                .map((step) => close(step, metric)),
        })),
    };
}

/** Fold a point into its step by its instrument. */
function fold(step: Step, point: Entry): void {
    // add increments, keep the latest gauge value, and merge histograms
    const { metric, value, histogram } = point;
    if (metric === "sum" && value !== undefined) {
        step.total += value;
    } else if (metric === "gauge" && value !== undefined) {
        if (step.last === undefined || point.time >= step.last.time) {
            step.last = { time: point.time, value };
        }
    } else if (metric === "histogram" && histogram !== undefined) {
        step.histogram =
            step.histogram === undefined ? histogram : merge(step.histogram, histogram);
    }
    // refuse a point missing its instrument's value
    else {
        throw new TypeError(`point ${point.name} carries no value for its instrument`);
    }
}

/** Close a step into its aggregate by instrument. */
function close(step: Step, metric: Entry["metric"]): SeriesStep {
    // report a sum's total and a gauge's last value
    const { last, histogram } = step;
    if (metric === "sum") {
        return { time: step.time, value: step.total };
    } else if (metric === "gauge" && last !== undefined) {
        return { time: step.time, value: last.value };
    } else if (metric !== "histogram" || histogram === undefined) {
        throw new TypeError(`step at ${step.time} folded no ${metric ?? "metric"} point`);
    }

    // estimate a histogram's percentiles, absent while it is empty
    const p50 = quantile(histogram, 0.5);
    const p90 = quantile(histogram, 0.9);
    const p99 = quantile(histogram, 0.99);

    return {
        time: step.time,
        count: histogram.count,
        sum: histogram.sum,
        ...(p50 === undefined ? {} : { p50 }),
        ...(p90 === undefined ? {} : { p90 }),
        ...(p99 === undefined ? {} : { p99 }),
    };
}

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

    // order the negative buckets from the largest magnitude, then zero, then the positive ones
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
function downscale(buckets: Buckets, steps: number): Buckets {
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
function add(first: Buckets, second: Buckets): Buckets {
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
