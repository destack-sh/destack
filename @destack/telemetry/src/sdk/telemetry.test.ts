import { testSimulation } from "@destack/test/world";
import { diag } from "@opentelemetry/api";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { nth } from "@destack/schema";
import { OtlpExporter } from "../otlp/index.ts";
import { startTelemetry } from "../bun/index.ts";

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
    await using simulation = testSimulation();
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
    await using simulation = testSimulation();
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

test("export spans and log records at a request's flush, and metrics only at the application's", async () => {
    // record a span, a log record and a counter on telemetry recording each signal it sends
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const sent: string[] = [];
    const exporter = new OtlpExporter(
        async (signal) => {
            sent.push(signal);
        },
        () => {},
    );
    const telemetry = await startTelemetry(exporter.options(source), environment);
    telemetry.tracers.getTracer(source.name).startSpan("answer").end();
    telemetry.loggers.getLogger(source.name).emit({ eventName: "answered" });
    telemetry.meters.getMeter(source.name).createCounter("answers").add(1);

    // send the records at the request's flush, then the metrics at the application's
    await telemetry.flush(["traces", "logs"]);
    const records = sent.splice(0).toSorted();
    await telemetry.flush();
    const flushed = sent.splice(0);
    await telemetry.shutdown();
    expect({ records, flushed }).toEqual({ records: ["logs", "traces"], flushed: ["metrics"] });
});

test("join flushes asked while one runs into the one queued behind it, a full flush covering fewer signals", async () => {
    // record a span, then hold the first export until released
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const sent: string[] = [];
    const held = Promise.withResolvers<void>();
    const exporter = new OtlpExporter(
        async (signal) => {
            sent.push(signal);
            if (sent.length === 1) {
                await held.promise;
            }
        },
        () => {},
    );
    const telemetry = await startTelemetry(exporter.options(source), environment);
    const span = () => telemetry.tracers.getTracer(source.name).startSpan("answer").end();
    span();

    // ask for flushes while the first exports, then release it
    const first = telemetry.flush(["traces", "logs"]);
    await simulation.scheduler.yield();
    span();
    const queued = [
        telemetry.flush(),
        telemetry.flush(["traces", "logs"]),
        telemetry.flush(["logs"]),
    ];
    held.resolve();
    await Promise.all([first, ...queued]);
    await telemetry.shutdown();

    // export the first span, then the second once, in the one full flush the others joined
    expect({ sent, isJoined: queued[1] === queued[0] && queued[2] === queued[0] }).toEqual({
        sent: ["traces", "traces"],
        isJoined: true,
    });
});
