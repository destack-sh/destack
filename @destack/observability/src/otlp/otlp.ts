import type { Select } from "@destack/db";
import type { ObjectConfiguration, ObjectType } from "@destack/object";
import { Digest, type DigestCrypto, type Identifier, schema } from "@destack/schema";
import { ObservabilityError } from "../error/error.ts";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    type OtlpEmitter,
    SESSION_ATTRIBUTE,
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    SESSION_STATUSES,
    VISIT_EVENT,
    EMITTER_ATTRIBUTES,
    OtlpExport,
    type OtlpKeyValue,
    OtlpLogsRequest,
    OtlpMetricsRequest,
    OtlpTracesRequest,
    type OtlpValue,
} from "@destack/telemetry/otlp";
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
import * as kinds from "../object/index.ts";

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
    /** The contents its identifiers derive from, the same on every retry. */
    readonly contents: readonly unknown[];
    /** The body as text. */
    readonly text: string | undefined;
    /** The trace. */
    readonly trace: string | undefined;
    /** The attributes split for keeping. */
    readonly split: AttributeSplit;
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

    /** Decode each resource's records of an export in a scope, under its attributes' emitter. */
    static async decode(
        exported: OtlpExport,
        scope: string,
        crypto: DigestCrypto,
    ): Promise<OtlpResource[]> {
        // decode the log records of each resource
        if (exported.signal === "logs") {
            return Promise.all(
                OtlpLogsRequest.parse(exported.body).resourceLogs.map(async (group) => {
                    const decoder = new OtlpDecoder(scope, group.resource.attributes, crypto);

                    return new OtlpResource(decoder.emitter, await decoder.logs(group.scopeLogs));
                }),
            );
        }
        // decode the spans of each resource
        else if (exported.signal === "traces") {
            return Promise.all(
                OtlpTracesRequest.parse(exported.body).resourceSpans.map(async (group) => {
                    const decoder = new OtlpDecoder(scope, group.resource.attributes, crypto);

                    return new OtlpResource(decoder.emitter, await decoder.spans(group.scopeSpans));
                }),
            );
        }
        // decode the metric points of each resource
        else {
            return Promise.all(
                OtlpMetricsRequest.parse(exported.body).resourceMetrics.map(async (group) => {
                    const decoder = new OtlpDecoder(scope, group.resource.attributes, crypto);
                    const events = await decoder.metrics(group.scopeMetrics);

                    return new OtlpResource(decoder.emitter, events);
                }),
            );
        }
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
                    events.visit.push(this.#visit(read, await this.#identity(kinds.visit, read)));
                } else if (read.name === ACTION_EVENT) {
                    events.action.push(
                        this.#action(read, await this.#identity(kinds.action, read)),
                    );
                } else if (read.name === SESSION_EVENTS.end) {
                    events.session.push(
                        this.#session(read, await this.#identity(kinds.session, read)),
                    );
                } else {
                    const id = await this.#identity(kinds.log, read);
                    events.log.push(this.#log(record, read, scope, id));
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
                        `span '${span.spanId}' is invalid: it ends before it starts`,
                    );
                }
                const split = this.#split(attributesOf(span.attributes));

                // add it with its annotations and links
                events.span.push({
                    id: await this.#identity(kinds.span, { contents: [span.traceId, span.spanId] }),
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
                            "POINT_VALUE_REQUIRED",
                            `point value of metric '${metric.name}' is required`,
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
                "METRIC_INVALID",
                `metric '${metric.name}' is invalid: it must be a delta sum, gauge or exponential histogram`,
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
        const id = await this.#identity(kinds.metric, {
            contents: [time, metric.name, this.emitter.instance, kept, start],
        });

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
            throw ObservabilityError.of("LOG_EVENT_REQUIRED", "log record event name is required");
        } else if (time === undefined) {
            throw ObservabilityError.of(
                "LOG_TIME_REQUIRED",
                `time of log record '${name}' is required`,
            );
        }

        // read the body as text, the trace and the attributes
        const body = record.body === undefined ? undefined : valueOf(record.body);
        const text = body === undefined || typeof body === "string" ? body : JSON.stringify(body);
        const at = microseconds(time);
        const trace = presentText(record.traceId);
        const split = this.#split(attributesOf(record.attributes));

        // collect the contents that identify it
        const contents = [at, name, trace, record.spanId, text, record.attributes];

        return { name, at, contents, text, trace, split };
    }

    /** Read a log record as a log event. */
    #log(
        record: OtlpLogRecord,
        read: OtlpRecord,
        scope: Instrumentation,
        id: Identifier<"log">,
    ): Item<typeof kinds.log> {
        const span = presentText(record.spanId);

        return {
            id,
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
    #visit(read: OtlpRecord, id: Identifier<"visit">): Item<typeof kinds.visit> {
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
            throw ObservabilityError.of("VISIT_ROUTE_REQUIRED", "visit route is required");
        }

        return {
            id,
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
    #action(read: OtlpRecord, id: Identifier<"action">): Item<typeof kinds.action> {
        const {
            [ANALYTICS_ATTRIBUTES.action]: name,
            [ANALYTICS_ATTRIBUTES.route]: route,
            ...properties
        } = read.split.attributes;
        if (typeof name !== "string" || name === "") {
            throw ObservabilityError.of("ACTION_NAME_REQUIRED", "action name is required");
        }

        return {
            id,
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
    #session(read: OtlpRecord, id: Identifier<"session">): Item<typeof kinds.session> {
        // refuse an end without a status, or with an unknown one
        const value = read.split.attributes[SESSION_STATUS_ATTRIBUTE];
        const status = SESSION_STATUSES.find((each) => each === value);
        if (value === undefined) {
            throw ObservabilityError.of(
                "SESSION_STATUS_REQUIRED",
                "session end status is required",
            );
        } else if (status === undefined) {
            throw ObservabilityError.of(
                "SESSION_STATUS_INVALID",
                `session status ${JSON.stringify(value)} is invalid: it must be a known status`,
            );
        }

        return {
            id,
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
                "PERSON_DENIED",
                `person '${claimed}' is forbidden: a page record names it, and its machine did not verify it`,
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

    /** Derive an event's identifier of its type from its contents, the same on every retry. */
    #identity<Configuration extends ObjectConfiguration>(
        object: ObjectType<Configuration>,
        read: Pick<OtlpRecord, "contents">,
    ): Promise<Identifier<Configuration["identity"]>> {
        return object.derive(this.emitter.scope, JSON.stringify(read.contents), this.#crypto);
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
            "TEMPORALITY_INVALID",
            `metric '${metric.name}' is invalid: it is cumulative and must export deltas`,
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
