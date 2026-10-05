import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { OtlpExporter, type OtlpSignal } from "../otlp/index.ts";
import { withTelemetry } from "./worker.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** A logs export, read down to each record's event name and string attributes. */
const LogsExport = schema.looseObject({
    resourceLogs: schema.array(
        schema.looseObject({
            scopeLogs: schema.array(
                schema.looseObject({
                    logRecords: schema.array(
                        schema.looseObject({
                            eventName: schema.string(),
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

test("record a handler's thrown error before throwing it on, exporting it before the invocation ends", async () => {
    // deliver every export, and keep the invocation's background work
    const delivered: { signal: OtlpSignal; body: unknown }[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            delivered.push({ signal, body: JSON.parse(new TextDecoder().decode(body)) });
        },
        (error) => {
            throw error;
        },
    );
    const background: Promise<unknown>[] = [];

    // run a handler that fails
    const handled = withTelemetry(
        exporter.options(source),
        () => {
            throw new TypeError("note is missing");
        },
        { waitUntil: (operation) => background.push(operation) },
    );
    await expect(handled).rejects.toThrow(new TypeError("note is missing"));
    await Promise.all(background);

    // export the failure as an exception record
    const records = delivered.flatMap(({ signal, body }) =>
        signal === "logs"
            ? LogsExport.parse(body).resourceLogs.flatMap((group) =>
                  group.scopeLogs.flatMap((scope) =>
                      scope.logRecords.map((record) => [
                          record.eventName,
                          record.attributes.find(({ key }) => key === "exception.message")?.value
                              .stringValue,
                      ]),
                  ),
              )
            : [],
    );
    expect(records).toEqual([["exception", "note is missing"]]);
});
