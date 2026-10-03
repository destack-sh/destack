import { type Identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import type { AttributeValue, Entry } from "./entry.ts";

/** The resource attribute naming the workload instance, from the OpenTelemetry semantic conventions. */
const INSTANCE_ATTRIBUTE = "service.instance.id";

/** A 64-bit integer as OTLP/JSON writes it: a decimal string, or a number when small. */
const Integer = schema.union([schema.string().regex(/^-?\d+$/u), schema.number().int()]);

/** An OTLP attribute value. */
type OtlpValue = {
    readonly stringValue?: string;
    readonly boolValue?: boolean;
    readonly intValue?: string | number;
    readonly doubleValue?: number;
    readonly bytesValue?: string;
    readonly arrayValue?: { readonly values?: readonly OtlpValue[] };
    readonly kvlistValue?: { readonly values?: readonly OtlpKeyValue[] };
};

/** An OTLP attribute. */
type OtlpKeyValue = { readonly key: string; readonly value?: OtlpValue };

/** An OTLP attribute value, recursive through arrays and key-value lists. */
const OtlpValue: schema.Schema<OtlpValue> = schema.lazy(() =>
    schema.looseObject({
        stringValue: schema.string().exactOptional(),
        boolValue: schema.boolean().exactOptional(),
        intValue: Integer.exactOptional(),
        doubleValue: schema.number().exactOptional(),
        bytesValue: schema.string().exactOptional(),
        arrayValue: schema
            .looseObject({ values: schema.array(OtlpValue).exactOptional() })
            .exactOptional(),
        kvlistValue: schema
            .looseObject({ values: schema.array(OtlpKeyValue).exactOptional() })
            .exactOptional(),
    }),
);

/** An OTLP attribute. */
const OtlpKeyValue: schema.Schema<OtlpKeyValue> = schema.lazy(() =>
    schema.looseObject({ key: schema.string(), value: OtlpValue.exactOptional() }),
);

/** The attributes of a resource or an item. */
const Attributes = schema.array(OtlpKeyValue).default([]);

/** The resource and instrumentation scope around a group of items. */
const Resource = schema.looseObject({ attributes: Attributes }).default({ attributes: [] });

/** The instrumentation scope around a group of items. */
const Scope = schema
    .object({ name: schema.string().default(""), version: schema.string().default("") })
    .default({ name: "", version: "" });

/** An OTLP/JSON logs export request. */
export const OtlpLogsRequest = schema.looseObject({
    resourceLogs: schema.array(
        schema.looseObject({
            resource: Resource,
            scopeLogs: schema.array(
                schema.looseObject({
                    scope: Scope,
                    logRecords: schema.array(
                        schema.looseObject({
                            timeUnixNano: Integer.exactOptional(),
                            observedTimeUnixNano: Integer.exactOptional(),
                            severityNumber: schema.number().int().min(0).max(24).exactOptional(),
                            eventName: schema.string().exactOptional(),
                            body: OtlpValue.exactOptional(),
                            attributes: Attributes,
                            traceId: schema
                                .string()
                                .regex(/^(?:[0-9a-f]{32})?$/u)
                                .exactOptional(),
                            spanId: schema
                                .string()
                                .regex(/^(?:[0-9a-f]{16})?$/u)
                                .exactOptional(),
                        }),
                    ),
                }),
            ),
        }),
    ),
});
/** An OTLP/JSON logs export request. */
export type OtlpLogsRequest = schema.Infer<typeof OtlpLogsRequest>;

/** An OTLP/JSON traces export request. */
export const OtlpTracesRequest = schema.looseObject({
    resourceSpans: schema.array(
        schema.looseObject({
            resource: Resource,
            scopeSpans: schema.array(
                schema.looseObject({
                    scope: Scope,
                    spans: schema.array(
                        schema.looseObject({
                            traceId: schema.string().regex(/^[0-9a-f]{32}$/u),
                            spanId: schema.string().regex(/^[0-9a-f]{16}$/u),
                            parentSpanId: schema
                                .string()
                                .regex(/^(?:[0-9a-f]{16})?$/u)
                                .exactOptional(),
                            name: schema.string().min(1),
                            startTimeUnixNano: Integer,
                            endTimeUnixNano: Integer,
                            attributes: Attributes,
                            status: schema
                                .looseObject({
                                    code: schema.literal([0, 1, 2]).default(0),
                                })
                                .default({ code: 0 }),
                        }),
                    ),
                }),
            ),
        }),
    ),
});
/** An OTLP/JSON traces export request. */
export type OtlpTracesRequest = schema.Infer<typeof OtlpTracesRequest>;

/** A number data point of a sum or a gauge. */
const NumberPoint = schema.looseObject({
    attributes: Attributes,
    startTimeUnixNano: Integer.exactOptional(),
    timeUnixNano: Integer,
    asInt: Integer.exactOptional(),
    asDouble: schema.number().exactOptional(),
});

/** The buckets of an exponential histogram data point on one side of zero. */
const OtlpBuckets = schema
    .looseObject({
        offset: schema.number().int().default(0),
        bucketCounts: schema.array(Integer).default([]),
    })
    .default({ offset: 0, bucketCounts: [] });

/** An OTLP/JSON metrics export request, of delta sums, gauges and exponential histograms. */
export const OtlpMetricsRequest = schema.looseObject({
    resourceMetrics: schema.array(
        schema.looseObject({
            resource: Resource,
            scopeMetrics: schema.array(
                schema.looseObject({
                    scope: Scope,
                    metrics: schema.array(
                        schema.looseObject({
                            name: schema.string().min(1),
                            unit: schema.string().default(""),
                            sum: schema
                                .looseObject({
                                    dataPoints: schema.array(NumberPoint),
                                    aggregationTemporality: schema.number().int(),
                                })
                                .exactOptional(),
                            gauge: schema
                                .looseObject({ dataPoints: schema.array(NumberPoint) })
                                .exactOptional(),
                            histogram: schema.looseObject({}).exactOptional(),
                            exponentialHistogram: schema
                                .looseObject({
                                    aggregationTemporality: schema.number().int(),
                                    dataPoints: schema.array(
                                        schema.looseObject({
                                            attributes: Attributes,
                                            startTimeUnixNano: Integer.exactOptional(),
                                            timeUnixNano: Integer,
                                            count: Integer,
                                            sum: schema.number().default(0),
                                            min: schema.number().exactOptional(),
                                            max: schema.number().exactOptional(),
                                            scale: schema.number().int(),
                                            zeroCount: Integer.default(0),
                                            positive: OtlpBuckets,
                                            negative: OtlpBuckets,
                                        }),
                                    ),
                                })
                                .exactOptional(),
                        }),
                    ),
                }),
            ),
        }),
    ),
});
/** An OTLP/JSON metrics export request. */
export type OtlpMetricsRequest = schema.Infer<typeof OtlpMetricsRequest>;

/** The OTLP aggregation temporality of deltas, the only one points store. */
const DELTA = 1;

/** The span outcomes by OTLP status code. */
const STATUSES = ["unset", "ok", "error"] as const;

/** Read the entries of OTLP/JSON export requests an installation or a host sent. */
export const Otlp = {
    /** Read the log records of a logs request with their event name or else body as name. */
    logs(request: OtlpLogsRequest, installation?: Identifier<"installation">): Entry[] {
        return request.resourceLogs.flatMap((group) => {
            const instance = instanceOf(group.resource.attributes);

            return group.scopeLogs.flatMap(({ scope, logRecords }) =>
                logRecords.map((record): Entry => {
                    // label the record by its event, or by a string body, and keep the body as text
                    const body =
                        record.body === undefined ? undefined : attributeValue(record.body);
                    const text =
                        body === undefined
                            ? undefined
                            : typeof body === "string"
                              ? body
                              : JSON.stringify(body);
                    const name =
                        record.eventName === undefined || record.eventName === ""
                            ? record.body?.stringValue
                            : record.eventName;
                    if (name === undefined || name === "") {
                        throw new ServiceError("BAD_REQUEST", {
                            message: "a log record names no event",
                        });
                    }
                    const time = record.timeUnixNano ?? record.observedTimeUnixNano;
                    if (time === undefined) {
                        throw new ServiceError("BAD_REQUEST", {
                            message: `log record ${name} carries no time`,
                        });
                    }

                    return {
                        kind: "log",
                        name,
                        time: microseconds(time),
                        ...(installation === undefined ? {} : { installation }),
                        ...(instance === undefined ? {} : { instance }),
                        source: scope,
                        ...(record.traceId === undefined || record.traceId === ""
                            ? {}
                            : { trace: record.traceId }),
                        ...(record.spanId === undefined || record.spanId === ""
                            ? {}
                            : { span: record.spanId }),
                        status: "unset",
                        ...(record.severityNumber === undefined || record.severityNumber === 0
                            ? {}
                            : { severity: record.severityNumber }),
                        ...(text === undefined ? {} : { body: text }),
                        attributes: attributes(record.attributes),
                    };
                }),
            );
        });
    },

    /** Read the spans of a traces request. */
    traces(request: OtlpTracesRequest, installation?: Identifier<"installation">): Entry[] {
        return request.resourceSpans.flatMap((group) => {
            const instance = instanceOf(group.resource.attributes);

            return group.scopeSpans.flatMap(({ scope, spans }) =>
                spans.map((span): Entry => {
                    const start = microseconds(span.startTimeUnixNano);
                    const end = microseconds(span.endTimeUnixNano);

                    return {
                        kind: "span",
                        name: span.name,
                        time: start,
                        duration: Math.max(0, end - start),
                        ...(installation === undefined ? {} : { installation }),
                        ...(instance === undefined ? {} : { instance }),
                        source: scope,
                        trace: span.traceId,
                        span: span.spanId,
                        ...(span.parentSpanId === undefined || span.parentSpanId === ""
                            ? {}
                            : { parent: span.parentSpanId }),
                        status: STATUSES[span.status.code],
                        attributes: attributes(span.attributes),
                    };
                }),
            );
        });
    },

    /** Read the points of a metrics request: delta sums, gauges and delta exponential histograms. */
    metrics(request: OtlpMetricsRequest, installation?: Identifier<"installation">): Entry[] {
        return request.resourceMetrics.flatMap((group) => {
            const instance = instanceOf(group.resource.attributes);

            return group.scopeMetrics.flatMap(({ scope, metrics }) =>
                metrics.flatMap((metric): Entry[] => {
                    // stamp every point of the metric alike
                    const base = {
                        kind: "point" as const,
                        name: metric.name,
                        ...(installation === undefined ? {} : { installation }),
                        ...(instance === undefined ? {} : { instance }),
                        source: scope,
                        status: "unset" as const,
                        ...(metric.unit === "" ? {} : { unit: metric.unit }),
                    };
                    const interval = (point: {
                        startTimeUnixNano?: string | number | undefined;
                        timeUnixNano: string | number;
                    }) => {
                        const end = microseconds(point.timeUnixNano);
                        const start = microseconds(point.startTimeUnixNano ?? point.timeUnixNano);

                        return { time: start, duration: Math.max(0, end - start) };
                    };

                    // read sums of deltas and gauges
                    const series = metric.sum ?? metric.gauge;
                    if (series !== undefined) {
                        if (
                            metric.sum !== undefined &&
                            metric.sum.aggregationTemporality !== DELTA
                        ) {
                            throw new ServiceError("BAD_REQUEST", {
                                message: `metric ${metric.name} is cumulative: export deltas`,
                            });
                        }
                        return series.dataPoints.map((point) => {
                            // refuse a point without its value
                            const value = point.asInt ?? point.asDouble;
                            if (value === undefined) {
                                throw new ServiceError("BAD_REQUEST", {
                                    message: `a point of metric ${metric.name} carries no value`,
                                });
                            }

                            return {
                                ...base,
                                ...interval(point),
                                metric:
                                    metric.sum === undefined
                                        ? ("gauge" as const)
                                        : ("sum" as const),
                                value: Number(value),
                                attributes: attributes(point.attributes),
                            };
                        });
                    }
                    // read delta exponential histograms
                    else if (metric.exponentialHistogram !== undefined) {
                        if (metric.exponentialHistogram.aggregationTemporality !== DELTA) {
                            throw new ServiceError("BAD_REQUEST", {
                                message: `metric ${metric.name} is cumulative: export deltas`,
                            });
                        }

                        return metric.exponentialHistogram.dataPoints.map((point) => ({
                            ...base,
                            ...interval(point),
                            metric: "histogram" as const,
                            histogram: {
                                count: Number(point.count),
                                sum: point.sum,
                                ...(point.min === undefined ? {} : { min: point.min }),
                                ...(point.max === undefined ? {} : { max: point.max }),
                                scale: point.scale,
                                zeroCount: Number(point.zeroCount),
                                positive: buckets(point.positive),
                                negative: buckets(point.negative),
                            },
                            attributes: attributes(point.attributes),
                        }));
                    }
                    // refuse explicit-bucket histograms
                    else {
                        throw new ServiceError("BAD_REQUEST", {
                            message: `metric ${metric.name} is no delta sum, gauge or exponential histogram`,
                        });
                    }
                }),
            );
        });
    },
};

/** Read one side of an exponential histogram's buckets. */
function buckets(side: {
    readonly offset: number;
    readonly bucketCounts: readonly (string | number)[];
}) {
    return { offset: side.offset, counts: side.bucketCounts.map(Number) };
}

/** Read the instance a resource names. */
function instanceOf(resource: readonly OtlpKeyValue[]): string | undefined {
    const value = resource.find((attribute) => attribute.key === INSTANCE_ATTRIBUTE)?.value;

    return value?.stringValue;
}

/** Convert OTLP nanoseconds to microseconds. */
function microseconds(nanoseconds: string | number): number {
    return Number(BigInt(nanoseconds) / 1000n);
}

/** Read OTLP attributes as attribute values, writing nested lists as JSON. */
function attributes(list: readonly OtlpKeyValue[]): Record<string, AttributeValue> {
    // keep the attributes that carry a value, as an empty value carries none
    return Object.fromEntries(
        list.flatMap(({ key, value }) => {
            const read = attributeValue(value ?? {});

            return read === undefined ? [] : [[key, read]];
        }),
    );
}

/** Read one OTLP attribute value. */
function attributeValue(value: OtlpValue): AttributeValue | undefined {
    // read primitives
    if (value.stringValue !== undefined) {
        return value.stringValue;
    } else if (value.boolValue !== undefined) {
        return value.boolValue;
    } else if (value.intValue !== undefined) {
        return Number(value.intValue);
    } else if (value.doubleValue !== undefined) {
        return value.doubleValue;
    }
    // read lists of primitives, and write anything nested as JSON
    else if (value.arrayValue !== undefined) {
        return (value.arrayValue.values ?? []).flatMap((item) => {
            const read = attributeValue(item);

            return Array.isArray(read) ? [JSON.stringify(read)] : read === undefined ? [] : [read];
        });
    } else if (value.kvlistValue !== undefined) {
        return JSON.stringify(attributes(value.kvlistValue.values ?? []));
    } else if (value.bytesValue !== undefined) {
        return value.bytesValue;
    }
    // read an empty value as none
    else {
        return undefined;
    }
}
