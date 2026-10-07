import type { EventKeyShape } from "../kind/kind.ts";

/** The microseconds in a millisecond, between event times and steps. */
const MICROSECONDS = 1000;

/** How a series folds the measured key of the events in each step. */
export type Fold = "sum" | "count" | "min" | "max" | "average" | "last";

/** How a series folds a kind's events: the measured key, the fold, the grouping keys and the step. */
export interface SeriesRequest<Shape extends EventKeyShape> {
    /** The numeric query key folded, needed by every fold but count. */
    readonly measure?: keyof Shape & string;
    /** The fold. */
    readonly fold: Fold;
    /** The query keys whose values each get a series of their own. */
    readonly group?: readonly (keyof Shape & string)[];
    /** The width of each step, in milliseconds, one step over the whole range when absent. */
    readonly step?: number;
}

/** One group's folded steps in time order. */
export interface Series {
    /** The value of each grouping key. */
    readonly group: Readonly<Record<string, string | number | null>>;
    /** The steps holding events. */
    readonly steps: readonly SeriesStep[];
}

/** One step of a series: its start, its folded value and the events folded. */
export interface SeriesStep {
    /** The step's start, in Unix microseconds, the range's start for a single step. */
    readonly start: number;
    /** The folded value. */
    readonly value: number;
    /** The events folded. */
    readonly events: number;
}

/** One group's step while folding: what every fold needs. */
export interface StepFold {
    /** The events folded. */
    readonly events: number;
    /** The measured values' sum. */
    readonly sum: number;
    /** The smallest measured value. */
    readonly min: number;
    /** The largest measured value. */
    readonly max: number;
    /** The latest event's time and measured value, absent when folded in SQL. */
    readonly last: { readonly time: number; readonly value: number } | undefined;
}

/** The series of a request while its events and partial folds come in, by group and step. */
export class SeriesFold<Shape extends EventKeyShape> {
    /** The measured key, absent for a count. */
    readonly measure: string | undefined;
    /** The grouping keys. */
    readonly group: readonly string[];
    /** The width of each step, in microseconds, absent for one step over the whole range. */
    readonly width: number | undefined;
    /** The fold. */
    readonly #fold: Fold;
    /** The range's start, in Unix microseconds, which a single step starts at. */
    readonly #from: number;
    /** Each group's values and steps, by the group's values as JSON. */
    readonly #groups = new Map<
        string,
        {
            readonly group: Record<string, string | number | null>;
            readonly steps: Map<number, StepFold>;
        }
    >();

    /** Start folding a request over a range, refusing a fold but count without a measure. */
    constructor(request: SeriesRequest<Shape>, from: number | undefined) {
        // require a measure for every fold but count
        if (request.measure === undefined && request.fold !== "count") {
            throw new TypeError(`a ${request.fold} series folds a measured key`);
        }
        this.measure = request.measure;
        this.group = request.group ?? [];
        this.width = request.step === undefined ? undefined : request.step * MICROSECONDS;
        this.#fold = request.fold;
        this.#from = from ?? 0;
    }

    /** Read the start of the step a time falls in. */
    startOf(time: number): number {
        return this.width === undefined ? this.#from : Math.floor(time / this.width) * this.width;
    }

    /** Fold a partial fold into the step of its group, as SQL folds hot events. */
    add(keys: Readonly<Record<string, unknown>>, start: number, fold: StepFold): void {
        // merge the fold into the step of the group its keys' values name
        const values = Object.fromEntries(this.group.map((name) => [name, scalarOf(keys[name])]));
        const key = JSON.stringify(values);
        const held = this.#groups.get(key) ?? { group: values, steps: new Map<number, StepFold>() };
        held.steps.set(start, merged(held.steps.get(start), fold));
        this.#groups.set(key, held);
    }

    /** Fold one event's measured value into its step, leaving out an event without one. */
    addEvent(event: {
        readonly time: number;
        readonly keys: Readonly<Record<string, unknown>>;
    }): void {
        const value = this.measure === undefined ? 0 : event.keys[this.measure];
        if (typeof value === "number") {
            this.add(event.keys, this.startOf(event.time), {
                events: 1,
                sum: value,
                min: value,
                max: value,
                last: { time: event.time, value },
            });
        }
    }

    /** Close each group's steps by the fold, oldest first. */
    close(): Series[] {
        return [...this.#groups.values()].map((held) => ({
            group: held.group,
            steps: [...held.steps.entries()]
                .toSorted(([left], [right]) => left - right)
                .map(([start, fold]) => ({
                    start,
                    value: folded(this.#fold, fold),
                    events: fold.events,
                })),
        }));
    }
}

/** Read a key's value as a group keeps it. */
function scalarOf(value: unknown): string | number | null {
    return typeof value === "string" || typeof value === "number" ? value : null;
}

/** Merge two folds of one step. */
function merged(left: StepFold | undefined, right: StepFold): StepFold {
    if (left === undefined) {
        return right;
    }

    return {
        events: left.events + right.events,
        sum: left.sum + right.sum,
        min: Math.min(left.min, right.min),
        max: Math.max(left.max, right.max),
        last:
            (left.last?.time ?? -Infinity) > (right.last?.time ?? -Infinity)
                ? left.last
                : right.last,
    };
}

/** Close a step's fold. */
function folded(fold: Fold, step: StepFold): number {
    switch (fold) {
        case "sum":
            return step.sum;
        case "count":
            return step.events;
        case "min":
            return step.min;
        case "max":
            return step.max;
        case "average":
            return step.sum / step.events;
        case "last":
            return step.last?.value ?? 0;
    }
}
