import { testSimulation } from "@destack/test/world";
import { TelemetryScope } from "../scope/index.ts";
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

test("export a log record inside its span as OTLP/JSON, correlated by trace", async () => {
    // deliver every request as parsed JSON
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const delivered: { signal: OtlpSignal; body: unknown }[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            delivered.push({ signal, body: JSON.parse(new TextDecoder().decode(body)) });
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source), environment);

    // log inside a span and export both
    try {
        const { log, span } = TelemetryScope.of(source, telemetry);
        await span("note.render", { blocks: 12 }, () => log.info("note.saved", { length: 5 }));
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // export the span and the record correlated by trace, each whole
    expect(delivered).toMatchInlineSnapshot(`
      [
        {
          "body": {
            "resourceSpans": [
              {
                "resource": {
                  "attributes": [
                    {
                      "key": "service.name",
                      "value": {
                        "stringValue": "@example/notes",
                      },
                    },
                    {
                      "key": "service.version",
                      "value": {
                        "stringValue": "2026.9.0",
                      },
                    },
                  ],
                  "droppedAttributesCount": 0,
                },
                "scopeSpans": [
                  {
                    "scope": {
                      "name": "@example/notes",
                      "version": "2026.9.0",
                    },
                    "spans": [
                      {
                        "attributes": [
                          {
                            "key": "blocks",
                            "value": {
                              "intValue": 12,
                            },
                          },
                        ],
                        "droppedAttributesCount": 0,
                        "droppedEventsCount": 0,
                        "droppedLinksCount": 0,
                        "endTimeUnixNano": "1790812800000000000",
                        "events": [],
                        "flags": 257,
                        "kind": 1,
                        "links": [],
                        "name": "note.render",
                        "spanId": "e3440b233e0a77dc",
                        "startTimeUnixNano": "1790812800000000000",
                        "status": {
                          "code": 0,
                        },
                        "traceId": "01a0f4c2c40086a37896d7dfdfd3257b",
                      },
                    ],
                  },
                ],
              },
            ],
          },
          "signal": "traces",
        },
        {
          "body": {
            "resourceLogs": [
              {
                "resource": {
                  "attributes": [
                    {
                      "key": "service.name",
                      "value": {
                        "stringValue": "@example/notes",
                      },
                    },
                    {
                      "key": "service.version",
                      "value": {
                        "stringValue": "2026.9.0",
                      },
                    },
                  ],
                  "droppedAttributesCount": 0,
                },
                "scopeLogs": [
                  {
                    "logRecords": [
                      {
                        "attributes": [
                          {
                            "key": "length",
                            "value": {
                              "intValue": 5,
                            },
                          },
                        ],
                        "body": {},
                        "droppedAttributesCount": 0,
                        "eventName": "note.saved",
                        "flags": 1,
                        "observedTimeUnixNano": "1790812800000000000",
                        "severityNumber": 9,
                        "severityText": "INFO",
                        "spanId": "e3440b233e0a77dc",
                        "timeUnixNano": "1790812800000000000",
                        "traceId": "01a0f4c2c40086a37896d7dfdfd3257b",
                      },
                    ],
                    "scope": {
                      "name": "@example/notes",
                      "version": "2026.9.0",
                    },
                  },
                ],
              },
            ],
          },
          "signal": "logs",
        },
      ]
    `);
});

test("post each signal below a target's endpoint through the target's fetch", async () => {
    // record each request the target's fetch posts
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const posted: { url: string; method: string | undefined; type: string | null }[] = [];
    const exporter = OtlpExporter.http(
        {
            endpoint: "http://machine.test/observability",
            fetch: async (request) => {
                posted.push({
                    url: request.url,
                    method: request.method,
                    type: request.headers.get("content-type"),
                });

                return new Response(null, { status: 200 });
            },
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source), environment);

    // log once and export it
    try {
        TelemetryScope.of(source, telemetry).log.info("note.saved", { length: 5 });
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
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
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
    const telemetry = await startTelemetry(exporter.options(source), environment);

    // capture an exception thrown in the package
    const error = new RangeError("page 9 of 3");
    error.stack =
        "RangeError: page 9 of 3\n    at Note.render (file:///srv/output/workload.js:12:34)";
    try {
        TelemetryScope.of(source, telemetry).captureException(error);
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
