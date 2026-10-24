import { fileURLToPath } from "node:url";
import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Simulation } from "@destack/environment/simulation";
import { present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { OtlpExporter } from "../otlp/index.ts";
import { Telemetry } from "../sdk/index.ts";
import { observeProcess } from "./bun.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The package directory, whose preload a child process runs with. */
const root = fileURLToPath(new URL("../..", import.meta.url));

/** A process that writes each logs export to its output and fails as its environment asks. */
const failing = `
import { BunEnvironment } from ${JSON.stringify(fileURLToPath(import.meta.resolve("@destack/environment/bun")))};
import { startTelemetry } from ${JSON.stringify(fileURLToPath(new URL("index.ts", import.meta.url)))};
import { OtlpExporter } from ${JSON.stringify(fileURLToPath(new URL("../otlp/index.ts", import.meta.url)))};

const exporter = new OtlpExporter(async (signal, body) => {
    if (signal === "logs") {
        process.stdout.write(new TextDecoder().decode(body) + "\\n");
    }
}, () => {});
await startTelemetry(exporter.options({ name: "@example/notes", version: "2026.9.0" }), BunEnvironment.system);
if (process.env.FAILURE === "rejection") {
    void Promise.reject(new TypeError("note sync rejected"));
} else {
    setTimeout(() => {
        throw new RangeError("note index out of range");
    });
}
`;

/** Run the failing process to its end. */
async function fail(failure: "exception" | "rejection") {
    // run the process from a world, its output and errors piped
    await using simulation = new Simulation({ seed: 1, start: START });
    const { process } = simulation.environment("test");
    const child = process.spawn([process.executable, "-e", failing], {
        workingDirectory: root,
        variables: { ...process.variables, FAILURE: failure },
        stdout: "pipe",
        stderr: "pipe",
    });
    const [output, errors, code] = await Promise.all([
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
        child.exited,
    ]);

    // set aside the host's times, ids and stacks
    const reported =
        failure === "exception"
            ? "RangeError: note index out of range"
            : "TypeError: note sync rejected";

    return {
        code,
        exports: output
            .split("\n")
            .filter((line) => line !== "")
            .map((line) => hostless(JSON.parse(line))),
        isReported: errors.split("\n").some((line) => line.trim() === reported),
    };
}

/** The keys of the host's times and generated ids, which change on every run. */
const HOST_KEYS = new Set(["timeUnixNano", "observedTimeUnixNano", "traceId", "spanId"]);

/** The attributes whose values the host generates: stack traces and session ids. */
const HOST_ATTRIBUTES = new Set(["exception.stacktrace", "session.id"]);

/** Set aside the parts of an export the host generates, keeping every other field as written. */
function hostless(value: unknown): unknown {
    // keep scalars, and walk arrays
    if (typeof value !== "object" || value === null) {
        return value;
    } else if (Array.isArray(value)) {
        return value.map(hostless);
    }
    // mark a generated attribute's value
    else if ("key" in value && HOST_ATTRIBUTES.has(String(value.key))) {
        return { ...value, value: "<host>" };
    }
    // mark the generated times and ids of an object, walking the rest
    else {
        return Object.fromEntries(
            Object.entries(value).map(([key, field]) => [
                key,
                HOST_KEYS.has(key) ? "<host>" : hostless(field),
            ]),
        );
    }
}

test("export a process's uncaught exception and unhandled rejection with its release, then end it as it would have ended", async () => {
    expect([await fail("exception"), await fail("rejection")]).toMatchInlineSnapshot(`
      [
        {
          "code": 1,
          "exports": [
            {
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
                              "key": "error.type",
                              "value": {
                                "stringValue": "RangeError",
                              },
                            },
                            {
                              "key": "exception.type",
                              "value": {
                                "stringValue": "RangeError",
                              },
                            },
                            {
                              "key": "exception.message",
                              "value": {
                                "stringValue": "note index out of range",
                              },
                            },
                            {
                              "key": "exception.stacktrace",
                              "value": "<host>",
                            },
                            {
                              "key": "exception.escaped",
                              "value": {
                                "boolValue": true,
                              },
                            },
                          ],
                          "body": {},
                          "droppedAttributesCount": 0,
                          "eventName": "exception",
                          "observedTimeUnixNano": "<host>",
                          "severityNumber": 17,
                          "severityText": "ERROR",
                          "timeUnixNano": "<host>",
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
          ],
          "isReported": true,
        },
        {
          "code": 1,
          "exports": [
            {
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
                              "key": "error.type",
                              "value": {
                                "stringValue": "TypeError",
                              },
                            },
                            {
                              "key": "exception.type",
                              "value": {
                                "stringValue": "TypeError",
                              },
                            },
                            {
                              "key": "exception.message",
                              "value": {
                                "stringValue": "note sync rejected",
                              },
                            },
                            {
                              "key": "exception.stacktrace",
                              "value": "<host>",
                            },
                            {
                              "key": "exception.escaped",
                              "value": {
                                "boolValue": true,
                              },
                            },
                          ],
                          "body": {},
                          "droppedAttributesCount": 0,
                          "eventName": "exception",
                          "observedTimeUnixNano": "<host>",
                          "severityNumber": 17,
                          "severityText": "ERROR",
                          "timeUnixNano": "<host>",
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
          ],
          "isReported": true,
        },
      ]
    `);
}, 20_000);

/** The metrics of an OTLP/JSON metrics export, as far as a process's CPU time reads them. */
const MetricsExport = schema.looseObject({
    resourceMetrics: schema.array(
        schema.looseObject({
            scopeMetrics: schema.array(
                schema.looseObject({
                    metrics: schema.array(
                        schema.looseObject({
                            name: schema.string(),
                            unit: schema.string(),
                            sum: schema.looseObject({
                                aggregationTemporality: schema.number(),
                                dataPoints: schema.array(
                                    schema.looseObject({
                                        attributes: schema.array(
                                            schema.looseObject({
                                                key: schema.string(),
                                                value: schema.looseObject({
                                                    stringValue: schema.string(),
                                                }),
                                            }),
                                        ),
                                        asDouble: schema.number(),
                                    }),
                                ),
                            }),
                        }),
                    ),
                }),
            ),
        }),
    ),
});

test("export the process's user and system CPU seconds as a delta process.cpu.time sum", async () => {
    // observe the process and export its metrics once
    const bodies: string[] = [];
    const source = { name: "@example/notes", version: "2026.9.0" };
    const exporter = new OtlpExporter(
        async (signal, body) => {
            if (signal === "metrics") {
                bodies.push(new TextDecoder().decode(body));
            }
        },
        () => {},
    );
    await using simulation = new Simulation({ seed: 1, start: START });
    const telemetry = new Telemetry(
        exporter.options(source),
        new AsyncLocalStorageContextManager(),
        simulation.environment("test"),
    );
    observeProcess(telemetry, source, testHost());
    await telemetry.flush();
    await telemetry.shutdown();

    // read the exported sum with its unit, temporality, modes and the user time used
    const exported = MetricsExport.parse(JSON.parse(present(bodies[0], "a metrics export")));
    const metric = exported.resourceMetrics
        .flatMap((group) => group.scopeMetrics)
        .flatMap((scoped) => scoped.metrics)
        .find((each) => each.name === "process.cpu.time");
    const { unit, sum } = present(metric, "the process.cpu.time metric");
    expect({
        unit,
        temporality: sum.aggregationTemporality,
        modes: sum.dataPoints.map((point) =>
            point.attributes.map((each) => [each.key, each.value.stringValue]),
        ),
        isUsed: present(sum.dataPoints[0], "the user point").asDouble > 0,
    }).toEqual({
        unit: "s",
        temporality: 1,
        modes: [[["cpu.mode", "user"]], [["cpu.mode", "system"]]],
        isUsed: true,
    });
});
