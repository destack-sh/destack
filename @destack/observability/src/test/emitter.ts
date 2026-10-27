/** An attribute value as a test writes it. */
export type TestValue = string | number | boolean | readonly string[];

/** Attributes as a test writes them, by key. */
export type TestAttributes = Readonly<Record<string, TestValue>>;

/** A log record an emitter exports, timed in Unix milliseconds. */
export interface TestLog {
    /** When it was emitted. */
    readonly time: number;
    /** Its event name. */
    readonly name: string;
    /** Its severity, absent for none. */
    readonly severity?: number;
    /** Its body, absent for none. */
    readonly body?: string;
    /** Its attributes. */
    readonly attributes?: TestAttributes;
    /** The trace it was emitted in. */
    readonly trace?: string;
    /** The span it was emitted in. */
    readonly span?: string;
}

/** A span an emitter exports, timed in Unix milliseconds. */
export interface TestSpan {
    /** The trace. */
    readonly trace: string;
    /** The span. */
    readonly span: string;
    /** The parent span, absent for a root. */
    readonly parent?: string;
    /** The span's name. */
    readonly name: string;
    /** When it started. */
    readonly start: number;
    /** When it ended. */
    readonly end: number;
    /** Its OTLP status code. */
    readonly status?: 0 | 1 | 2;
    /** Its attributes. */
    readonly attributes?: TestAttributes;
    /** The events it recorded, such as exceptions. */
    readonly annotations?: readonly {
        /** The event's name. */
        readonly name: string;
        /** When it was recorded. */
        readonly time: number;
        /** Its attributes. */
        readonly attributes: TestAttributes;
    }[];
}

/** An exception record an emitter exports, timed in Unix milliseconds. */
export interface TestException {
    /** When it was captured. */
    readonly time: number;
    /** Its exception attributes. */
    readonly attributes: TestAttributes;
    /** The trace it was captured in. */
    readonly trace?: string;
    /** The span it was captured in. */
    readonly span?: string;
}

/** A workload exporting OTLP/JSON under its resource attributes and instrumentation scope. */
export class TestEmitter {
    /** The resource attributes each export carries. */
    readonly resource: TestAttributes;
    /** The instrumentation scope each export carries. */
    readonly scope: { readonly name: string; readonly version: string };

    /** Keep an emitter's resource and scope. */
    constructor(
        resource: TestAttributes,
        scope: { readonly name: string; readonly version: string },
    ) {
        this.resource = resource;
        this.scope = scope;
    }

    /** Export under further resource attributes, such as the browser a page runs in. */
    with(resource: TestAttributes): TestEmitter {
        return new TestEmitter({ ...this.resource, ...resource }, this.scope);
    }

    /** Write a logs export of records. */
    logs(records: readonly TestLog[]) {
        return {
            resourceLogs: [
                {
                    resource: { attributes: TestEmitter.#attributes(this.resource) },
                    scopeLogs: [
                        {
                            scope: this.scope,
                            logRecords: records.map((record) => ({
                                timeUnixNano: TestEmitter.nanoseconds(record.time),
                                ...(record.severity === undefined
                                    ? {}
                                    : { severityNumber: record.severity }),
                                eventName: record.name,
                                ...(record.body === undefined
                                    ? {}
                                    : { body: { stringValue: record.body } }),
                                attributes: TestEmitter.#attributes(record.attributes),
                                ...(record.trace === undefined ? {} : { traceId: record.trace }),
                                ...(record.span === undefined ? {} : { spanId: record.span }),
                            })),
                        },
                    ],
                },
            ],
        };
    }

    /** Write a logs export of exception records at the error severity. */
    exceptions(records: readonly TestException[]) {
        return this.logs(records.map((record) => ({ ...record, name: "exception", severity: 17 })));
    }

    /** Write a traces export of spans. */
    traces(spans: readonly TestSpan[]) {
        return {
            resourceSpans: [
                {
                    resource: { attributes: TestEmitter.#attributes(this.resource) },
                    scopeSpans: [
                        {
                            scope: this.scope,
                            spans: spans.map((span) => ({
                                traceId: span.trace,
                                spanId: span.span,
                                ...(span.parent === undefined ? {} : { parentSpanId: span.parent }),
                                name: span.name,
                                startTimeUnixNano: TestEmitter.nanoseconds(span.start),
                                endTimeUnixNano: TestEmitter.nanoseconds(span.end),
                                attributes: TestEmitter.#attributes(span.attributes),
                                events: (span.annotations ?? []).map((annotation) => ({
                                    name: annotation.name,
                                    timeUnixNano: TestEmitter.nanoseconds(annotation.time),
                                    attributes: TestEmitter.#attributes(annotation.attributes),
                                })),
                                status: { code: span.status ?? 0 },
                            })),
                        },
                    ],
                },
            ],
        };
    }

    /** Write a metrics export of metrics as OTLP/JSON writes them. */
    metrics(metrics: readonly Readonly<Record<string, unknown>>[]) {
        return {
            resourceMetrics: [
                {
                    resource: { attributes: TestEmitter.#attributes(this.resource) },
                    scopeMetrics: [{ scope: this.scope, metrics }],
                },
            ],
        };
    }

    /** Write milliseconds as OTLP nanoseconds. */
    static nanoseconds(time: number): string {
        return String(BigInt(time) * 1_000_000n);
    }

    /** Write attributes as OTLP writes them. */
    static #attributes(attributes: TestAttributes = {}) {
        return Object.entries(attributes).map(([key, value]) => ({
            key,
            value: TestEmitter.#value(value),
        }));
    }

    /** Write one attribute value as OTLP writes it, a list as an array of strings. */
    static #value(value: TestValue) {
        // write a scalar as its typed value
        if (typeof value === "string") {
            return { stringValue: value };
        } else if (typeof value === "number" && Number.isInteger(value)) {
            return { intValue: String(value) };
        } else if (typeof value === "number") {
            return { doubleValue: value };
        } else if (typeof value === "boolean") {
            return { boolValue: value };
        }

        return { arrayValue: { values: value.map((item) => ({ stringValue: item })) } };
    }
}
