import { through, union } from "@destack/access";
import { machine, space } from "@destack/account/object";
import { Histogram } from "@destack/event";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { installation } from "@destack/stack/object";
import { AttributeKey, Instrumentation } from "../event/attribute.ts";
import { TELEMETRY_FLUSH, TELEMETRY_RETENTION } from "../event/policy.ts";

/** The instruments a metric point aggregates. */
export const INSTRUMENTS = ["sum", "gauge", "histogram"] as const;

/** What a metric point keeps beside its keys. */
export const MetricData = defineSchema(
    schema.object({
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
);

/** One data point of a metric an installation or a machine emitted. */
export const metric = defineObject(
    {
        name: "metric",
        plural: "metrics",
        scope: [space, machine],
        storage: {
            kind: "events",
            flush: TELEMETRY_FLUSH,
            retention: TELEMETRY_RETENTION,
            delivery: "at-most-once",
            distribution: "histogram",
        },
        fields: {
            /** The point's interval, unit and distribution. */
            data: field.json(MetricData),
            /** The installation that emitted it, absent for the machine's own metrics. */
            installation: field.reference(installation).optional(),
            /** The digest of the emitting build's manifest. */
            build: field.string().optional(),
            /** The metric's name. */
            name: field.string(),
            /** The instrument: sum, gauge or histogram. */
            instrument: field.enum(INSTRUMENTS),
            /** A sum's increment, a gauge's value, or a histogram's sum. */
            value: field.number(),
            /** A histogram's count of values, absent for a sum or a gauge. */
            count: field.integer().optional(),
            /** The attributes, bounded as the metric declares them. */
            attributes: field.json(AttributeKey),
        },
        permissions: {
            read: union(through("installation", "observe"), through("scope", "observe")),
            // let the machine serving the scope append what it forwards
            append: through("scope", "serve"),
        },
    },
    (method) => ({
        /** Append the records an export carries. */
        append: method.createMany("append", {
            fields: [
                "installationId",
                "build",
                "name",
                "instrument",
                "value",
                "count",
                "attributes",
                "data",
            ],
        }),
    }),
);
