import {
    InMemoryLogRecordExporter,
    LoggerProvider,
    type ReadableLogRecord,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";
import { MeterProvider, MetricReader, type ResourceMetrics } from "@opentelemetry/sdk-metrics";
import {
    InMemorySpanExporter,
    type ReadableSpan,
    SimpleSpanProcessor,
    TracerProvider,
} from "@opentelemetry/sdk-trace";
import type { ContextManager, TextMapPropagator } from "@opentelemetry/api";
import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import type { Clock } from "../clock/clock.ts";
import type { Random } from "../random/random.ts";
import { TraceIdGenerator } from "./generator.ts";
import { type Telemetry, W3C_PROPAGATOR } from "./telemetry.ts";

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
    /** The ended spans. */
    readonly #spans = new InMemorySpanExporter();
    /** The emitted log records. */
    readonly #logs = new InMemoryLogRecordExporter();
    /** The reader collecting the metrics. */
    readonly #metrics = new CollectedMetrics();

    /** Record a world's telemetry in memory, drawing identifiers from its clock and randomness. */
    constructor(environment: { readonly clock: Clock; readonly random: Random }) {
        // keep the clock, and record every span, record and metric in memory
        this.clock = environment.clock;
        this.tracers = new TracerProvider({
            idGenerator: new TraceIdGenerator(environment),
            spanProcessors: [new SimpleSpanProcessor({ exporter: this.#spans })],
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
}
