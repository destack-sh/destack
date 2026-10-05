import { metrics } from "@opentelemetry/api";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { aligned } from "@destack/schema";
import { OtlpExporter } from "../otlp/index.ts";
import { startTelemetry } from "../bun/index.ts";
import { Telemetry } from "./telemetry.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** Build options exporting nowhere and keeping each reported failure. */
function options(reported: string[] = []) {
    return new OtlpExporter(
        async () => {},
        (error) => reported.push(aligned(error.message.split("\n"), 0)),
    ).options(source);
}

test("register one application's providers, refuse a second until it shuts down, and refuse flushing after", async () => {
    // start once, and refuse another start while the first runs
    const reported: string[] = [];
    const first = await startTelemetry(options(reported));
    const isRegistered = metrics.getMeterProvider() === first.metrics;
    const competing = await startTelemetry(options()).then(
        () => "started",
        (error: unknown) => String(error),
    );

    // shut down once for every caller, release the globals, and refuse flushing a stopped one
    const shutdowns = [first.shutdown(), first.shutdown()];
    await Promise.all(shutdowns);
    const flushed = await first.flush().then(
        () => "flushed",
        (error: unknown) => String(error),
    );
    const second = await startTelemetry(options());
    await second.shutdown();

    expect({
        isRegistered,
        competing,
        isShared: shutdowns[0] === shutdowns[1],
        flushed,
        reported,
        isReleased: metrics.getMeterProvider() !== first.metrics,
    }).toEqual({
        isRegistered: true,
        competing: "Error: OpenTelemetry is already initialized in this application",
        isShared: true,
        flushed: "Error: telemetry is shut down",
        reported: ["Error: @opentelemetry/api: Attempted duplicate registration of API: context"],
        isReleased: true,
    });
});

test("leave process globals untouched by an isolated instance", async () => {
    const telemetry = new Telemetry(options());
    try {
        expect(metrics.getMeterProvider() === telemetry.metrics).toBe(false);
    } finally {
        await telemetry.shutdown();
    }
});
