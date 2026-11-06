import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Package } from "@destack/package";
import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { Telemetry } from "../sdk/index.ts";
import { TelemetryScope } from "../scope/scope.ts";
import { ProcessExporter } from "./process.ts";

/** The package whose records the process writes. */
const OWNER = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/notes",
    version: "2026.9.0",
});

test("write each record to the process's log as one line, an exception with its type, message and stack", async () => {
    // start telemetry writing its records to the process's log
    await using simulation = testSimulation();
    const lines: string[] = [];
    const exporter = new ProcessExporter((line) => lines.push(line));
    const telemetry = await Telemetry.start(
        {
            ...OWNER,
            signals: () => ({
                traces: { spanProcessors: [] },
                logs: { processors: [exporter.processor] },
            }),
            metrics: {},
            report: (error) => {
                throw error;
            },
        },
        new AsyncLocalStorageContextManager(),
        simulation.environment("process"),
    );

    // log an event and capture a failure
    try {
        const { log, captureException } = TelemetryScope.of(OWNER, telemetry);
        log.info("note.saved", { length: 5 });
        const failure = new TypeError("the note is gone");
        failure.stack = "TypeError: the note is gone\n    at save";
        captureException(failure);
    } finally {
        await telemetry.shutdown();
    }

    // write the event, then the failure with its stack
    expect(lines).toEqual([
        "INFO note.saved\n",
        "ERROR exception TypeError: the note is gone\nTypeError: the note is gone\n    at save\n",
    ]);
});
