import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { OtlpExporter } from "../otlp/index.ts";
import { startTelemetry } from "./browser.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** A logs export, read down to each record's event name and attributes. */
const LogsExport = schema.looseObject({
    resourceLogs: schema.array(
        schema.looseObject({
            resource: schema.looseObject({ attributes: schema.array(schema.unknown()) }),
            scopeLogs: schema.array(
                schema.looseObject({
                    logRecords: schema.array(
                        schema.looseObject({
                            eventName: schema.string(),
                            severityText: schema.string(),
                            attributes: schema.array(
                                schema.looseObject({
                                    key: schema.string(),
                                    value: schema.looseObject({
                                        stringValue: schema.string().exactOptional(),
                                        boolValue: schema.boolean().exactOptional(),
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

/** A metrics export of counters, read down to each data point's value and attributes. */
const SessionCount = schema.looseObject({
    resourceMetrics: schema.array(
        schema.looseObject({
            scopeMetrics: schema.array(
                schema.looseObject({
                    metrics: schema.array(
                        schema.looseObject({
                            name: schema.string(),
                            sum: schema.looseObject({
                                dataPoints: schema.array(
                                    schema.looseObject({
                                        asInt: schema.union([schema.string(), schema.number()]),
                                        attributes: schema.array(
                                            schema.looseObject({
                                                key: schema.string(),
                                                value: schema.looseObject({
                                                    stringValue: schema.string(),
                                                }),
                                            }),
                                        ),
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

/** A page's window, hidden or shown as a test sets it. */
class PageWindow extends EventTarget {
    /** The page's document and its visibility. */
    readonly document: { visibilityState: DocumentVisibilityState } = {
        visibilityState: "visible",
    };
}

test("export a page's uncaught errors and unhandled rejections to its origin at once, its records as it hides, and its crashed session as it leaves", async () => {
    // deliver every export to the page's origin, waiting for each, and collect the sessions records name
    const sessions = new Set<string>();
    const delivered: { url: string; keepalive: boolean | undefined; body: unknown }[] = [];
    let arrived = Promise.withResolvers<void>();
    const metrics: unknown[] = [];
    const send = async (url: string, options: RequestInit) => {
        // keep the metrics apart, and wake the reader for each logs export
        const body: unknown = JSON.parse(await new Response(options.body).text());
        if (url.endsWith("/v1/metrics")) {
            metrics.push(body);
        } else {
            delivered.push({ url, keepalive: options.keepalive, body });
            arrived.resolve();
        }

        return Response.json({});
    };
    const exported = async () => {
        await arrived.promise;
        arrived = Promise.withResolvers<void>();

        // read each record's event, severity and attributes, and the resource's
        const [request] = delivered.splice(0);
        const { resourceLogs } = LogsExport.parse(request?.body);

        return {
            url: request?.url,
            keepalive: request?.keepalive,
            resources: resourceLogs.map((group) => group.resource.attributes),
            records: resourceLogs.flatMap((group) =>
                group.scopeLogs.flatMap((scope) =>
                    scope.logRecords.map((record) => [
                        record.eventName,
                        record.severityText,
                        Object.fromEntries(
                            record.attributes
                                .filter((attribute) => attribute.key !== "exception.stacktrace")
                                .map((attribute) => {
                                    // name the session by its role, keeping its identifier
                                    const value =
                                        attribute.value.stringValue ?? attribute.value.boolValue;
                                    if (attribute.key === "session.id") {
                                        sessions.add(String(value));

                                        return [attribute.key, "session"];
                                    }

                                    return [attribute.key, value];
                                }),
                        ),
                    ]),
                ),
            ),
        };
    };

    // start the page's telemetry on its window
    const page = new PageWindow();
    const exporter = OtlpExporter.origin((error) => {
        throw error;
    }, send);
    const telemetry = await startTelemetry(exporter.options(source), page);
    try {
        // fail with an uncaught error and with an unhandled rejection
        page.dispatchEvent(new ErrorEvent("error", { error: new TypeError("note is missing") }));
        const thrown = await exported();
        page.dispatchEvent(
            Object.assign(new Event("unhandledrejection"), {
                reason: new RangeError("note index out of range"),
            }),
        );
        const rejected = await exported();

        // log a note, exported once the page hides
        telemetry.scope(source).log.info("note.saved", { length: "short" });
        page.document.visibilityState = "hidden";
        page.dispatchEvent(new Event("visibilitychange"));
        const hidden = await exported();

        // leave the page, ending its session
        page.dispatchEvent(new Event("pagehide"));
        const left = await exported();

        const resource = [
            { key: "service.name", value: { stringValue: "@example/notes" } },
            { key: "service.version", value: { stringValue: "2026.9.0" } },
        ];
        expect([thrown, rejected, hidden, left]).toEqual([
            {
                url: "/.destack/telemetry/v1/logs",
                keepalive: true,
                resources: [resource],
                records: [
                    ["session.start", "INFO", { "session.id": "session" }],
                    [
                        "exception",
                        "ERROR",
                        {
                            "error.type": "TypeError",
                            "exception.type": "TypeError",
                            "exception.message": "note is missing",
                            "exception.escaped": true,
                            "session.id": "session",
                        },
                    ],
                ],
            },
            {
                url: "/.destack/telemetry/v1/logs",
                keepalive: true,
                resources: [resource],
                records: [
                    [
                        "exception",
                        "ERROR",
                        {
                            "error.type": "RangeError",
                            "exception.type": "RangeError",
                            "exception.message": "note index out of range",
                            "exception.escaped": true,
                            "session.id": "session",
                        },
                    ],
                ],
            },
            {
                url: "/.destack/telemetry/v1/logs",
                keepalive: true,
                resources: [resource],
                records: [["note.saved", "INFO", { length: "short", "session.id": "session" }]],
            },
            {
                url: "/.destack/telemetry/v1/logs",
                keepalive: true,
                resources: [resource],
                records: [
                    [
                        "session.end",
                        "INFO",
                        { "session.status": "crashed", "session.id": "session" },
                    ],
                ],
            },
        ]);

        // name one session across the page's records, and count it as crashed once its metrics export
        expect(sessions.size).toBe(1);
        await expect.poll(() => metrics.length).toBe(1);
        expect(
            SessionCount.parse(metrics.at(-1)).resourceMetrics.flatMap((group) =>
                group.scopeMetrics.flatMap((scope) =>
                    scope.metrics.map((metric) => [
                        metric.name,
                        metric.sum.dataPoints.map((point) => [
                            Number(point.asInt),
                            point.attributes.map(({ key, value }) => [key, value.stringValue]),
                        ]),
                    ]),
                ),
            ),
        ).toEqual([["session.count", [[1, [["session.status", "crashed"]]]]]]);
    } finally {
        await telemetry.shutdown();
    }
});
