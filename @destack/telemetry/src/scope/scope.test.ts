import { expect, expectTypeOf, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { startTelemetry } from "../bun/index.ts";
import { OtlpExporter } from "../otlp/index.ts";
import { markCaptured } from "./scope.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** The metrics of an export request, by name. */
const Exported = schema.looseObject({
    resourceMetrics: schema.array(
        schema.looseObject({
            scopeMetrics: schema.array(
                schema.looseObject({
                    metrics: schema.array(
                        schema.looseObject({
                            name: schema.string(),
                            unit: schema.string(),
                            sum: schema
                                .looseObject({
                                    dataPoints: schema.array(
                                        schema.looseObject({
                                            attributes: schema.array(schema.unknown()),
                                            asDouble: schema.number().exactOptional(),
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

test("declare metrics whose attributes take only their declared values, and export them with their unit", async () => {
    // collect the metrics each export carries
    const metrics: unknown[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            if (signal === "metrics") {
                const request = Exported.parse(JSON.parse(new TextDecoder().decode(body)));
                for (const group of request.resourceMetrics) {
                    for (const scope of group.scopeMetrics) {
                        metrics.push(
                            ...scope.metrics.map((metric) => ({
                                name: metric.name,
                                unit: metric.unit,
                                points: metric.sum?.dataPoints.map((point) => [
                                    point.attributes,
                                    point.asDouble,
                                ]),
                            })),
                        );
                    }
                }
            }
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source));

    // count saves by notebook, refusing undeclared values and attributes at compile time
    try {
        const saves = telemetry.scope(source).metric.counter("note.saves", {
            unit: "{save}",
            attributes: { notebook: ["personal", "shared"] },
        });
        saves.add(2, { notebook: "shared" });
        expectTypeOf<{ notebook: "shared" }>().toExtend<Parameters<typeof saves.add>[1]>();
        expectTypeOf<{ notebook: "archive" }>().not.toExtend<Parameters<typeof saves.add>[1]>();
        expectTypeOf<{ folder: "inbox" }>().not.toExtend<Parameters<typeof saves.add>[1]>();
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // export the counter with its unit and the declared attribute
    expect(metrics).toEqual([
        {
            name: "note.saves",
            unit: "{save}",
            points: [[[{ key: "notebook", value: { stringValue: "shared" } }], 2]],
        },
    ]);
});

/** The log records of an export request with their attributes. */
const ExportedLogs = schema.looseObject({
    resourceLogs: schema.array(
        schema.looseObject({
            scopeLogs: schema.array(
                schema.looseObject({
                    logRecords: schema.array(
                        schema.looseObject({
                            eventName: schema.string(),
                            severityText: schema.string().exactOptional(),
                            attributes: schema.array(
                                schema.object({
                                    key: schema.string(),
                                    value: schema.looseObject({
                                        stringValue: schema.string().exactOptional(),
                                        boolValue: schema.boolean().exactOptional(),
                                        arrayValue: schema.unknown().exactOptional(),
                                    }),
                                }),
                            ),
                        }),
                    ),
                }),
            ),
        }),
    ),
});

test("capture handled failures and messages as exceptions with their error type, level, tags and fingerprint, each failure once", async () => {
    // collect the records each export carries, without stack traces
    const records: unknown[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            if (signal === "logs") {
                const request = ExportedLogs.parse(JSON.parse(new TextDecoder().decode(body)));
                for (const record of request.resourceLogs.flatMap((group) =>
                    group.scopeLogs.flatMap((scope) => scope.logRecords),
                )) {
                    records.push([
                        record.eventName,
                        record.severityText,
                        Object.fromEntries(
                            record.attributes
                                .filter((attribute) => attribute.key !== "exception.stacktrace")
                                .map((attribute) => [
                                    attribute.key,
                                    attribute.value.stringValue ??
                                        attribute.value.boolValue ??
                                        attribute.value.arrayValue,
                                ]),
                        ),
                    ]);
                }
            }
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source));

    // capture a handled failure with a tag, a refusal of its caller, and a warning message with its own fingerprint
    try {
        const scope = telemetry.scope(source);
        const failure = new RangeError("page 9 of 3");
        scope.captureException(failure, { tags: { feature: "export" } });

        // capture the failure again where it escapes, and a failure its answering service captured
        scope.captureException(failure, { isEscaped: true });
        const answered = new Error("internal server error");
        markCaptured(answered);
        scope.captureException(answered, { isEscaped: true });
        scope.captureException(
            Object.assign(new Error("note is gone"), {
                toServiceError: () => ({ code: "NOT_FOUND" as const, message: "note is gone" }),
            }),
        );
        scope.captureMessage("quota nearly reached", { level: "warning", fingerprint: ["quota"] });
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // record each failure once as an exception that did not escape, at its level, the refusal typed by its service code
    expect(records).toEqual([
        [
            "exception",
            "ERROR",
            {
                "error.type": "RangeError",
                "exception.type": "RangeError",
                "exception.message": "page 9 of 3",
                "exception.escaped": false,
                "tag.feature": "export",
            },
        ],
        [
            "exception",
            "ERROR",
            {
                "error.type": "NOT_FOUND",
                "exception.type": "Error",
                "exception.message": "note is gone",
                "exception.escaped": false,
            },
        ],
        [
            "exception",
            "WARN",
            {
                "exception.message": "quota nearly reached",
                "exception.escaped": false,
                "exception.fingerprint": { values: [{ stringValue: "quota" }] },
            },
        ],
    ]);
});

test("record visits and actions as the platform's destack.visit and destack.action records, refusing a log record under its names", async () => {
    // collect each export's records with their string attributes
    const records: unknown[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            if (signal === "logs") {
                const request = ExportedLogs.parse(JSON.parse(new TextDecoder().decode(body)));
                for (const record of request.resourceLogs.flatMap((group) =>
                    group.scopeLogs.flatMap((scope) => scope.logRecords),
                )) {
                    records.push([
                        record.eventName,
                        Object.fromEntries(
                            record.attributes.map((attribute) => [
                                attribute.key,
                                attribute.value.stringValue,
                            ]),
                        ),
                    ]);
                }
            }
        },
        (error) => {
            throw error;
        },
    );
    const telemetry = await startTelemetry(exporter.options(source));

    // record a visit and an action, then try a log record under the platform's names
    let refused: unknown;
    try {
        const scope = telemetry.scope(source);
        scope.visit("/notes/:id", { path: "/notes/1", title: "Groceries" });
        scope.track("note.shared", { audience: "space" });
        try {
            scope.log.info("destack.visit");
        } catch (error) {
            refused = error instanceof Error ? error.message : error;
        }
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // keep the route, path and title of the visit, the action's name beside its properties, and refuse the log record
    expect({ records, refused }).toEqual({
        records: [
            [
                "destack.visit",
                {
                    "url.template": "/notes/:id",
                    "url.path": "/notes/1",
                    "destack.title": "Groceries",
                },
            ],
            ["destack.action", { audience: "space", "destack.action.name": "note.shared" }],
        ],
        refused: "log record names under destack. are reserved: destack.visit",
    });
});
