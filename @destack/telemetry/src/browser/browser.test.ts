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
                                    value: schema.looseObject({ stringValue: schema.string() }),
                                }),
                            ),
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

test("export a page's uncaught errors and unhandled rejections to its origin at once, and its records as it hides", async () => {
    // deliver every export to the page's origin, waiting for each
    const delivered: { url: string; keepalive: boolean | undefined; body: unknown }[] = [];
    let arrived = Promise.withResolvers<void>();
    const send = async (url: string, options: RequestInit) => {
        delivered.push({
            url,
            keepalive: options.keepalive,
            body: JSON.parse(await new Response(options.body).text()),
        });
        arrived.resolve();

        return Response.json({});
    };
    const exported = async () => {
        await arrived.promise;
        arrived = Promise.withResolvers<void>();

        // read each record's event, severity and string attributes, and the resource's
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
                                .map((attribute) => [attribute.key, attribute.value.stringValue]),
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
    const telemetry = await startTelemetry(
        exporter.options(source, { manifest: "b".repeat(64) }),
        page,
    );
    try {
        // fail with an uncaught error, then with an unhandled rejection
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

        const resource = [
            { key: "service.name", value: { stringValue: "@example/notes" } },
            { key: "service.version", value: { stringValue: "2026.9.0" } },
            { key: "destack.build.manifest", value: { stringValue: "b".repeat(64) } },
        ];
        expect([thrown, rejected, hidden]).toEqual([
            {
                url: "/.destack/telemetry/v1/logs",
                keepalive: true,
                resources: [resource],
                records: [
                    [
                        "exception",
                        "ERROR",
                        { "exception.type": "TypeError", "exception.message": "note is missing" },
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
                            "exception.type": "RangeError",
                            "exception.message": "note index out of range",
                        },
                    ],
                ],
            },
            {
                url: "/.destack/telemetry/v1/logs",
                keepalive: true,
                resources: [resource],
                records: [["note.saved", "INFO", { length: "short" }]],
            },
        ]);
    } finally {
        await telemetry.shutdown();
    }
});
