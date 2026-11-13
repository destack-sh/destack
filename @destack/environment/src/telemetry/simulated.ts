import {
    InMemoryLogRecordExporter,
    LoggerProvider,
    type ReadableLogRecord,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";
import {
    type DataPoint,
    DataPointType,
    type Histogram,
    MeterProvider,
    MetricReader,
    type ResourceMetrics,
} from "@opentelemetry/sdk-metrics";
import {
    InMemorySpanExporter,
    type ReadableSpan,
    SimpleSpanProcessor,
    type Span,
    type SpanProcessor,
    TracerProvider,
} from "@opentelemetry/sdk-trace";
import type { ContextManager, TextMapPropagator } from "@opentelemetry/api";
import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Clock } from "../clock/clock.ts";
import type { Random } from "../random/random.ts";
import { TraceIdGenerator } from "./generator.ts";
import { type Telemetry, W3C_PROPAGATOR } from "./telemetry.ts";
import { SpanPath } from "./span.ts";

/** The span attribute counting the statements run inside a span, as the database totals them. */
const STATEMENTS = "db.client.operation.count";

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** One name path of the span tree: how many spans it has and their world and host time. */
interface SpanTotals {
    /** The spans on the path. */
    count: number;
    /** Their summed world milliseconds, as the simulated clock reads them. */
    world: number;
    /** Their summed host milliseconds, as the machine spent them. */
    host: number;
    /** The statements run inside them, nested spans included. */
    statements: number;
    /** The paths below, by span name. */
    readonly children: Map<string, SpanTotals>;
}

/** A span processor recording each span's host duration beside its simulated one. */
class HostTimes implements SpanProcessor {
    /** The host time each running span started at, by span. */
    readonly #started = new Map<string, number>();
    /** The host milliseconds each ended span took, by span. */
    readonly durations = new Map<string, number>();

    /** Note a span's host start. */
    onStart(span: Span): void {
        this.#started.set(span.spanContext().spanId, Clock.monotonic.now());
    }

    /** Record an ended span's host duration. */
    onEnd(span: ReadableSpan): void {
        const { spanId } = span.spanContext();
        const started = this.#started.get(spanId);
        if (started !== undefined) {
            this.durations.set(spanId, Clock.monotonic.now() - started);
            this.#started.delete(spanId);
        }
    }

    /** Flush nothing, as durations stay in memory. */
    forceFlush(): Promise<void> {
        return Promise.resolve();
    }

    /** Stop nothing, as durations stay in memory. */
    shutdown(): Promise<void> {
        return Promise.resolve();
    }
}

/** A metric reader the world collects from on demand, beside no timer. */
class CollectedMetrics extends MetricReader {
    /** Flush nothing, as collections happen on demand. */
    protected override async onForceFlush(): Promise<void> {}

    /** Stop nothing, as no timer runs. */
    protected override async onShutdown(): Promise<void> {}
}

/** The telemetry a world records in memory: every span, log record and metric its nodes emit. */
export class SimulatedTelemetry implements Telemetry {
    /** The tracers, recording each ended span. */
    readonly tracers: TracerProvider;
    /** The meters, collected on demand. */
    readonly meters: MeterProvider;
    /** The loggers, recording each record. */
    readonly loggers: LoggerProvider;
    /** The W3C propagator of trace context and baggage. */
    readonly propagator: TextMapPropagator = W3C_PROPAGATOR;
    /** The context active in each node's running work. */
    readonly context: ContextManager = new AsyncLocalStorageContextManager().enable();
    /** The world's clock, which the spans and log records are stamped on. */
    readonly clock: Clock;
    /** The host's clock, which durations read as the world's clock stands still for real work. */
    readonly monotonic: Pick<Clock, "now"> = Clock.monotonic;
    /** The ended spans. */
    readonly #spans = new InMemorySpanExporter();
    /** The emitted log records. */
    readonly #logs = new InMemoryLogRecordExporter();
    /** The reader collecting the metrics. */
    readonly #metrics = new CollectedMetrics();
    /** The host duration of every ended span. */
    readonly #hostTimes = new HostTimes();

    /** Record a world's telemetry in memory, drawing identifiers from its clock and randomness. */
    constructor(environment: { readonly clock: Clock; readonly random: Random }) {
        // keep the clock, and record every span, record and metric in memory
        this.clock = environment.clock;
        this.tracers = new TracerProvider({
            idGenerator: new TraceIdGenerator(environment),
            spanProcessors: [this.#hostTimes, new SimpleSpanProcessor({ exporter: this.#spans })],
        });
        this.meters = new MeterProvider({ readers: [this.#metrics] });
        this.loggers = new LoggerProvider({
            processors: [new SimpleLogRecordProcessor({ exporter: this.#logs })],
        });
    }

    /** The spans ended so far, oldest first. */
    get spans(): readonly ReadableSpan[] {
        return this.#spans.getFinishedSpans();
    }

    /** The log records emitted so far, oldest first. */
    get logs(): readonly ReadableLogRecord[] {
        return this.#logs.getFinishedLogRecords();
    }

    /** Collect the metrics recorded so far. */
    async metrics(): Promise<ResourceMetrics> {
        const { resourceMetrics } = await this.#metrics.collect();

        return resourceMetrics;
    }

    /** Describe the spans as a tree of name paths with world and host time, then histograms. */
    async describe(): Promise<string> {
        // sum the spans along their name paths from their roots
        const byId = new Map(this.spans.map((span) => [span.spanContext().spanId, span]));
        const root: SpanTotals = {
            count: 0,
            world: 0,
            host: 0,
            statements: 0,
            children: new Map(),
        };
        for (const span of this.spans) {
            const names: string[] = [];
            for (let current: ReadableSpan | undefined = span; current !== undefined;) {
                names.unshift(SpanPath.label(current.name, current.attributes));
                const parent: string | undefined = current.parentSpanContext?.spanId;
                current = parent === undefined ? undefined : byId.get(parent);
            }
            let path = root;
            for (const name of names) {
                let child = path.children.get(name);
                if (child === undefined) {
                    child = { count: 0, world: 0, host: 0, statements: 0, children: new Map() };
                    path.children.set(name, child);
                }
                path = child;
            }
            const [seconds, nanoseconds] = span.duration;
            path.count += 1;
            path.world += seconds * MILLISECONDS_PER_SECOND + nanoseconds / 1_000_000;
            path.host += this.#hostTimes.durations.get(span.spanContext().spanId) ?? 0;
            const statements = span.attributes[STATEMENTS];
            path.statements += typeof statements === "number" ? statements : 0;
        }

        // write the paths, the costliest first, then each histogram's count and summed milliseconds
        const lines = ["spans: count, world ms, host ms, statements"];
        SimulatedTelemetry.#write(root, 0, lines);
        lines.push("histograms: count, total, p50, p95, in ms for durations, by target");
        for (const { metrics } of (await this.metrics()).scopeMetrics) {
            for (const metric of metrics) {
                if (metric.dataPointType === DataPointType.HISTOGRAM) {
                    const scale = metric.descriptor.unit === "s" ? MILLISECONDS_PER_SECOND : 1;
                    SimulatedTelemetry.#writeHistogram(
                        metric.descriptor.name,
                        metric.dataPoints,
                        scale,
                        lines,
                    );
                }
            }
        }

        return lines.join("\n");
    }

    /** Write a histogram's totals, then each group of points by target. */
    static #writeHistogram(
        name: string,
        points: readonly DataPoint<Histogram>[],
        scale: number,
        lines: string[],
    ): void {
        // merge the points by target
        const groups = new Map<string, Histogram[]>();
        for (const point of points) {
            const named = SpanPath.naming(point.attributes);
            groups.set(named, [...(groups.get(named) ?? []), point.value]);
        }

        // write the whole, then the groups, the costliest first
        lines.push(`  ${name}  ${SimulatedTelemetry.#summary([...groups.values()].flat(), scale)}`);
        const ordered = [...groups].toSorted(([, left], [, right]) => summed(right) - summed(left));
        for (const [named, values] of groups.size > 1 ? ordered : []) {
            lines.push(
                `    ${named || "(unnamed)"}  ${SimulatedTelemetry.#summary(values, scale)}`,
            );
        }
    }

    /** Summarise histogram values: their count, total and estimated median and 95th percentile. */
    static #summary(values: readonly Histogram[], scale: number): string {
        // read the count and total, then estimate the quantiles from the buckets
        const count = values.reduce((sum, value) => sum + value.count, 0);
        const total = summed(values) * scale;
        const p50 = percentile(values, 0.5) * scale;
        const p95 = percentile(values, 0.95) * scale;

        return `${count}  ${total.toFixed(0)}  ${p50.toFixed(1)}  ${p95.toFixed(1)}`;
    }

    /** Write a path's children, the costliest by host time first, indented by depth. */
    static #write(path: SpanTotals, depth: number, lines: string[]): void {
        const children = [...path.children].toSorted(
            ([, left], [, right]) => right.host - left.host,
        );
        for (const [name, child] of children) {
            const indent = "  ".repeat(depth + 1);
            lines.push(
                `${indent}${name}  ×${child.count}  ${child.world.toFixed(0)}  ${child.host.toFixed(0)}  ${child.statements}`,
            );
            SimulatedTelemetry.#write(child, depth + 1, lines);
        }
    }

    /** Count each sum, and each histogram's records and sum, in all and by target. */
    async counts(): Promise<Record<string, number>> {
        // add each measurement under its key
        const counts: Record<string, number> = {};
        const add = (key: string, value: number) => {
            counts[key] = (counts[key] ?? 0) + value;
        };
        for (const { metrics } of (await this.metrics()).scopeMetrics) {
            for (const metric of metrics) {
                // count a histogram's records and sum, in all and by target
                const { name, unit } = metric.descriptor;
                if (metric.dataPointType === DataPointType.HISTOGRAM) {
                    const sum = unit === "s" ? "seconds" : "sum";
                    for (const point of metric.dataPoints) {
                        const named = SpanPath.naming(point.attributes);
                        add(`${name}.count`, point.value.count);
                        add(`${name}.${sum}`, point.value.sum ?? 0);
                        if (named !== "") {
                            add(`${name}.count.${named}`, point.value.count);
                            add(`${name}.${sum}.${named}`, point.value.sum ?? 0);
                        }
                    }
                }
                // add up a sum
                else if (metric.dataPointType === DataPointType.SUM) {
                    for (const point of metric.dataPoints) {
                        add(name, point.value);
                    }
                }
            }
        }

        return counts;
    }
}

/** Add up the sums of some histogram values. */
function summed(values: readonly Histogram[]): number {
    return values.reduce((sum, value) => sum + (value.sum ?? 0), 0);
}

/** Estimate a quantile of some histogram values by interpolating within their merged buckets. */
function percentile(values: readonly Histogram[], quantile: number): number {
    // merge the bucket counts, which share their boundaries
    const first = values[0];
    if (first === undefined) {
        return 0;
    }
    const { boundaries } = first.buckets;
    const counts = first.buckets.counts.map((_, index) =>
        values.reduce((sum, value) => sum + (value.buckets.counts[index] ?? 0), 0),
    );
    const count = counts.reduce((sum, each) => sum + each, 0);

    // find the bucket the quantile falls in, and interpolate between its bounds
    const target = quantile * count;
    let seen = 0;
    for (const [index, inBucket] of counts.entries()) {
        if (seen + inBucket >= target && inBucket > 0) {
            const lower = index === 0 ? 0 : (boundaries[index - 1] ?? 0);
            const upper =
                boundaries[index] ?? Math.max(...values.map((value) => value.max ?? lower));

            return lower + ((target - seen) / inBucket) * (upper - lower);
        }
        seen += inBucket;
    }

    return 0;
}
