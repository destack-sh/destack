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

/** A logs request with one record, read for its attributes. */
const LogsRequest = schema.looseObject({
    resourceLogs: schema.tuple([
        schema.looseObject({
            scopeLogs: schema.tuple([
                schema.looseObject({
                    logRecords: schema.tuple([
                        schema.looseObject({
                            attributes: schema.array(
                                schema.looseObject({
                                    key: schema.string(),
                                    value: schema.unknown(),
                                }),
                            ),
                        }),
                    ]),
                }),
            ]),
        }),
    ]),
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

test("post each signal below a target's endpoint through the target's fetch", async () => {
    // record each request the target's fetch posts
    const posted: { url: string; method: string | undefined; type: string | null }[] = [];
    const exporter = OtlpExporter.http(
        {
            endpoint: "http://machine.test/observability",
            fetch: async (url, options) => {
                posted.push({
                    url,
                    method: options.method,
                    type: new Headers(options.headers).get("content-type"),
                });

                return new Response(null, { status: 200 });
            },
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source));

    // log once and export it
    try {
        telemetry.scope(source).log.info("note.saved", { length: 5 });
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // post the logs below the endpoint as OTLP/JSON
    expect(posted).toEqual([
        {
            url: "http://machine.test/observability/v1/logs",
            method: "POST",
            type: "application/json",
        },
    ]);
});

test("export a captured exception with the frames its build's symbolicator resolves", async () => {
    // resolve each frame to its source, as a build's source maps do
    const delivered: unknown[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            if (signal === "logs") {
                delivered.push(JSON.parse(new TextDecoder().decode(body)));
            }
        },
        (error) => {
            throw error;
        },
        {
            frames: async (frames) =>
                frames.map((frame) => ({ ...frame, path: "src/note.ts", line: 7, column: 3 })),
        },
    );
    const telemetry = await startTelemetry(exporter.options(source));

    // capture an exception thrown in the package
    const error = new RangeError("page 9 of 3");
    error.stack =
        "RangeError: page 9 of 3\n    at Note.render (file:///srv/output/workload.js:12:34)";
    try {
        telemetry.scope(source).captureException(error);
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // keep the resolved frames beside the raw trace
    const frames = [
        {
            function: "Note.render",
            file: "file:///srv/output/workload.js",
            line: 7,
            column: 3,
            isInApp: true,
            path: "src/note.ts",
        },
    ];
    const [{ resourceLogs }] = schema.tuple([LogsRequest]).parse(delivered);
    const attributes = resourceLogs[0].scopeLogs[0].logRecords[0].attributes;
    expect(attributes.find(({ key }) => key === "destack.exception.frames")).toEqual({
        key: "destack.exception.frames",
        value: { stringValue: JSON.stringify(frames) },
    });
});
