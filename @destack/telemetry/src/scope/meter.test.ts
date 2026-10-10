import { afterEach, expect, test } from "@destack/test";
import { metrics } from "@opentelemetry/api";
import { MeterProvider, MetricReader } from "@opentelemetry/sdk-metrics";
import { ProxyMeter } from "./meter.ts";

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
});

/** Register a meter provider globally with a reader collecting it. */
function register(): CollectingReader {
    const reader = new CollectingReader();
    metrics.setGlobalMeterProvider(new MeterProvider({ readers: [reader] }));

    return reader;
}

test("record on the provider that registers after a package created its counter", async () => {
    // create the counter before any provider and count once without one
    const counter = new ProxyMeter("@acme/notes").createCounter("notes.saved");
    counter.add(1);

    // count twice once a provider registers
    const reader = register();
    counter.add(2);

    expect(await reader.sums()).toEqual({ "notes.saved": 2 });
});

test("record on the provider registered last, leaving the one it replaced", async () => {
    // count on a first provider, then on its replacement
    const counter = new ProxyMeter("@acme/notes").createCounter("notes.saved");
    const first = register();
    counter.add(1);
    metrics.disable();
    const second = register();
    counter.add(5);

    expect([await first.sums(), await second.sums()]).toEqual([
        { "notes.saved": 1 },
        { "notes.saved": 5 },
    ]);
});
