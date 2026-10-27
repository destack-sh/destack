import { Event } from "@destack/event";
import type { Select } from "@destack/db";
import type { ObjectType } from "@destack/object";
import { Digest, type DigestCrypto, type Identifier, schema } from "@destack/schema";
import { ObservabilityError } from "../error/error.ts";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    type OtlpEmitter,
    type OtlpSignal,
    SESSION_ATTRIBUTE,
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    SESSION_STATUSES,
    VISIT_EVENT,
    FRAMES_ATTRIBUTE,
} from "@destack/telemetry/otlp";
import { StackFrame } from "@destack/package/stack";
import {
    type AttributeKey,
    type Attributes,
    type AttributeValue,
    type Instrumentation,
    PERSON_ATTRIBUTE,
    SENSITIVE_PREFIX,
    textOf,
} from "../event/index.ts";
import { type INSTRUMENTS, SPAN_STATUSES } from "../object/index.ts";
import type * as kinds from "../object/index.ts";

/** The resource attributes the forwarding machine sets on each export, by emitter field. */
export const EMITTER_ATTRIBUTES = {
    installation: "destack.installation.id",
    instance: "service.instance.id",
    build: "destack.build.manifest",
    person: "enduser.id",
    address: "client.address",
    userAgent: "user_agent.original",
} as const;

/** The field of an export request keeping its resources' groups, by signal. */
const RESOURCE_GROUPS = {
    logs: "resourceLogs",
    traces: "resourceSpans",
    metrics: "resourceMetrics",
} as const;

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
/** A record's attributes. */
type OtlpAttributes = schema.Infer<typeof OtlpAttributes>;

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

/** An OTLP attribute keeping an exception's symbolicated frames as JSON text. */
const FramesEntry = schema.looseObject({
    key: schema.literal(FRAMES_ATTRIBUTE),
    value: schema.looseObject({ stringValue: schema.string() }),
});

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

/** An event of a type as its scope appends it: its row without the scope, with its data. */
export type Item<Type extends ObjectType> = Omit<Select<Type["table"]>, "scope" | "data"> & {
    readonly data: NonNullable<Select<Type["table"]>["data"]>;
};

/** The events of one resource, by type. */
export interface OtlpEvents {
    /** The log records. */
    readonly log: Item<typeof kinds.log>[];
    /** The spans. */
    readonly span: Item<typeof kinds.span>[];
    /** The metric points. */
    readonly metric: Item<typeof kinds.metric>[];
    /** The visits. */
    readonly visit: Item<typeof kinds.visit>[];
    /** The actions. */
    readonly action: Item<typeof kinds.action>[];
    /** The sessions ended. */
    readonly session: Item<typeof kinds.session>[];
}

/** A log record's or a span's attributes split for keeping. */
interface AttributeSplit {
    /** The attributes kept as a key. */
    readonly attributes: AttributeKey;
    /** The attributes marked personal, absent without a person to seal them under. */
    readonly personal?: Attributes;
    /** The person they acted for. */
    readonly person: string | null;
    /** The session they were recorded in. */
    readonly session: string | null;
}

/** One log record of a logs request. */
type OtlpLogRecord =
    OtlpLogsRequest["resourceLogs"][number]["scopeLogs"][number]["logRecords"][number];

/** One metric of a metrics request. */
type OtlpMetric =
    OtlpMetricsRequest["resourceMetrics"][number]["scopeMetrics"][number]["metrics"][number];

/** What every log record has: its event name, time, identity, body, trace and attributes. */
interface OtlpRecord {
    /** The event name. */
    readonly name: string;
    /** When it happened, in Unix microseconds. */
    readonly at: number;
    /** The identity its contents derive. */
    readonly id: string;
    /** The body as text. */
    readonly text: string | undefined;
    /** The trace. */
    readonly trace: string | undefined;
    /** The attributes split for keeping. */
    readonly split: AttributeSplit;
}

/** An OTLP/JSON export request of one signal. */
export class OtlpExport {
    /** The signal. */
    readonly signal: OtlpSignal;
    /** The request body. */
    readonly body: unknown;

    /** Take an export request of a signal. */
    constructor(signal: OtlpSignal, body: unknown) {
        this.signal = signal;
        this.body = body;
    }

    /** Set the emitter a forwarding machine knows on each resource, refusing a claimed emitter. */
    stamp(emitter: Readonly<Partial<Record<keyof typeof EMITTER_ATTRIBUTES, string>>>): OtlpExport {
        // refuse resources claiming the forwarder's emitter attribute
        const groups = RESOURCE_GROUPS[this.signal];
        const request = schema
            .looseObject({ [groups]: schema.array(schema.looseObject({ resource: Resource })) })
            .parse(this.body);
        const resources = request[groups] ?? [];
        const reserved = new Set<string>(Object.values(EMITTER_ATTRIBUTES));
        const claimed = new Set(
            resources.flatMap((group) =>
                group.resource.attributes
                    .map((attribute) => attribute.key)
                    .filter((key) => reserved.has(key)),
            ),
        );
        if (claimed.size > 0) {
            throw ObservabilityError.of(
                "EMITTER_ATTRIBUTE_RESERVED",
                `only the receiver can record an export's emitter, found ${[...claimed].join(", ")}`,
            );
        }

        // append the known attributes as text to each resource's own
        const known = Object.entries(EMITTER_ATTRIBUTES).flatMap(([name, key]) => {
            const value: unknown = Reflect.get(emitter, name);

            return typeof value === "string" ? [{ key, value: { stringValue: value } }] : [];
        });
        const stamped = resources.map((group) => ({
            ...group,
            resource: { ...group.resource, attributes: [...group.resource.attributes, ...known] },
        }));

        return new OtlpExport(this.signal, { ...request, [groups]: stamped });
    }

    /** Attribute the frames of the exceptions the export's logs and span events keep. */
    async attribute(
        attribute: (frames: readonly StackFrame[]) => Promise<StackFrame[]>,
    ): Promise<OtlpExport> {
        // rewrite the frames of each log record
        if (this.signal === "logs") {
            const request = OtlpLogsRequest.parse(this.body);
            for (const group of request.resourceLogs) {
                for (const scoped of group.scopeLogs) {
                    for (const record of scoped.logRecords) {
                        record.attributes = await attributeFrames(record.attributes, attribute);
                    }
                }
            }

            return new OtlpExport(this.signal, request);
        }
        // rewrite the frames of each span's events
        else if (this.signal === "traces") {
            const request = OtlpTracesRequest.parse(this.body);
            for (const group of request.resourceSpans) {
                for (const scoped of group.scopeSpans) {
                    for (const event of scoped.spans.flatMap((span) => span.events)) {
                        event.attributes = await attributeFrames(event.attributes, attribute);
                    }
                }
            }

            return new OtlpExport(this.signal, request);
        }
        // keep metrics, which record no exceptions
        else {
            return this;
        }
    }

    /** Decode each resource's records in a scope, under the emitter its attributes name. */
    async decode(scope: string, crypto: DigestCrypto): Promise<OtlpResource[]> {
        // decode the log records of each resource
        if (this.signal === "logs") {
            return Promise.all(
                OtlpLogsRequest.parse(this.body).resourceLogs.map(async (group) => {
                    const decoder = new OtlpDecoder(scope, group.resource.attributes, crypto);

                    return new OtlpResource(decoder.emitter, await decoder.logs(group.scopeLogs));
                }),
            );
        }
        // decode the spans of each resource
        else if (this.signal === "traces") {
            return Promise.all(
                OtlpTracesRequest.parse(this.body).resourceSpans.map(async (group) => {
                    const decoder = new OtlpDecoder(scope, group.resource.attributes, crypto);

                    return new OtlpResource(decoder.emitter, await decoder.spans(group.scopeSpans));
                }),
            );
        }
        // decode the metric points of each resource
        else {
            return Promise.all(
                OtlpMetricsRequest.parse(this.body).resourceMetrics.map(async (group) => {
                    const decoder = new OtlpDecoder(scope, group.resource.attributes, crypto);
                    const events = await decoder.metrics(group.scopeMetrics);

                    return new OtlpResource(decoder.emitter, events);
                }),
            );
        }
    }
}

/** One resource of an export: its emitter, as its forwarder named it, and its events by type. */
export class OtlpResource {
    /** The emitter. */
    readonly emitter: OtlpEmitter;
    /** The events, by type, before their issues and visitor are known. */
    readonly events: OtlpEvents;

    /** Take a resource's emitter and its decoded events. */
    constructor(emitter: OtlpEmitter, events: OtlpEvents) {
        this.emitter = emitter;
        this.events = events;
    }

    /** Build the items to append: events with their issues, visits with the visitor's hash. */
    items(known: {
        readonly issues: ReadonlyMap<string, string>;
        readonly visitor?: string;
    }): OtlpEvents {
        // set each log record's and span's issue
        const { issues, visitor } = known;
        const log = this.events.log.map((event, index) => ({
            ...event,
            issue: issues.get(`log:${index}`) ?? null,
        }));
        const span = this.events.span.map((event, index) => ({
            ...event,
            issue: issues.get(`span:${index}`) ?? null,
        }));

        // set the visitor's hash on each visit, action and session
        const hashed = <Visited>(events: readonly Visited[]) =>
            visitor === undefined ? [...events] : events.map((event) => ({ ...event, visitor }));

        return {
            log,
            span,
            metric: this.events.metric,
            visit: hashed(this.events.visit),
            action: hashed(this.events.action),
            session: hashed(this.events.session),
        };
    }
}

/** Decodes one resource's records under the emitter its forwarding machine named. */
class OtlpDecoder {
    /** The emitter. */
    readonly emitter: OtlpEmitter;
    /** The WebCrypto hashing event identities. */
    readonly #crypto: DigestCrypto;

    /** Read a resource's emitter in a scope from the attributes its forwarding machine set. */
    constructor(scope: string, resource: readonly OtlpKeyValue[], crypto: DigestCrypto) {
        // hash event identities with the WebCrypto
        this.#crypto = crypto;

        // read the forwarder's attributes as text
        const attributes = attributesOf(resource);
        const text = (key: string) => textOf(attributes[key]);
        const installation = text(EMITTER_ATTRIBUTES.installation);
        const instance = text(EMITTER_ATTRIBUTES.instance);
        const build = text(EMITTER_ATTRIBUTES.build);
        const person = text(EMITTER_ATTRIBUTES.person);
        const address = text(EMITTER_ATTRIBUTES.address);
        const userAgent = text(EMITTER_ATTRIBUTES.userAgent);

        // name the emitter
        this.emitter = {
            scope,
            ...(installation === undefined
                ? {}
                : { installation: schema.identifier("installation").parse(installation) }),
            ...(instance === undefined
                ? {}
                : { instance: schema.identifier("instance").parse(instance) }),
            ...(build === undefined ? {} : { build: Digest.parse(build) }),
            ...(person === undefined ? {} : { person: schema.identifier("user").parse(person) }),
            ...(address === undefined || userAgent === undefined
                ? {}
                : { visitor: { address, userAgent } }),
        };
    }

    /** Decode the resource's log records, visits, actions and session ends. */
    async logs(groups: OtlpLogsRequest["resourceLogs"][number]["scopeLogs"]): Promise<OtlpEvents> {
        const events = OtlpDecoder.#events();
        for (const { scope, logRecords } of groups) {
            for (const record of logRecords) {
                // read a visit, an action or a session's end by its reserved name, else a log record
                const read = await this.#record(record);
                if (read.name === VISIT_EVENT) {
                    events.visit.push(this.#visit(read));
                } else if (read.name === ACTION_EVENT) {
                    events.action.push(this.#action(read));
                } else if (read.name === SESSION_EVENTS.end) {
                    events.session.push(this.#session(read));
                } else {
                    events.log.push(this.#log(record, read, scope));
                }
            }
        }

        return events;
    }

    /** Decode the resource's spans with their annotations and links. */
    async spans(
        groups: OtlpTracesRequest["resourceSpans"][number]["scopeSpans"],
    ): Promise<OtlpEvents> {
        const events = OtlpDecoder.#events();
        for (const { scope, spans } of groups) {
            for (const span of spans) {
                // time the span and split its attributes
                const start = microseconds(span.startTimeUnixNano);
                const end = microseconds(span.endTimeUnixNano);
                if (end < start) {
                    throw ObservabilityError.of(
                        "SPAN_TIMES_INVALID",
                        `span ${span.spanId} ends before it starts`,
                    );
                }
                const split = this.#split(attributesOf(span.attributes));

                // add it with its annotations and links
                events.span.push({
                    id: await this.#identity([span.traceId, span.spanId]),
                    time: start,
                    ...this.#stamp(),
                    name: span.name,
                    trace: span.traceId,
                    parent: presentText(span.parentSpanId) ?? null,
                    status: SPAN_STATUSES[span.status.code],
                    duration: end - start,
                    issue: null,
                    session: split.session,
                    person: split.person,
                    attributes: split.attributes,
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
                        ...this.#instance(),
                        instrumentation: scope,
                        ...(split.personal === undefined ? {} : { personal: split.personal }),
                    },
                });
            }
        }

        return events;
    }

    /** Decode the points of the resource's metrics. */
    async metrics(
        groups: OtlpMetricsRequest["resourceMetrics"][number]["scopeMetrics"],
    ): Promise<OtlpEvents> {
        const events = OtlpDecoder.#events();
        for (const { scope, metrics } of groups) {
            for (const metric of metrics) {
                events.metric.push(...(await this.#points(metric, scope)));
            }
        }

        return events;
    }

    /** Build an empty list of each type's events. */
    static #events(): OtlpEvents {
        return { log: [], span: [], metric: [], visit: [], action: [], session: [] };
    }

    /** Read a metric's points. */
    async #points(
        metric: OtlpMetric,
        scope: Instrumentation,
    ): Promise<Item<typeof kinds.metric>[]> {
        // read sums of deltas and gauges
        const series = metric.sum ?? metric.gauge;
        if (series !== undefined) {
            requireDelta(metric, metric.sum?.aggregationTemporality);

            return Promise.all(
                series.dataPoints.map((each) => {
                    // refuse a point without its value
                    const value = each.asInt ?? each.asDouble;
                    if (value === undefined) {
                        throw ObservabilityError.of(
                            "POINT_VALUE_MISSING",
                            `metric ${metric.name} has a point with no value`,
                        );
                    }

                    return this.#point(metric, scope, each, {
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
                    this.#point(metric, scope, each, {
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
            throw ObservabilityError.of(
                "METRIC_UNSUPPORTED",
                `metric ${metric.name} is not a delta sum, gauge or exponential histogram`,
            );
        }
    }

    /** Read one point of a metric as a metric event. */
    async #point(
        metric: OtlpMetric,
        scope: Instrumentation,
        point: {
            readonly attributes: readonly OtlpKeyValue[];
            readonly startTimeUnixNano?: string | number;
            readonly timeUnixNano: string | number;
        },
        measured: {
            readonly instrument: (typeof INSTRUMENTS)[number];
            readonly value: number;
            readonly count: number | null;
            readonly histogram?: Item<typeof kinds.metric>["data"]["histogram"];
        },
    ): Promise<Item<typeof kinds.metric>> {
        // time the point by its interval and identify it by its contents
        const time = microseconds(point.timeUnixNano);
        const start = microseconds(point.startTimeUnixNano ?? point.timeUnixNano);
        const kept = keyOf(attributesOf(point.attributes));
        const id = await this.#identity([time, metric.name, this.emitter.instance, kept, start]);

        return {
            id,
            time,
            ...this.#stamp(),
            name: metric.name,
            instrument: measured.instrument,
            value: measured.value,
            count: measured.count,
            attributes: kept,
            data: {
                start,
                ...(metric.unit === "" ? {} : { unit: metric.unit }),
                ...(measured.histogram === undefined ? {} : { histogram: measured.histogram }),
                ...this.#instance(),
                instrumentation: scope,
            },
        };
    }

    /** Read what every log record has. */
    async #record(record: OtlpLogRecord): Promise<OtlpRecord> {
        // require the event name and the time
        const time = record.timeUnixNano ?? record.observedTimeUnixNano;
        const name =
            record.eventName === undefined || record.eventName === ""
                ? record.body?.stringValue
                : record.eventName;
        if (name === undefined || name === "") {
            throw ObservabilityError.of("LOG_EVENT_MISSING", "the log record has no event name");
        } else if (time === undefined) {
            throw ObservabilityError.of("LOG_TIME_MISSING", `log record ${name} has no time`);
        }

        // read the body as text, the trace and the attributes
        const body = record.body === undefined ? undefined : valueOf(record.body);
        const text = body === undefined || typeof body === "string" ? body : JSON.stringify(body);
        const at = microseconds(time);
        const trace = presentText(record.traceId);
        const split = this.#split(attributesOf(record.attributes));

        // identify the record by its contents
        const id = await this.#identity([at, name, trace, record.spanId, text, record.attributes]);

        return { name, at, id, text, trace, split };
    }

    /** Read a log record as a log event. */
    #log(record: OtlpLogRecord, read: OtlpRecord, scope: Instrumentation): Item<typeof kinds.log> {
        const span = presentText(record.spanId);

        return {
            id: read.id,
            time: read.at,
            ...this.#stamp(),
            name: read.name,
            severity: record.severityNumber ?? 0,
            trace: read.trace ?? null,
            issue: null,
            session: read.split.session,
            person: read.split.person,
            attributes: read.split.attributes,
            data: {
                ...(read.text === undefined ? {} : { body: read.text }),
                ...(span === undefined ? {} : { span }),
                ...this.#instance(),
                instrumentation: scope,
                ...(record.observedTimeUnixNano === undefined
                    ? {}
                    : { observed: microseconds(record.observedTimeUnixNano) }),
                ...(read.split.personal === undefined ? {} : { personal: read.split.personal }),
            },
        };
    }

    /** Read a visit record as a visit of its route. */
    #visit(read: OtlpRecord): Item<typeof kinds.visit> {
        // read the visit's text attributes
        const { attributes, person, personal } = read.split;
        const text = (key: string) => textOf(attributes[key]);
        const path = text(ANALYTICS_ATTRIBUTES.path);
        const title = text(ANALYTICS_ATTRIBUTES.title);
        const referrer = text(ANALYTICS_ATTRIBUTES.referrer);
        const locale = text(ANALYTICS_ATTRIBUTES.locale);

        // refuse a visit naming no route
        const route = text(ANALYTICS_ATTRIBUTES.route);
        if (route === undefined) {
            throw ObservabilityError.of("VISIT_ROUTE_MISSING", "the visit has no route");
        }

        return {
            id: read.id,
            time: read.at,
            ...this.#stamp(),
            route,
            session: read.split.session,
            trace: read.trace ?? null,
            visitor: null,
            person,
            data: {
                ...(path === undefined ? {} : { path }),
                ...(title === undefined ? {} : { title }),
                ...(referrer === undefined ? {} : { referrer }),
                ...(locale === undefined ? {} : { locale }),
                ...(personal === undefined ? {} : { personal }),
            },
        };
    }

    /** Read an action record as an action of its name with its properties. */
    #action(read: OtlpRecord): Item<typeof kinds.action> {
        const {
            [ANALYTICS_ATTRIBUTES.action]: name,
            [ANALYTICS_ATTRIBUTES.route]: route,
            ...properties
        } = read.split.attributes;
        if (typeof name !== "string" || name === "") {
            throw ObservabilityError.of("ACTION_NAME_MISSING", "the action has no name");
        }

        return {
            id: read.id,
            time: read.at,
            ...this.#stamp(),
            name,
            route: typeof route === "string" ? route : null,
            session: read.split.session,
            trace: read.trace ?? null,
            visitor: null,
            person: read.split.person,
            attributes: properties,
            data: read.split.personal === undefined ? {} : { personal: read.split.personal },
        };
    }

    /** Read a session's end record as the session, by how it ended. */
    #session(read: OtlpRecord): Item<typeof kinds.session> {
        // refuse an end without a status, or with an unknown one
        const value = read.split.attributes[SESSION_STATUS_ATTRIBUTE];
        const status = SESSION_STATUSES.find((each) => each === value);
        if (value === undefined) {
            throw ObservabilityError.of("SESSION_STATUS_MISSING", "the session end has no status");
        } else if (status === undefined) {
            throw ObservabilityError.of(
                "SESSION_STATUS_INVALID",
                `unknown session status ${JSON.stringify(value)}`,
            );
        }

        return {
            id: read.id,
            time: read.at,
            ...this.#stamp(),
            status,
            visitor: null,
            person: read.split.person,
            data: {
                ...this.#instance(),
                ...(read.split.personal === undefined ? {} : { personal: read.split.personal }),
            },
        };
    }

    /** Split attributes into the queried ones and the personal ones. */
    #split(attributes: Attributes): AttributeSplit {
        // read the person the emitter verified, else the one a workload instance's record names
        const named = attributes[PERSON_ATTRIBUTE];
        const claimed = textOf(named);
        const { person: verified, instance } = this.emitter;
        if (instance === undefined && claimed !== undefined && claimed !== verified) {
            throw ObservabilityError.of(
                "PERSON_UNVERIFIED",
                `a page record names person ${claimed}, whom its machine did not verify`,
            );
        }
        const person = verified ?? claimed ?? null;

        // read the session they were recorded in
        const session = textOf(attributes[SESSION_ATTRIBUTE]) ?? null;

        // separate the personal attributes, and only for a person
        const personal = Object.fromEntries(
            Object.entries(attributes).flatMap(([key, value]) =>
                key.startsWith(SENSITIVE_PREFIX)
                    ? [[key.slice(SENSITIVE_PREFIX.length), value]]
                    : [],
            ),
        );
        const kept = Object.fromEntries(
            Object.entries(attributes).filter(
                ([key]) =>
                    !key.startsWith(SENSITIVE_PREFIX) &&
                    key !== PERSON_ATTRIBUTE &&
                    key !== SESSION_ATTRIBUTE,
            ),
        );

        return {
            attributes: keyOf(kept),
            ...(person === null || Object.keys(personal).length === 0 ? {} : { personal }),
            person,
            session,
        };
    }

    /** Identify an event of the resource by its contents, the same on every retry. */
    #identity(contents: readonly unknown[]): Promise<string> {
        return Event.identity(this.emitter.scope, contents, this.#crypto);
    }

    /** Read the installation and build the forwarder named, as keys. */
    #stamp(): {
        readonly installationId: Identifier<"installation"> | null;
        readonly build: string | null;
    } {
        return {
            installationId: this.emitter.installation ?? null,
            build: this.emitter.build ?? null,
        };
    }

    /** Read the instance the forwarder named, as data. */
    #instance(): { readonly instance?: string } {
        return this.emitter.instance === undefined ? {} : { instance: this.emitter.instance };
    }
}

/** Refuse a cumulative metric, whose points cannot be added up: a gauge names no temporality. */
function requireDelta(metric: OtlpMetric, temporality: number | undefined): void {
    if (temporality !== undefined && temporality !== DELTA) {
        throw ObservabilityError.of(
            "TEMPORALITY_UNSUPPORTED",
            `metric ${metric.name} is cumulative: export deltas`,
        );
    }
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
    // drop the attributes without a value
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

/** Attribute the symbolicated frames an exception's attributes keep, leaving other attributes. */
async function attributeFrames(
    attributes: OtlpAttributes,
    attribute: (frames: readonly StackFrame[]) => Promise<StackFrame[]>,
): Promise<OtlpAttributes> {
    return Promise.all(
        attributes.map(async (entry) => {
            // leave an attribute other than the frames
            const frames = FramesEntry.safeParse(entry);
            if (!frames.success) {
                return entry;
            }

            // rewrite the frames as their build attributes them
            const read = StackFrame.array().parse(JSON.parse(frames.data.value.stringValue));

            return { ...entry, value: { stringValue: JSON.stringify(await attribute(read)) } };
        }),
    );
}
