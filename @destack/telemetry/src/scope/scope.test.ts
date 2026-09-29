import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { startTelemetry } from "../host/index.ts";
import { OtlpExporter } from "../otlp/index.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "1.0.0",
};

/** The metrics of an export request, by name. */
type Exported = {
    resourceMetrics: {
        scopeMetrics: {
            metrics: {
                name: string;
                unit: string;
                sum?: { dataPoints: { attributes: unknown[]; asDouble?: number }[] };
            }[];
        }[];
    }[];
};

test("declare metrics whose attributes take only their declared values, and export them with their unit", async () => {
    // collect the metrics each export carries
    const metrics: unknown[] = [];
    const exporter = new OtlpExporter(
        async (signal, body) => {
            if (signal === "metrics") {
                const request = JSON.parse(new TextDecoder().decode(body)) as Exported;
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
        const _refused = () => {
            // @ts-expect-error an undeclared value
            saves.add(1, { notebook: "archive" });
            // @ts-expect-error an undeclared attribute
            saves.add(1, { folder: "inbox" });
        };
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
