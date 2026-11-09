import { schema } from "@destack/schema";
import { StackFrame } from "@destack/package/stack";
import { TelemetryError } from "../error/error.ts";
import { FRAMES_ATTRIBUTE, type OtlpSignal } from "./exporter.ts";

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
export type OtlpValue = {
    readonly stringValue?: string;
    readonly boolValue?: boolean;
    readonly intValue?: string | number;
    readonly doubleValue?: number;
    readonly bytesValue?: string;
    readonly arrayValue?: { readonly values?: readonly OtlpValue[] };
    readonly kvlistValue?: { readonly values?: readonly OtlpKeyValue[] };
};

/** An OTLP attribute. */
export type OtlpKeyValue = { readonly key: string; readonly value?: OtlpValue };

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
    resourceLogs: schema
        .array(
            schema.looseObject({
                resource: Resource,
                scopeLogs: schema
                    .array(
                        schema.looseObject({
                            scope: Scope,
                            logRecords: schema
                                .array(
                                    schema.looseObject({
                                        timeUnixNano: Integer.exactOptional(),
                                        observedTimeUnixNano: Integer.exactOptional(),
                                        severityNumber: schema
                                            .number()
                                            .int()
                                            .min(0)
                                            .max(24)
                                            .exactOptional(),
                                        eventName: schema.string().exactOptional(),
                                        body: OtlpValue.exactOptional(),
                                        attributes: OtlpAttributes,
                                        traceId: OptionalTrace,
                                        spanId: OptionalSpan,
                                    }),
                                )
                                .default([]),
                        }),
                    )
                    .default([]),
            }),
        )
        .default([]),
});
/** An OTLP/JSON logs export request. */
export type OtlpLogsRequest = schema.Infer<typeof OtlpLogsRequest>;

/** An OTLP/JSON traces export request. */
export const OtlpTracesRequest = schema.looseObject({
    resourceSpans: schema
        .array(
            schema.looseObject({
                resource: Resource,
                scopeSpans: schema
                    .array(
                        schema.looseObject({
                            scope: Scope,
                            spans: schema
                                .array(
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
                                                    traceId: schema
                                                        .string()
                                                        .regex(/^[0-9a-f]{32}$/u),
                                                    spanId: schema
                                                        .string()
                                                        .regex(/^[0-9a-f]{16}$/u),
                                                }),
                                            )
                                            .default([]),
                                        status: schema
                                            .looseObject({
                                                code: schema.literal([0, 1, 2]).default(0),
                                            })
                                            .default({ code: 0 }),
                                    }),
                                )
                                .default([]),
                        }),
                    )
                    .default([]),
            }),
        )
        .default([]),
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
    resourceMetrics: schema
        .array(
            schema.looseObject({
                resource: Resource,
                scopeMetrics: schema
                    .array(
                        schema.looseObject({
                            scope: Scope,
                            metrics: schema
                                .array(
                                    schema.looseObject({
                                        name: schema.string().min(1),
                                        unit: schema.string().default(""),
                                        sum: schema
                                            .looseObject({
                                                dataPoints: schema.array(NumberPoint).default([]),
                                                aggregationTemporality: schema.number().int(),
                                            })
                                            .exactOptional(),
                                        gauge: schema
                                            .looseObject({
                                                dataPoints: schema.array(NumberPoint).default([]),
                                            })
                                            .exactOptional(),
                                        histogram: schema.looseObject({}).exactOptional(),
                                        exponentialHistogram: schema
                                            .looseObject({
                                                aggregationTemporality: schema.number().int(),
                                                dataPoints: schema
                                                    .array(
                                                        schema.looseObject({
                                                            attributes: OtlpAttributes,
                                                            startTimeUnixNano:
                                                                Integer.exactOptional(),
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
                                                    )
                                                    .default([]),
                                            })
                                            .exactOptional(),
                                    }),
                                )
                                .default([]),
                        }),
                    )
                    .default([]),
            }),
        )
        .default([]),
});
/** An OTLP/JSON metrics export request. */
export type OtlpMetricsRequest = schema.Infer<typeof OtlpMetricsRequest>;

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

    /** Read the request by a schema of its signal, refusing a body it does not match. */
    #read<Request>(shape: schema.Schema<Request>): Request {
        // refuse it naming each mismatch at its path
        const parsed = shape.safeParse(this.body);
        if (!parsed.success) {
            const issues = parsed.error.issues.map(
                (issue) => `${issue.path.join(".")}: ${issue.message}`,
            );
            throw TelemetryError.of(
                "EXPORT_INVALID",
                `the ${this.signal} export is invalid at ${issues.join("; ")}`,
            );
        }

        return parsed.data;
    }

    /** Set the emitter a forwarding machine knows on each resource, refusing a claimed emitter. */
    stamp(emitter: Readonly<Partial<Record<keyof typeof EMITTER_ATTRIBUTES, string>>>): OtlpExport {
        // refuse resources claiming the forwarder's emitter attribute
        const groups = RESOURCE_GROUPS[this.signal];
        const request = this.#read(
            schema.looseObject({
                [groups]: schema.array(schema.looseObject({ resource: Resource })).default([]),
            }),
        );
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
            throw TelemetryError.of(
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
            const request = this.#read(OtlpLogsRequest);
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
            const request = this.#read(OtlpTracesRequest);
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
