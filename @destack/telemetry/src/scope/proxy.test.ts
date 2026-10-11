import { afterEach, expect, test } from "@destack/test";
import { metrics, type Span, trace } from "@opentelemetry/api";
import { logs } from "@opentelemetry/api-logs";
import { MeterProvider, MetricReader } from "@opentelemetry/sdk-metrics";
import {
    InMemoryLogRecordExporter,
    LoggerProvider,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";
import {
    InMemorySpanExporter,
    SimpleSpanProcessor,
    TracerProvider,
} from "@opentelemetry/sdk-trace";
import { ProxyLogger, ProxyMeter, ProxyTracer } from "./proxy.ts";

/** A reader collecting a provider's metrics on demand. */
class CollectingReader extends MetricReader {
    /** Flush nothing, as collection is on demand. */
    protected override async onForceFlush(): Promise<void> {}

    /** Stop nothing, as collection is on demand. */
    protected override async onShutdown(): Promise<void> {}

    /** Read the sum of each counter by name. */
    async sums(): Promise<Record<string, number>> {
        const { resourceMetrics } = await this.collect();

        return Object.fromEntries(
            resourceMetrics.scopeMetrics.flatMap((scope) =>
                scope.metrics.map((metric) => [
                    metric.descriptor.name,
                    metric.dataPoints.reduce((sum, point) => sum + Number(point.value), 0),
                ]),
            ),
        );
    }
}

afterEach(() => {
    metrics.disable();
    trace.disable();
    logs.disable();
});

/** Register a meter provider globally with a reader collecting it. */
function registerMeter(): CollectingReader {
    const reader = new CollectingReader();
    metrics.setGlobalMeterProvider(new MeterProvider({ readers: [reader] }));

    return reader;
}

/** Register a tracer and a logger provider globally, each with an exporter keeping what it receives. */
function registerTracing(): { spans: InMemorySpanExporter; records: InMemoryLogRecordExporter } {
    const spans = new InMemorySpanExporter();
    const records = new InMemoryLogRecordExporter();
    trace.setGlobalTracerProvider(
        new TracerProvider({ spanProcessors: [new SimpleSpanProcessor({ exporter: spans })] }),
    );
    logs.setGlobalLoggerProvider(
        new LoggerProvider({
            processors: [new SimpleLogRecordProcessor({ exporter: records })],
        }),
    );

    return { spans, records };
}

test("record on the provider that registers after a package created its counter", async () => {
    // create the counter before any provider and count once without one
    const counter = new ProxyMeter("@acme/notes", "1.0.0").createCounter("notes.saved");
    counter.add(1);

    // count twice once a provider registers
    const reader = registerMeter();
    counter.add(2);

    expect(await reader.sums()).toEqual({ "notes.saved": 2 });
});

test("record on the provider registered last, leaving the one it replaced", async () => {
    // count on a first provider, then on its replacement
    const counter = new ProxyMeter("@acme/notes", "1.0.0").createCounter("notes.saved");
    const first = registerMeter();
    counter.add(1);
    metrics.disable();
    const second = registerMeter();
    counter.add(5);

    expect([await first.sums(), await second.sums()]).toEqual([
        { "notes.saved": 1 },
        { "notes.saved": 5 },
    ]);
});

test("start spans and emit records on the providers registered last, leaving the ones they replaced", () => {
    // take a tracer and a logger, then register a first pair of providers
    const tracer = new ProxyTracer("@acme/notes", "1.0.0");
    const logger = new ProxyLogger("@acme/notes", "1.0.0");
    const first = registerTracing();
    tracer.startSpan("first").end();
    logger.emit({ eventName: "first" });

    // replace them, as a process starting telemetry again does
    trace.disable();
    logs.disable();
    const second = registerTracing();
    tracer.startActiveSpan("second", (span: Span) => span.end());
    logger.emit({ eventName: "second" });

    expect(
        [first, second].map(({ spans, records }) => [
            spans.getFinishedSpans().map((span) => span.name),
            records.getFinishedLogRecords().map((record) => record.eventName),
        ]),
    ).toEqual([
        [["first"], ["first"]],
        [["second"], ["second"]],
    ]);
});
