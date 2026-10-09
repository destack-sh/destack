import type { EventFilter, EventStore, Series } from "@destack/event";
import { EventKey } from "../event/key.ts";
import { metric } from "../event/index.ts";
import { merge, quantile } from "./histogram.ts";
import type { Histogram } from "../event/index.ts";

/** The quantile each percentile fold reads from merged histograms. */
export const QUANTILES = { p50: 0.5, p90: 0.9, p99: 0.99 } as const;

/** The percentile folds of histogram points. */
export const QUANTILE_FOLDS = [
    "p50",
    "p90",
    "p99",
] as const satisfies readonly (keyof typeof QUANTILES)[];

/** A percentile fold of histogram points. */
export type QuantileFold = (typeof QUANTILE_FOLDS)[number];

/** The microseconds in a millisecond, between event times and steps. */
const MICROSECONDS = 1000;

/** Fold histogram points into percentiles, which the event store's folds cannot merge. */
export const MetricFold = {
    /** Report whether a fold is a percentile of histogram points. */
    isQuantile(fold: string): fold is QuantileFold {
        return Object.hasOwn(QUANTILES, fold);
    },

    /** Read a percentile per step and group of the histogram points a filter selects, merging each group's histograms of a step. */
    async series(
        store: EventStore,
        filter: EventFilter,
        request: {
            readonly fold: QuantileFold;
            readonly group: readonly string[];
            readonly step?: number;
        },
    ): Promise<Series[]> {
        // merge each group's histograms per step
        const width = request.step === undefined ? undefined : request.step * MICROSECONDS;
        const groups = new Map<
            string,
            {
                readonly group: Record<string, string | number | null>;
                readonly steps: Map<number, { histogram: Histogram; events: number }>;
            }
        >();
        const histograms = {
            ...filter,
            where: { AND: [filter.where ?? {}, { instrument: "histogram" }] },
        };
        for await (const event of store.export(metric, histograms)) {
            // read the point's group and step
            const { histogram } = event.data;
            if (histogram === undefined) {
                continue;
            }
            const group = Object.fromEntries(
                request.group.map((name) => [name, EventKey.read(event.keys, name)]),
            );
            const key = JSON.stringify(group);
            const held = groups.get(key) ?? {
                group,
                steps: new Map<number, { histogram: Histogram; events: number }>(),
            };
            groups.set(key, held);
            const start =
                width === undefined ? (filter.from ?? 0) : Math.floor(event.time / width) * width;

            // merge it into the step
            const step = held.steps.get(start);
            held.steps.set(
                start,
                step === undefined
                    ? { histogram, events: 1 }
                    : { histogram: merge(step.histogram, histogram), events: step.events + 1 },
            );
        }

        // read each step's percentile, oldest first, leaving out steps whose histograms hold no values
        return [...groups.values()].map((held) => ({
            group: held.group,
            steps: [...held.steps.entries()]
                .toSorted(([left], [right]) => left - right)
                .flatMap(([start, step]) => {
                    const value = quantile(step.histogram, QUANTILES[request.fold]);

                    return value === undefined ? [] : [{ start, value, events: step.events }];
                }),
        }));
    },
};
