import type { EventInput, EventKind } from "@destack/event";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    type OtlpEmitter,
    type OtlpSignal,
    VISIT_EVENT,
} from "@destack/telemetry/otlp";
import {
    type AttributeKey,
    type Attributes,
    type AttributeValue,
    type Instrumentation,
    type INSTRUMENTS,
    PERSON_ATTRIBUTE,
    SENSITIVE_PREFIX,
    SPAN_STATUSES,
} from "../event/index.ts";
import type * as kinds from "../event/index.ts";

/** The resource attributes naming an export's emitter, which only the receiver records and no export claims. */
export const RESOURCE_ATTRIBUTES = ["service.instance.id", "destack.build.manifest"] as const;

/** The attribute naming the session a signal belongs to, from the OpenTelemetry semantic conventions. */
const SESSION_ATTRIBUTE = "session.id";

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
const OtlpAttributes = schema.array(OtlpKeyValue).default([]);

/** The resource around a group of items. */
const Resource = schema.looseObject({ attributes: OtlpAttributes }).default({ attributes: [] });

/** The instrumentation scope around a group of items. */
const Scope = schema
    .object({ name: schema.string().default(""), version: schema.string().default("") })
    .default({ name: "", version: "" });

/** A trace identifier, or an empty one for none. */
const OptionalTrace = schema
    .string()
    .regex(/^(?:[0-9a-f]{32})?$/u)
    .exactOptional();

/** A span identifier, or an empty one for none. */
const OptionalSpan = schema
    .string()
    .regex(/^(?:[0-9a-f]{16})?$/u)
    .exactOptional();

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
                            attributes: OtlpAttributes,
                            traceId: OptionalTrace,
                            spanId: OptionalSpan,
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
                            parentSpanId: OptionalSpan,
                            name: schema.string().min(1),
                            kind: schema.number().int().exactOptional(),
                            startTimeUnixNano: Integer,
                            endTimeUnixNano: Integer,
                            attributes: OtlpAttributes,
                            events: schema
                                .array(
                                    schema.looseObject({
                                        timeUnixNano: Integer,
                                        name: schema.string(),
                                        attributes: OtlpAttributes,
                                    }),
                                )
                                .default([]),
                            links: schema
                                .array(
                                    schema.looseObject({
                                        traceId: schema.string().regex(/^[0-9a-f]{32}$/u),
                                        spanId: schema.string().regex(/^[0-9a-f]{16}$/u),
                                    }),
                                )
                                .default([]),
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
    attributes: OtlpAttributes,
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
                                            attributes: OtlpAttributes,
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

/** The OTLP aggregation temporality of deltas, the only one metric events keep. */
const DELTA = 1;

/** An event as its kind appends it. */
type InputOf<Kind> =
    Kind extends EventKind<infer Shape, infer Data> ? EventInput<Shape, Data> : never;

/** The events of one export, by kind, before the receiver groups and appends them. */
export interface Decoded {
    /** The log records. */
    readonly log: InputOf<typeof kinds.log>[];
    /** The spans. */
    readonly span: InputOf<typeof kinds.span>[];
    /** The metric points. */
    readonly metric: InputOf<typeof kinds.metric>[];
    /** The visits. */
    readonly visit: InputOf<typeof kinds.visit>[];
    /** The actions. */
    readonly action: InputOf<typeof kinds.action>[];
}

/** A log record's or a span's attributes split for keeping: the queried ones, the personal ones and their person. */
interface Split {
    /** The attributes kept as a key. */
    readonly attributes: AttributeKey;
    /** The attributes marked personal, absent without a person to seal them under. */
    readonly personal?: Attributes;
    /** The person they acted for. */
    readonly person: string | null;
}

/** Read OTLP/JSON exports as the events of their kinds, under the emitter their receiver verified. */
export const Otlp = {
    /** Read the events of a signal's export request, refusing a resource that names its own emitter. */
    async read(signal: OtlpSignal, body: unknown, emitter: OtlpEmitter): Promise<Decoded> {
        const decoded: Decoded = { log: [], span: [], metric: [], visit: [], action: [] };
        if (signal === "logs") {
            await Otlp.logs(OtlpLogsRequest.parse(body), emitter, decoded);
        } else if (signal === "traces") {
            Otlp.traces(OtlpTracesRequest.parse(body), emitter, decoded);
        } else {
            await Otlp.metrics(OtlpMetricsRequest.parse(body), emitter, decoded);
        }

        return decoded;
    },

    /** Read the log records of a logs request: visits and actions by their reserved names, every other record as a log. */
    async logs(request: OtlpLogsRequest, emitter: OtlpEmitter, decoded: Decoded): Promise<void> {
        for (const group of request.resourceLogs) {
            requireUnclaimed(group.resource.attributes);
            for (const { scope, logRecords } of group.scopeLogs) {
                for (const record of logRecords) {
                    // read a visit or an action by its reserved name, else a log record
                    const read = await recordOf(record, emitter);
                    if (read.name === VISIT_EVENT) {
                        decoded.visit.push(
                            visitOf(read.at, read.id, read.split, read.trace, emitter),
                        );
                    } else if (read.name === ACTION_EVENT) {
                        decoded.action.push(
                            actionOf(read.at, read.id, read.split, read.trace, emitter),
                        );
                    } else {
                        decoded.log.push(logOf(record, read, scope, emitter));
                    }
                }
            }
        }
    },

    /** Read the spans of a traces request with their annotations and links. */
    traces(request: OtlpTracesRequest, emitter: OtlpEmitter, decoded: Decoded): void {
        for (const group of request.resourceSpans) {
            requireUnclaimed(group.resource.attributes);
            for (const { scope, spans } of group.scopeSpans) {
                for (const span of spans) {
                    const start = microseconds(span.startTimeUnixNano);
                    const end = microseconds(span.endTimeUnixNano);
                    const split = splitOf(attributesOf(span.attributes), emitter);
                    decoded.span.push({
                        scope: emitter.scope,
                        id: `${span.traceId}-${span.spanId}`,
                        time: start,
                        keys: {
                            ...stampOf(emitter),
                            name: span.name,
                            trace: span.traceId,
                            parent: presentText(span.parentSpanId) ?? null,
                            status: SPAN_STATUSES[span.status.code],
                            duration: Math.max(0, end - start),
                            issue: null,
                            person: split.person,
                            attributes: split.attributes,
                        },
                        data: {
                            span: span.spanId,
                            ...(span.kind === undefined ? {} : { kind: span.kind }),
                            annotations: span.events.map((event) => ({
                                name: event.name,
                                time: microseconds(event.timeUnixNano),
                                attributes: attributesOf(event.attributes),
                            })),
                            links: span.links.map((link) => ({
                                trace: link.traceId,
                                span: link.spanId,
                            })),
                            ...instanceOf(emitter),
                            instrumentation: scope,
                            ...(split.personal === undefined ? {} : { personal: split.personal }),
                        },
                    });
                }
            }
        }
    },

    /** Read the points of a metrics request: delta sums, gauges and delta exponential histograms. */
    async metrics(
        request: OtlpMetricsRequest,
        emitter: OtlpEmitter,
        decoded: Decoded,
    ): Promise<void> {
        for (const group of request.resourceMetrics) {
            requireUnclaimed(group.resource.attributes);
            for (const { scope, metrics } of group.scopeMetrics) {
                for (const metric of metrics) {
                    decoded.metric.push(...(await pointsOf(metric, scope, emitter)));
                }
            }
        }
    },
};

/** One metric of a metrics request. */
type OtlpMetric =
    OtlpMetricsRequest["resourceMetrics"][number]["scopeMetrics"][number]["metrics"][number];

/** Read a metric's points: a delta sum's or a gauge's values, or a delta exponential histogram's, refusing any other. */
async function pointsOf(
    metric: OtlpMetric,
    scope: Instrumentation,
    emitter: OtlpEmitter,
): Promise<InputOf<typeof kinds.metric>[]> {
    // read sums of deltas and gauges
    const series = metric.sum ?? metric.gauge;
    if (series !== undefined) {
        requireDelta(metric, metric.sum?.aggregationTemporality);

        return Promise.all(
            series.dataPoints.map((each) => {
                // refuse a point without its value
                const value = each.asInt ?? each.asDouble;
                if (value === undefined) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `a point of metric ${metric.name} carries no value`,
                    });
                }

                return metricPoint(metric, scope, emitter, each, each.attributes, {
                    instrument: metric.sum === undefined ? "gauge" : "sum",
                    value: Number(value),
                    count: null,
                });
            }),
        );
    }
    // read delta exponential histograms
    else if (metric.exponentialHistogram !== undefined) {
        requireDelta(metric, metric.exponentialHistogram.aggregationTemporality);

        return Promise.all(
            metric.exponentialHistogram.dataPoints.map((each) =>
                metricPoint(metric, scope, emitter, each, each.attributes, {
                    instrument: "histogram",
                    value: each.sum,
                    count: Number(each.count),
                    histogram: {
                        count: Number(each.count),
                        sum: each.sum,
                        ...(each.min === undefined ? {} : { min: each.min }),
                        ...(each.max === undefined ? {} : { max: each.max }),
                        scale: each.scale,
                        zeroCount: Number(each.zeroCount),
                        positive: bucketsOf(each.positive),
                        negative: bucketsOf(each.negative),
                    },
                }),
            ),
        );
    }
    // refuse explicit-bucket histograms
    else {
        throw new ServiceError("BAD_REQUEST", {
            message: `metric ${metric.name} is no delta sum, gauge or exponential histogram`,
        });
    }
}

/** Write one point of a metric as a metric event, timed by its interval and named by its contents. */
async function metricPoint(
    metric: OtlpMetric,
    scope: Instrumentation,
    emitter: OtlpEmitter,
    interval: {
        readonly startTimeUnixNano?: string | number;
        readonly timeUnixNano: string | number;
    },
    attributes: readonly OtlpKeyValue[],
    measured: {
        readonly instrument: (typeof INSTRUMENTS)[number];
        readonly value: number;
        readonly count: number | null;
        readonly histogram?: InputOf<typeof kinds.metric>["data"]["histogram"];
    },
): Promise<InputOf<typeof kinds.metric>> {
    // time the point by its interval and name it by its contents
    const time = microseconds(interval.timeUnixNano);
    const start = microseconds(interval.startTimeUnixNano ?? interval.timeUnixNano);
    const kept = keyOf(attributesOf(attributes));

    return {
        scope: emitter.scope,
        id: await identityOf(time, [metric.name, emitter.instance, kept, start]),
        time,
        keys: {
            ...stampOf(emitter),
            name: metric.name,
            instrument: measured.instrument,
            value: measured.value,
            count: measured.count,
            attributes: kept,
        },
        data: {
            start,
            ...(metric.unit === "" ? {} : { unit: metric.unit }),
            ...(measured.histogram === undefined ? {} : { histogram: measured.histogram }),
            ...instanceOf(emitter),
            instrumentation: scope,
        },
    };
}

/** One log record of a logs request. */
type OtlpLogRecord =
    OtlpLogsRequest["resourceLogs"][number]["scopeLogs"][number]["logRecords"][number];

/** Read what every log record carries: its event name, time, identity, body text, trace and attributes. */
async function recordOf(record: OtlpLogRecord, emitter: OtlpEmitter) {
    // require the event name and the time
    const time = record.timeUnixNano ?? record.observedTimeUnixNano;
    const name =
        record.eventName === undefined || record.eventName === ""
            ? record.body?.stringValue
            : record.eventName;
    if (name === undefined || name === "") {
        throw new ServiceError("BAD_REQUEST", { message: "a log record names no event" });
    } else if (time === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `log record ${name} carries no time` });
    }

    // read the body as text, the trace and the attributes
    const text = textOf(record.body === undefined ? undefined : valueOf(record.body));
    const at = microseconds(time);
    const trace = presentText(record.traceId);
    const split = splitOf(attributesOf(record.attributes), emitter);

    // name the record by its contents
    const id = await identityOf(at, [name, trace, record.spanId, text, record.attributes]);

    return { name, at, id, text, trace, split };
}

/** Write a log record as a log event. */
function logOf(
    record: OtlpLogRecord,
    read: Awaited<ReturnType<typeof recordOf>>,
    scope: Instrumentation,
    emitter: OtlpEmitter,
): InputOf<typeof kinds.log> {
    const span = presentText(record.spanId);

    return {
        scope: emitter.scope,
        id: read.id,
        time: read.at,
        keys: {
            ...stampOf(emitter),
            name: read.name,
            severity: record.severityNumber ?? 0,
            trace: read.trace ?? null,
            issue: null,
            person: read.split.person,
            attributes: read.split.attributes,
        },
        data: {
            ...(read.text === undefined ? {} : { body: read.text }),
            ...(span === undefined ? {} : { span }),
            ...instanceOf(emitter),
            instrumentation: scope,
            ...(record.observedTimeUnixNano === undefined
                ? {}
                : { observed: microseconds(record.observedTimeUnixNano) }),
            ...(read.split.personal === undefined ? {} : { personal: read.split.personal }),
        },
    };
}

/** Read a record's body as text: a string as it is, any other value as JSON. */
function textOf(body: unknown): string | undefined {
    return body === undefined || typeof body === "string" ? body : JSON.stringify(body);
}

/** Read a visit record as a visit of its route. */
function visitOf(
    time: number,
    id: string,
    split: Split,
    trace: string | undefined,
    emitter: OtlpEmitter,
): InputOf<typeof kinds.visit> {
    // read the visit's text attributes
    const { attributes } = split;
    const text = (key: string) => {
        const value = attributes[key];

        return typeof value === "string" && value !== "" ? value : undefined;
    };
    const path = text(ANALYTICS_ATTRIBUTES.path);
    const title = text(ANALYTICS_ATTRIBUTES.title);
    const referrer = text(ANALYTICS_ATTRIBUTES.referrer);
    const locale = text(ANALYTICS_ATTRIBUTES.locale);

    // refuse a visit naming no route
    const route = text(ANALYTICS_ATTRIBUTES.route);
    if (route === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: "a visit names no route" });
    }

    return {
        scope: emitter.scope,
        id,
        time,
        keys: {
            ...stampOf(emitter),
            route,
            session: text(SESSION_ATTRIBUTE) ?? null,
            trace: trace ?? null,
            visitor: null,
            person: split.person,
        },
        data: {
            ...(path === undefined ? {} : { path }),
            ...(title === undefined ? {} : { title }),
            ...(referrer === undefined ? {} : { referrer }),
            ...(locale === undefined ? {} : { locale }),
            ...(split.personal === undefined ? {} : { personal: split.personal }),
        },
    };
}

/** Read an action record as an action of its name with its properties. */
function actionOf(
    time: number,
    id: string,
    split: Split,
    trace: string | undefined,
    emitter: OtlpEmitter,
): InputOf<typeof kinds.action> {
    const {
        [ANALYTICS_ATTRIBUTES.action]: name,
        [ANALYTICS_ATTRIBUTES.route]: route,
        [SESSION_ATTRIBUTE]: session,
        ...properties
    } = split.attributes;

    return {
        scope: emitter.scope,
        id,
        time,
        keys: {
            ...stampOf(emitter),
            name: typeof name === "string" ? name : "",
            route: typeof route === "string" ? route : null,
            session: typeof session === "string" ? session : null,
            trace: trace ?? null,
            visitor: null,
            person: split.person,
            attributes: properties,
        },
        data: split.personal === undefined ? {} : { personal: split.personal },
    };
}

/** Split attributes into the queried ones and the personal ones, dropping personal ones without a person to seal them under. */
function splitOf(attributes: Attributes, emitter: OtlpEmitter): Split {
    // read the person the emitter verified, else the one the attributes name
    const named = attributes[PERSON_ATTRIBUTE];
    const person = emitter.person ?? (typeof named === "string" && named !== "" ? named : null);

    // keep the personal attributes apart, and only for a person
    const personal = Object.fromEntries(
        Object.entries(attributes).flatMap(([key, value]) =>
            key.startsWith(SENSITIVE_PREFIX) ? [[key.slice(SENSITIVE_PREFIX.length), value]] : [],
        ),
    );
    const kept = Object.fromEntries(
        Object.entries(attributes).filter(
            ([key]) => !key.startsWith(SENSITIVE_PREFIX) && key !== PERSON_ATTRIBUTE,
        ),
    );

    return {
        attributes: keyOf(kept),
        ...(person === null || Object.keys(personal).length === 0 ? {} : { personal }),
        person,
    };
}

/** Write attributes as a key: each list as JSON. */
function keyOf(attributes: Attributes): AttributeKey {
    return Object.fromEntries(
        Object.entries(attributes).map(([key, value]) => [
            key,
            Array.isArray(value) ? JSON.stringify(value) : value,
        ]),
    );
}

/** Refuse a resource naming its own instance or build, which only the receiver records. */
function requireUnclaimed(resource: readonly OtlpKeyValue[]): void {
    const claimed = RESOURCE_ATTRIBUTES.filter((key) =>
        resource.some((attribute) => attribute.key === key),
    );
    if (claimed.length > 0) {
        throw new ServiceError("FORBIDDEN", {
            message: `only the receiver records an export's emitter, found ${claimed.join(", ")}`,
        });
    }
}

/** Read the installation and build the receiver verified, as keys. */
function stampOf(emitter: OtlpEmitter): {
    readonly installation: string | null;
    readonly build: string | null;
} {
    return { installation: emitter.installation ?? null, build: emitter.build ?? null };
}

/** Read the instance the receiver verified, as data. */
function instanceOf(emitter: OtlpEmitter): { readonly instance?: string } {
    return emitter.instance === undefined ? {} : { instance: emitter.instance };
}

/** Name an event without an identity of its own by its time and a digest of its contents, so a repeated export keeps it once. */
async function identityOf(time: number, contents: unknown): Promise<string> {
    const bytes = new TextEncoder().encode(JSON.stringify(contents));
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();

    return `${time.toString(16).padStart(14, "0")}-${digest.slice(0, 16)}`;
}

/** Refuse a cumulative metric, whose points cannot be added up: a gauge names no temporality. */
function requireDelta(metric: OtlpMetric, temporality: number | undefined): void {
    if (temporality !== undefined && temporality !== DELTA) {
        throw new ServiceError("BAD_REQUEST", {
            message: `metric ${metric.name} is cumulative: export deltas`,
        });
    }
}

/** Read one side of an exponential histogram's buckets. */
function bucketsOf(side: {
    readonly offset: number;
    readonly bucketCounts: readonly (string | number)[];
}) {
    return { offset: side.offset, counts: side.bucketCounts.map(Number) };
}

/** Convert OTLP nanoseconds to microseconds. */
function microseconds(nanoseconds: string | number): number {
    return Number(BigInt(nanoseconds) / 1000n);
}

/** Read an identifier OTLP may leave empty, absent when empty. */
function presentText(value: string | undefined): string | undefined {
    return value === undefined || value === "" ? undefined : value;
}

/** Read OTLP attributes as attribute values, writing nested lists as JSON. */
function attributesOf(list: readonly OtlpKeyValue[]): Attributes {
    // keep the attributes that carry a value, as an empty value carries none
    return Object.fromEntries(
        list.flatMap(({ key, value }) => {
            const read = valueOf(value ?? {});

            return read === undefined ? [] : [[key, read]];
        }),
    );
}

/** Read one OTLP attribute value. */
function valueOf(value: OtlpValue): AttributeValue | undefined {
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
            const read = valueOf(item);

            return Array.isArray(read) ? [JSON.stringify(read)] : read === undefined ? [] : [read];
        });
    } else if (value.kvlistValue !== undefined) {
        return JSON.stringify(attributesOf(value.kvlistValue.values ?? []));
    } else if (value.bytesValue !== undefined) {
        return value.bytesValue;
    }
    // read an empty value as none
    else {
        return undefined;
    }
}
