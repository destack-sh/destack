import { Event } from "@destack/event";
import type { Select } from "@destack/db";
import type { ObjectType } from "@destack/object";
import { Digest, type Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/object";
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
} from "@destack/telemetry/otlp";
import {
    type AttributeKey,
    type Attributes,
    type AttributeValue,
    type Instrumentation,
    PERSON_ATTRIBUTE,
    SENSITIVE_PREFIX,
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

/** An event of a type as its scope appends it: its row without the scope, with its data. */
export type Item<Type extends ObjectType> = Omit<Select<Type["table"]>, "scope" | "data"> & {
    readonly data: NonNullable<Select<Type["table"]>["data"]>;
};

/** The events of one resource, by type. */
export interface Decoded {
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
interface Split {
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

/** What every log record carries: its event name, time, identity, body, trace and attributes. */
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
    readonly split: Split;
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
            throw new CallError("FORBIDDEN", {
                message: `only the receiver records an export's emitter, found ${[...claimed].join(", ")}`,
            });
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

    /** Decode each resource's records in a scope, under the emitter its attributes name. */
    async decode(scope: string): Promise<OtlpResource[]> {
        // decode the log records of each resource
        if (this.signal === "logs") {
            return Promise.all(
                OtlpLogsRequest.parse(this.body).resourceLogs.map(async (group) => {
                    const resource = new OtlpResource(scope, group.resource.attributes);
                    await resource.addLogs(group.scopeLogs);

                    return resource;
                }),
            );
        }
        // decode the spans of each resource
        else if (this.signal === "traces") {
            return Promise.all(
                OtlpTracesRequest.parse(this.body).resourceSpans.map(async (group) => {
                    const resource = new OtlpResource(scope, group.resource.attributes);
                    await resource.addSpans(group.scopeSpans);

                    return resource;
                }),
            );
        }
        // decode the metric points of each resource
        else {
            return Promise.all(
                OtlpMetricsRequest.parse(this.body).resourceMetrics.map(async (group) => {
                    const resource = new OtlpResource(scope, group.resource.attributes);
                    await resource.addMetrics(group.scopeMetrics);

                    return resource;
                }),
            );
        }
    }
}

/** One resource of an export: its emitter, as its forwarder named it, and its events by type. */
export class OtlpResource {
    /** The emitter. */
    readonly emitter: OtlpEmitter;
    /** The events decoded so far, by type. */
    events: Decoded = { log: [], span: [], metric: [], visit: [], action: [], session: [] };

    /** Read a resource's emitter in a scope from the attributes its forwarding machine set. */
    constructor(scope: string, resource: readonly OtlpKeyValue[]) {
        // read the forwarder's attributes as text
        const attributes = attributesOf(resource);
        const text = (key: string) => {
            const value = attributes[key];

            return typeof value === "string" && value !== "" ? value : undefined;
        };
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

    /** Add the resource's log records, visits and actions. */
    async addLogs(groups: OtlpLogsRequest["resourceLogs"][number]["scopeLogs"]): Promise<void> {
        for (const { scope, logRecords } of groups) {
            for (const record of logRecords) {
                // read a visit, an action or a session's end by its reserved name, else a log record
                const read = await this.#record(record);
                if (read.name === VISIT_EVENT) {
                    this.events.visit.push(this.#visit(read));
                } else if (read.name === ACTION_EVENT) {
                    this.events.action.push(this.#action(read));
                } else if (read.name === SESSION_EVENTS.end) {
                    this.events.session.push(this.#session(read));
                } else {
                    this.events.log.push(this.#log(record, read, scope));
                }
            }
        }
    }

    /** Add the resource's spans with their annotations and links. */
    async addSpans(
        groups: OtlpTracesRequest["resourceSpans"][number]["scopeSpans"],
    ): Promise<void> {
        for (const { scope, spans } of groups) {
            for (const span of spans) {
                // time the span and split its attributes
                const start = microseconds(span.startTimeUnixNano);
                const end = microseconds(span.endTimeUnixNano);
                const split = this.#split(attributesOf(span.attributes));

                // add it with its annotations and links
                this.events.span.push({
                    id: await this.#identity([span.traceId, span.spanId]),
                    time: start,
                    ...this.#stamp(),
                    name: span.name,
                    trace: span.traceId,
                    parent: presentText(span.parentSpanId) ?? null,
                    status: SPAN_STATUSES[span.status.code],
                    duration: Math.max(0, end - start),
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
    }

    /** Add the points of the resource's metrics. */
    async addMetrics(
        groups: OtlpMetricsRequest["resourceMetrics"][number]["scopeMetrics"],
    ): Promise<void> {
        for (const { scope, metrics } of groups) {
            for (const metric of metrics) {
                this.events.metric.push(...(await this.#points(metric, scope)));
            }
        }
    }

    /** Set the issue of each log record and span recording a grouped exception, by `type:index`. */
    setIssues(issues: ReadonlyMap<string, string>): void {
        this.events = {
            ...this.events,
            log: this.events.log.map((event, index) => ({
                ...event,
                issue: issues.get(`log:${index}`) ?? event.issue,
            })),
            span: this.events.span.map((event, index) => ({
                ...event,
                issue: issues.get(`span:${index}`) ?? event.issue,
            })),
        };
    }

    /** Set the visitor's hash on each visit, action and session. */
    setVisitor(visitor: string): void {
        this.events = {
            ...this.events,
            visit: this.events.visit.map((event) => ({ ...event, visitor })),
            action: this.events.action.map((event) => ({ ...event, visitor })),
            session: this.events.session.map((event) => ({ ...event, visitor })),
        };
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
                        throw new CallError("BAD_REQUEST", {
                            message: `a point of metric ${metric.name} carries no value`,
                        });
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
            throw new CallError("BAD_REQUEST", {
                message: `metric ${metric.name} is no delta sum, gauge or exponential histogram`,
            });
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

    /** Read what every log record carries. */
    async #record(record: OtlpLogRecord): Promise<OtlpRecord> {
        // require the event name and the time
        const time = record.timeUnixNano ?? record.observedTimeUnixNano;
        const name =
            record.eventName === undefined || record.eventName === ""
                ? record.body?.stringValue
                : record.eventName;
        if (name === undefined || name === "") {
            throw new CallError("BAD_REQUEST", { message: "a log record names no event" });
        } else if (time === undefined) {
            throw new CallError("BAD_REQUEST", { message: `log record ${name} carries no time` });
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
            throw new CallError("BAD_REQUEST", { message: "a visit names no route" });
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

        return {
            id: read.id,
            time: read.at,
            ...this.#stamp(),
            name: typeof name === "string" ? name : "",
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
        // refuse an end naming no status the sessions know
        const status = SESSION_STATUSES.find(
            (each) => each === read.split.attributes[SESSION_STATUS_ATTRIBUTE],
        );
        if (status === undefined) {
            throw new CallError("BAD_REQUEST", { message: "a session's end names no status" });
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
    #split(attributes: Attributes): Split {
        // read the person the emitter verified, else the one the attributes name
        const named = attributes[PERSON_ATTRIBUTE];
        const person =
            this.emitter.person ?? (typeof named === "string" && named !== "" ? named : null);

        // read the session they were recorded in
        const recorded = attributes[SESSION_ATTRIBUTE];
        const session = typeof recorded === "string" && recorded !== "" ? recorded : null;

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
        return Event.identity(this.emitter.scope, contents);
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
        throw new CallError("BAD_REQUEST", {
            message: `metric ${metric.name} is cumulative: export deltas`,
        });
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
    // drop the attributes without a value, as an empty value carries none
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
