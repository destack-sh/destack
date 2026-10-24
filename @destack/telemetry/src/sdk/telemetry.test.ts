import { diag } from "@opentelemetry/api";
import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { nth } from "@destack/schema";
import { OtlpExporter } from "../otlp/index.ts";
import { startTelemetry } from "../bun/index.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

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
        (error) => reported.push(nth(error.message.split("\n"), 0)),
    ).options(source);
}

test("pass the SDK's warnings to every running telemetry, and to the rest once one shuts down", async () => {
    // warn while both run, once one stopped, and once none runs
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const [firstReported, secondReported]: string[][] = [[], []];
    const first = await startTelemetry(options(firstReported), environment);
    const second = await startTelemetry(options(secondReported), environment);
    diag.warn("both running");
    await first.shutdown();
    diag.warn("second running");
    await second.shutdown();
    diag.warn("none running");

    expect({ firstReported, secondReported }).toEqual({
        firstReported: ["both running"],
        secondReported: ["both running", "second running"],
    });
});

test("start telemetries side by side, share one shutdown between callers, and refuse flushing after", async () => {
    // start twice in one process, as two worlds do
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const first = await startTelemetry(options(), environment);
    const competing = await startTelemetry(options(), environment).then(
        async (started) => {
            await started.shutdown();

            return "started";
        },
        (error: unknown) => String(error),
    );

    // shut down once for every caller, and refuse flushing a stopped one
    const shutdowns = [first.shutdown(), first.shutdown()];
    await Promise.all(shutdowns);
    const flushed = await first.flush().then(
        () => "flushed",
        (error: unknown) => String(error),
    );
    const second = await startTelemetry(options(), environment);
    await second.shutdown();

    expect({ competing, isShared: shutdowns[0] === shutdowns[1], flushed }).toEqual({
        competing: "started",
        isShared: true,
        flushed: "Error: telemetry is shut down",
    });
});
