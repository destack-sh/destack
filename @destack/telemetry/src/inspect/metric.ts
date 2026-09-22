import { defineSchema, schema } from "@destack/schema";
import {
    AggregationTemporality,
    DataPointType,
    type MetricData,
    type ResourceMetrics,
} from "@opentelemetry/sdk-metrics";
import { AttributeDescription, describeAttributes } from "./value.ts";
import {
    describeResource,
    describeScope,
    ResourceDescription,
    ScopeDescription,
    TimeDescription,
} from "./resource.ts";

/** Standard aggregation intervals. */
const TEMPORALITY = {
    [AggregationTemporality.DELTA]: "delta",
    [AggregationTemporality.CUMULATIVE]: "cumulative",
} as const;

/** A floating point observation, including non-finite values. */
const NumberDescription = schema.union([
    schema.number(),
    schema.enum(["NaN", "Infinity", "-Infinity"]),
]);
/** Fields shared by every metric point. */
const PointDescription = schema.object({
    startTime: TimeDescription,
    endTime: TimeDescription,
    attributes: AttributeDescription,
});
/** Fields shared by every metric aggregation. */
const AggregationDescription = schema.object({
    name: schema.string(),
    description: schema.string(),
    unit: schema.string(),
    valueType: schema.number().int(),
    temporality: schema.enum(["delta", "cumulative"]),
});
/** Histogram summary values. */
const HistogramDescription = schema.object({
    count: schema.number().int(),
    sum: NumberDescription.optional(),
    min: NumberDescription.optional(),
    max: NumberDescription.optional(),
});
/** A range of exponential histogram buckets. */
const BucketDescription = schema.object({
    offset: schema.number().int(),
    bucketCounts: schema.array(schema.number().int()),
});

/** A metric aggregation with the corresponding point value shape. */
export const MetricDescription = defineSchema(
    schema.discriminatedUnion("kind", [
        AggregationDescription.extend({
            kind: schema.literal("sum"),
            isMonotonic: schema.boolean(),
            points: schema.array(PointDescription.extend({ value: NumberDescription })),
        }),
        AggregationDescription.extend({
            kind: schema.literal("gauge"),
            points: schema.array(PointDescription.extend({ value: NumberDescription })),
        }),
        AggregationDescription.extend({
            kind: schema.literal("histogram"),
            points: schema.array(
                PointDescription.extend({
                    value: HistogramDescription.extend({
                        buckets: schema.object({
                            boundaries: schema.array(NumberDescription),
                            counts: schema.array(schema.number().int()),
                        }),
                    }),
                }),
            ),
        }),
        AggregationDescription.extend({
            kind: schema.literal("exponentialHistogram"),
            points: schema.array(
                PointDescription.extend({
                    value: HistogramDescription.extend({
                        scale: schema.number().int(),
                        zeroCount: schema.number().int(),
                        positive: BucketDescription,
                        negative: BucketDescription,
                    }),
                }),
            ),
        }),
    ]),
);
/** A metric's portable description. */
export type MetricDescription = schema.Infer<typeof MetricDescription>;

/** A collection of metrics grouped by resource and emitting library. */
export const MetricCollectionDescription = defineSchema(
    schema.object({
        resource: ResourceDescription,
        scopes: schema.array(
            schema.object({ scope: ScopeDescription, metrics: schema.array(MetricDescription) }),
        ),
    }),
);
/** A metric collection's portable description. */
export type MetricCollectionDescription = schema.Infer<typeof MetricCollectionDescription>;

/** Describe a collection without retaining SDK resources. */
export function describeMetrics(collection: ResourceMetrics): MetricCollectionDescription {
    return {
        resource: describeResource(collection.resource),
        scopes: collection.scopeMetrics.map((scope) => ({
            scope: describeScope(scope.scope),
            metrics: scope.metrics.map(describeMetric),
        })),
    };
}

/** Describe each metric aggregation with its exact value shape. */
export function describeMetric(metric: MetricData): MetricDescription {
    const base = {
        name: metric.descriptor.name,
        description: metric.descriptor.description,
        unit: metric.descriptor.unit,
        valueType: metric.descriptor.valueType,
        temporality: TEMPORALITY[metric.aggregationTemporality],
    };
    const points = metric.dataPoints.map((point) => ({
        startTime: point.startTime,
        endTime: point.endTime,
        attributes: describeAttributes(point.attributes),
        value: describeNumbers(point.value),
    }));

    // preserve the aggregation discriminant and its corresponding point shape
    switch (metric.dataPointType) {
        case DataPointType.SUM:
            return MetricDescription.parse({
                ...base,
                kind: "sum",
                isMonotonic: metric.isMonotonic,
                points,
            });
        case DataPointType.GAUGE:
            return MetricDescription.parse({ ...base, kind: "gauge", points });
        case DataPointType.HISTOGRAM:
            return MetricDescription.parse({ ...base, kind: "histogram", points });
        case DataPointType.EXPONENTIAL_HISTOGRAM:
            return MetricDescription.parse({ ...base, kind: "exponentialHistogram", points });
    }
}

/** Preserve special numbers in numeric aggregation structures. */
function describeNumbers(value: unknown): unknown {
    if (typeof value === "number") {
        return Number.isFinite(value) ? value : String(value);
    }
    if (Array.isArray(value)) {
        return value.map(describeNumbers);
    }
    if (typeof value === "object" && value !== null) {
        return Object.fromEntries(
            Object.entries(value).map(([key, item]) => [key, describeNumbers(item)]),
        );
    }

    return value;
}
