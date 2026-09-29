import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { startTelemetry } from "../host/index.ts";
import { OtlpExporter, type OtlpSignal } from "./exporter.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** The resource every signal of the package carries. */
const resource = {
    attributes: [
        { key: "service.name", value: { stringValue: "@example/notes" } },
        { key: "service.version", value: { stringValue: "2026.9.0" } },
    ],
    droppedAttributesCount: 0,
};

test("export a log record inside its span as OTLP/JSON, correlated by trace", async () => {
    // deliver every request as parsed JSON
    const delivered: { signal: OtlpSignal; body: unknown }[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            delivered.push({ signal, body: JSON.parse(new TextDecoder().decode(body)) });
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source));

    // log inside a span, then export both
    try {
        const { log, span } = telemetry.scope(source);
        await span("note.render", { blocks: 12 }, () => log.info("note.saved", { length: 5 }));
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // name the span's trace and span on the log record
    const [traces, logs] = delivered as [
        {
            body: {
                resourceSpans: [{ scopeSpans: [{ spans: [{ traceId: string; spanId: string }] }] }];
            };
        },
        unknown,
    ];
    const { traceId, spanId } = traces.body.resourceSpans[0].scopeSpans[0].spans[0];
    const scope = { name: "@example/notes", version: "2026.9.0" };
    expect([traces, logs]).toEqual([
        {
            signal: "traces",
            body: {
                resourceSpans: [
                    {
                        resource,
                        scopeSpans: [
                            {
                                scope,
                                spans: [
                                    {
                                        traceId,
                                        spanId,
                                        name: "note.render",
                                        kind: 1,
                                        startTimeUnixNano: expect.any(String),
                                        endTimeUnixNano: expect.any(String),
                                        attributes: [{ key: "blocks", value: { intValue: 12 } }],
                                        droppedAttributesCount: 0,
                                        events: [],
                                        droppedEventsCount: 0,
                                        status: { code: 0 },
                                        links: [],
                                        droppedLinksCount: 0,
                                        flags: 257,
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
        },
        {
            signal: "logs",
            body: {
                resourceLogs: [
                    {
                        resource,
                        scopeLogs: [
                            {
                                scope,
                                logRecords: [
                                    {
                                        timeUnixNano: expect.any(String),
                                        observedTimeUnixNano: expect.any(String),
                                        severityNumber: 9,
                                        severityText: "INFO",
                                        body: {},
                                        eventName: "note.saved",
                                        attributes: [{ key: "length", value: { intValue: 5 } }],
                                        droppedAttributesCount: 0,
                                        traceId,
                                        spanId,
                                        flags: 1,
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
        },
    ]);
});
