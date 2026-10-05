import { defineSchema, Digest, schema } from "@destack/schema";

/** The most entries one search page returns. */
const PAGE_LIMIT = 1000;

/** A primitive attribute value, as OpenTelemetry attributes carry them. */
const Primitive = schema.union([schema.string(), schema.number(), schema.boolean()]);

/** An attribute value: a primitive or a list of primitives. */
export const AttributeValue = defineSchema(schema.union([Primitive, schema.array(Primitive)]));
/** An attribute value. */
export type AttributeValue = schema.Infer<typeof AttributeValue>;

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

/** A span, a log record or a metric point an installation or a host emitted. */
export const Entry = defineSchema(
    schema.object({
        /** A timed span, a log record at a point in time, or a metric's aggregate over an interval. */
        kind: schema.enum(["span", "log", "point"]),
        /** The span's name, the log record's event name, or the metric's name. */
        name: schema.string().min(1),
        /** The start or emission time, in Unix microseconds. */
        time: schema.number().int(),
        /** The span's duration or the point's interval, in microseconds. */
        duration: schema.number().int().nonnegative().exactOptional(),
        /** The installation that emitted it, absent for the host of a host's scope. */
        installation: schema.identifier("installation").exactOptional(),
        /** The workload instance that emitted it. */
        instance: schema.string().min(1).exactOptional(),
        /** The digest of the emitting build's manifest. */
        build: Digest.exactOptional(),
        /** The instrumenting package or library. */
        source: schema.object({
            /** The package or library name. */
            name: schema.string().min(1),
            /** Its version. */
            version: schema.string(),
        }),
        /** The trace, as 32 hexadecimal digits. */
        trace: schema
            .string()
            .regex(/^[0-9a-f]{32}$/u)
            .exactOptional(),
        /** The span, or the span a log record was emitted in, as 16 hexadecimal digits. */
        span: schema
            .string()
            .regex(/^[0-9a-f]{16}$/u)
            .exactOptional(),
        /** The parent span, as 16 hexadecimal digits. */
        parent: schema
            .string()
            .regex(/^[0-9a-f]{16}$/u)
            .exactOptional(),
        /** The span's outcome. */
        status: schema.enum(["unset", "ok", "error"]),
        /** The log record's severity, 1 to 24 as OpenTelemetry numbers it. */
        severity: schema.number().int().min(1).max(24).exactOptional(),
        /** The log record's body as text. */
        body: schema.string().exactOptional(),
        /** The point's instrument: a sum of increments, a gauge's last value, or a histogram. */
        metric: schema.enum(["sum", "gauge", "histogram"]).exactOptional(),
        /** The point's unit, such as ms or {block}. */
        unit: schema.string().exactOptional(),
        /** The sum's or the gauge's value. */
        value: schema.number().exactOptional(),
        /** The histogram's distribution. */
        histogram: Histogram.exactOptional(),
        /** The attributes. */
        attributes: schema.record(schema.string(), AttributeValue),
    }),
);
/** A span, a log record or a metric point. */
export type Entry = schema.Infer<typeof Entry>;

/** The entries a search or tail selects. */
export const EntryFilter = defineSchema(
    schema.object({
        /** The space of the installation, or the host whose own entries to select. */
        scope: schema.string().min(1),
        /** The installation whose entries to select, or the scope's host when absent. */
        installation: schema.identifier("installation").exactOptional(),
        /** The lowest severity, such as 13 for warnings. */
        severity: schema.number().int().min(1).max(24).exactOptional(),
        /** Text the record's name or body contains, ignoring case. */
        text: schema.string().min(1).exactOptional(),
        /** The attribute values to select, each equal. */
        attributes: schema.record(schema.string(), AttributeValue).exactOptional(),
        /** The names to select, every name when absent. */
        names: schema.array(schema.string().min(1)).exactOptional(),
        /** The trace to select. */
        trace: schema
            .string()
            .regex(/^[0-9a-f]{32}$/u)
            .exactOptional(),
    }),
);
/** The entries a search or tail selects. */
export type EntryFilter = schema.Infer<typeof EntryFilter>;

/** A search for log records in a time window, newest first. */
export const EntrySearch = defineSchema(
    EntryFilter.extend({
        /** The earliest time, in Unix microseconds. */
        from: schema.number().int(),
        /** The time before which to search, in Unix microseconds, such as the previous page's end. */
        before: schema.number().int(),
        /** The most entries to return. */
        limit: schema.number().int().min(1).max(PAGE_LIMIT),
    }),
);
/** A search for log records. */
export type EntrySearch = schema.Infer<typeof EntrySearch>;

/** One page of a search, newest first. */
export const EntryPage = defineSchema(
    schema.object({
        /** The entries, newest first. */
        entries: schema.array(Entry),
        /** The time to search before for the next page, absent after the last page. */
        before: schema.number().int().exactOptional(),
    }),
);
/** One page of a search. */
export type EntryPage = schema.Infer<typeof EntryPage>;

/** A request for a metric's series in a window, one aggregate per step and attribute group. */
export const PointSeries = defineSchema(
    schema.object({
        /** The space of the installation, or the host whose own metric to read. */
        scope: schema.string().min(1),
        /** The installation whose metric to read, or the scope's host when absent. */
        installation: schema.identifier("installation").exactOptional(),
        /** The metric's name. */
        name: schema.string().min(1),
        /** The earliest time, in Unix microseconds. */
        from: schema.number().int(),
        /** The time before which to read, in Unix microseconds. */
        before: schema.number().int(),
        /** The step width, in microseconds, at least a minute. */
        step: schema.number().int().min(60_000_000),
        /** The points read, as a filter over their attributes, every point when absent. */
        filter: schema.string().min(1).max(1000).exactOptional(),
        /** The attributes to group by, resource attributes such as the build's included, every point into one group when empty. */
        group: schema.array(schema.string().min(1)),
    }),
);
/** A request for a metric's series. */
export type PointSeries = schema.Infer<typeof PointSeries>;

/** One step of a series: the metric's aggregate over the step. */
export const SeriesStep = defineSchema(
    schema.object({
        /** The step's start, in Unix microseconds. */
        time: schema.number().int(),
        /** The summed increments of a sum, or the last value of a gauge. */
        value: schema.number().exactOptional(),
        /** The recorded values of a histogram. */
        count: schema.number().int().nonnegative().exactOptional(),
        /** Their sum. */
        sum: schema.number().exactOptional(),
        /** The median. */
        p50: schema.number().exactOptional(),
        /** The 90th percentile. */
        p90: schema.number().exactOptional(),
        /** The 99th percentile. */
        p99: schema.number().exactOptional(),
    }),
);
/** One step of a series. */
export type SeriesStep = schema.Infer<typeof SeriesStep>;

/** A metric's series per attribute group. */
export const Series = defineSchema(
    schema.object({
        /** The series, one per attribute group, steps oldest first. */
        series: schema.array(
            schema.object({
                /** The group's attribute values. */
                attributes: schema.record(schema.string(), AttributeValue),
                /** The steps with points, oldest first. */
                steps: schema.array(SeriesStep),
            }),
        ),
    }),
);
/** A metric's series per attribute group. */
export type Series = schema.Infer<typeof Series>;
