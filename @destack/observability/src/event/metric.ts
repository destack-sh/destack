import { defineEventKind } from "@destack/event/declare";
import { TELEMETRY_ACCESS } from "./access.ts";
import { defineSchema, schema } from "@destack/schema";
import { AttributeKey, Instrumentation } from "./attribute.ts";
import { DAY, TELEMETRY_FLUSH } from "./policy.ts";

/** The instruments a metric point aggregates: a sum of increments, a gauge's value, or a histogram. */
export const INSTRUMENTS = ["sum", "gauge", "histogram"] as const;

/** The buckets of a base-2 exponential histogram above or below zero. */
const Buckets = schema.object({
    /** The index of the first bucket. */
    offset: schema.number().int(),
    /** The count of each bucket from the first. */
    counts: schema.array(schema.number().int().nonnegative()),
});

/** A base-2 exponential histogram: bucket i above zero counts values in (base^i, base^(i+1)], base = 2^(2^-scale). */
export const Histogram = defineSchema(
    schema.object({
        /** The recorded values. */
        count: schema.number().int().nonnegative(),
        /** Their sum. */
        sum: schema.number(),
        /** The smallest value, when recorded. */
        min: schema.number().exactOptional(),
        /** The largest value, when recorded. */
        max: schema.number().exactOptional(),
        /** The resolution: each bucket spans a factor of 2^(2^-scale). */
        scale: schema.number().int().min(-10).max(20),
        /** The values equal to zero. */
        zeroCount: schema.number().int().nonnegative(),
        /** The buckets above zero. */
        positive: Buckets,
        /** The buckets below zero. */
        negative: Buckets,
    }),
);
/** A base-2 exponential histogram. */
export type Histogram = schema.Infer<typeof Histogram>;

/** One data point of a metric an installation or a machine emitted, as OpenTelemetry's metric data model records it. */
export const metric = defineEventKind({
    name: "metric",
    description: "One data point of a metric an installation or a machine emitted.",
    keys: schema.object({
        /** The installation that emitted it, null for the machine's own metrics. */
        installation: schema.string().nullable(),
        /** The digest of the emitting build's manifest. */
        build: schema.string().nullable(),
        /** The metric's name. */
        name: schema.string(),
        /** The instrument: sum, gauge or histogram. */
        instrument: schema.enum(INSTRUMENTS),
        /** A sum's increment, a gauge's value, or a histogram's sum. */
        value: schema.number(),
        /** A histogram's count of values, null for a sum or a gauge. */
        count: schema.number().int().nonnegative().nullable(),
        /** The attributes, bounded as the metric declares them. */
        attributes: AttributeKey,
    }),
    data: schema.object({
        /** The start of the interval the point aggregates, in Unix microseconds. */
        start: schema.number().int(),
        /** The unit, such as ms or {block}. */
        unit: schema.string().exactOptional(),
        /** A histogram's distribution. */
        histogram: Histogram.exactOptional(),
        /** The workload instance that emitted it. */
        instance: schema.string().exactOptional(),
        /** The library that recorded it. */
        instrumentation: Instrumentation,
    }),
    delivery: "at-most-once",
    policy: { flush: TELEMETRY_FLUSH, retention: 30 * DAY },
    access: TELEMETRY_ACCESS,
});
