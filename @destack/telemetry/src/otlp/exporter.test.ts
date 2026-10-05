import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { startTelemetry } from "../bun/index.ts";
import { OtlpExporter, type OtlpSignal } from "./exporter.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** A traces request with one span, read for its trace and span identifiers. */
const TracesRequest = schema.looseObject({
    signal: schema.literal("traces"),
    body: schema.looseObject({
        resourceSpans: schema.tuple([
            schema.looseObject({
                scopeSpans: schema.tuple([
                    schema.looseObject({
                        spans: schema.tuple([
                            schema.looseObject({
                                traceId: schema.string(),
                                spanId: schema.string(),
                            }),
                        ]),
                    }),
                ]),
            }),
        ]),
    }),
});

/** The resource every signal of the package carries. */
const resource: unknown = expect.objectContaining({
    attributes: [
        { key: "service.name", value: { stringValue: "@example/notes" } },
        { key: "service.version", value: { stringValue: "2026.9.0" } },
    ],
});

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

    // log inside a span and export both
    try {
        const { log, span } = telemetry.scope(source);
        await span("note.render", { blocks: 12 }, () => log.info("note.saved", { length: 5 }));
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // name the span's trace and span on the log record
    const [traces, logs] = delivered;
    const [{ scopeSpans }] = TracesRequest.parse(traces).body.resourceSpans;
    const { traceId, spanId } = scopeSpans[0].spans[0];
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
                                    expect.objectContaining({
                                        traceId,
                                        spanId,
                                        name: "note.render",
                                        attributes: [{ key: "blocks", value: { intValue: 12 } }],
                                    }),
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
                                    expect.objectContaining({
                                        severityNumber: 9,
                                        severityText: "INFO",
                                        eventName: "note.saved",
                                        attributes: [{ key: "length", value: { intValue: 5 } }],
                                        traceId,
                                        spanId,
                                    }),
                                ],
                            },
                        ],
                    },
                ],
            },
        },
    ]);
});
