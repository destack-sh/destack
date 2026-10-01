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
    const instruments = new Set(points.map((point) => point.metric));
    if (instruments.size > 1) {
        throw new ServiceError("CONFLICT", {
            message: `metric ${request.name} is reported as ${[...instruments].join(" and ")}`,
        });
    }

    // group each point, then fold it into its step
    const groups = new Map<
        string,
        { attributes: Record<string, AttributeValue>; steps: Map<number, Step> }
    >();
    for (const point of points) {
        const attributes = Object.fromEntries(
            request.group.flatMap((key) =>
                point.attributes[key] === undefined ? [] : [[key, point.attributes[key]!]],
            ),
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
                .sort((first, second) => first.time - second.time)
                .map((step) => close(step, points[0]!.metric!)),
        })),
    };
}

/** Fold a point into its step by its instrument. */
function fold(step: Step, point: Entry): void {
    // add increments, keep the latest gauge value, and merge histograms
    if (point.metric === "sum") {
        step.total += point.value!;
    } else if (point.metric === "gauge") {
        if (step.last === undefined || point.time >= step.last.time) {
            step.last = { time: point.time, value: point.value! };
        }
    } else {
        step.histogram =
            step.histogram === undefined
                ? point.histogram!
                : merge(step.histogram, point.histogram!);
    }
}

/** Close a step into its aggregate by instrument. */
function close(step: Step, metric: NonNullable<Entry["metric"]>): SeriesStep {
    // report a sum's total and a gauge's last value
    if (metric === "sum") {
        return { time: step.time, value: step.total };
    } else if (metric === "gauge") {
        return { time: step.time, value: step.last!.value };
    }

    // report a histogram's count, sum and percentiles
    const histogram = step.histogram!;

    return {
        time: step.time,
        count: histogram.count,
        sum: histogram.sum,
        p50: quantile(histogram, 0.5),
        p90: quantile(histogram, 0.9),
        p99: quantile(histogram, 0.99),
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

    // walk the negative buckets from the largest magnitude, then zero, then the positive ones
    let seen = 0;
    let estimate: number | undefined;
    const negative = histogram.negative;
    for (let index = negative.counts.length - 1; index >= 0 && estimate === undefined; index--) {
        seen += negative.counts[index]!;
        if (seen > rank) {
            estimate = -(base ** (negative.offset + index + 0.5));
        }
    }
    seen += histogram.zeroCount;
    if (estimate === undefined && seen > rank) {
        estimate = 0;
    }
    const positive = histogram.positive;
    for (let index = 0; index < positive.counts.length && estimate === undefined; index++) {
        seen += positive.counts[index]!;
        if (seen > rank) {
            estimate = base ** (positive.offset + index + 0.5);
        }
    }

    // keep the estimate within the recorded range
    const low = histogram.min ?? -Infinity;
    const high = histogram.max ?? Infinity;

    return estimate === undefined ? undefined : Math.min(high, Math.max(low, estimate));
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
            counts[side.offset + position - offset]! += count;
        }
    }

    return { offset, counts };
}
