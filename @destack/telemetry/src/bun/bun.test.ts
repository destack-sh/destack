import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { OtlpExporter } from "../otlp/index.ts";
import { Telemetry } from "../sdk/index.ts";
import { observeProcess } from "./bun.ts";

/** The package directory, whose preload a child process runs with. */
const root = fileURLToPath(new URL("../..", import.meta.url));

/** A process that writes each logs export to its output and fails as its environment asks. */
const failing = `
import { startTelemetry } from ${JSON.stringify(fileURLToPath(new URL("index.ts", import.meta.url)))};
import { OtlpExporter } from ${JSON.stringify(fileURLToPath(new URL("../otlp/index.ts", import.meta.url)))};

const exporter = new OtlpExporter(async (signal, body) => {
    if (signal === "logs") {
        process.stdout.write(new TextDecoder().decode(body) + "\\n");
    }
}, () => {});
await startTelemetry(exporter.options({ name: "@example/notes", version: "2026.9.0" }));
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
    const child = spawn(process.execPath, ["-e", failing], {
        cwd: root,
        env: { ...process.env, FAILURE: failure },
    });
    let output = "";
    let errors = "";
    child.stdout.on("data", (chunk: Buffer) => (output += chunk.toString()));
    child.stderr.on("data", (chunk: Buffer) => (errors += chunk.toString()));
    const code = await new Promise<number | null>((resolve) => {
        child.on("close", resolve);
    });

    return {
        code,
        exports: output
            .split("\n")
            .filter((line) => line !== "")
            .map((line): unknown => JSON.parse(line)),
        isReported: errors.includes(failure === "exception" ? "note index" : "note sync"),
    };
}

/** The resource of the release. */
const resource: unknown = expect.objectContaining({
    attributes: [
        { key: "service.name", value: { stringValue: "@example/notes" } },
        { key: "service.version", value: { stringValue: "2026.9.0" } },
    ],
});

/** The logs export of one captured failure of the release. */
function captured(type: string, message: string) {
    const attributes: unknown = expect.arrayContaining([
        { key: "exception.type", value: { stringValue: type } },
        { key: "exception.message", value: { stringValue: message } },
    ]);

    return {
        resourceLogs: [
            {
                resource,
                scopeLogs: [
                    {
                        scope: { name: "@example/notes", version: "2026.9.0" },
                        logRecords: [
                            expect.objectContaining({
                                severityNumber: 17,
                                severityText: "ERROR",
                                eventName: "exception",
                                attributes,
                            }),
                        ],
                    },
                ],
            },
        ],
    };
}

test("export a process's uncaught exception and unhandled rejection with its release, then end it as it would have ended", async () => {
    expect([await fail("exception"), await fail("rejection")]).toEqual([
        {
            code: 1,
            exports: [captured("RangeError", "note index out of range")],
            isReported: true,
        },
        { code: 1, exports: [captured("TypeError", "note sync rejected")], isReported: true },
    ]);
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
    const telemetry = new Telemetry(exporter.options(source));
    observeProcess(telemetry, source);
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
