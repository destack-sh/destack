import { type Context, SpanStatusCode, trace, TraceFlags } from "@opentelemetry/api";
import { SeverityNumber } from "@opentelemetry/api-logs";
import type { LogRecordProcessor, ReadWriteLogRecord } from "@opentelemetry/sdk-logs";
import type { ReadableSpan, Span, SpanProcessor } from "@opentelemetry/sdk-trace";
import { hrTimeToMilliseconds } from "@opentelemetry/core";
import * as telemetry from "../scope/scope.ts";
import type {} from "@destack/package/import-meta";

/** How long a local root runs before its trace counts as slow and is kept: a second. */
const SLOW_MILLISECONDS = 1000;

/** The most local traces with a running root that the sampler follows at once. */
const PENDING_TRACES = 1000;

/** The most spans or log records one unsampled trace buffers. */
const PENDING_SIGNALS = 1000;

/** The traces dropped because too many local roots ran at once. */
const evicted = telemetry
    .scope(import.meta.destack.package)
    .metric.counter("telemetry.tail.evicted", {
        unit: "{trace}",
        description: "Traces dropped because too many local roots ran at once.",
        attributes: {},
    });

/** The spans and log records of undecided traces dropped because their trace buffered too many. */
const dropped = telemetry
    .scope(import.meta.destack.package)
    .metric.counter("telemetry.tail.dropped", {
        unit: "{signal}",
        description: "Spans and log records of undecided traces dropped beyond each trace's bound.",
        attributes: { signal: ["span", "log"] },
    });

/** The unsampled spans and log records of one local trace, buffered until the trace is decided. */
interface PendingTrace {
    /** When the trace's local root started, in epoch milliseconds. */
    readonly startedAt: number;
    /** The ended spans. */
    readonly spans: ReadableSpan[];
    /** The informational and debug log records. */
    readonly logs: ReadWriteLogRecord[];
}

/** Keeps unsampled local traces that fail or run slow, and drops the rest when their root ends. */
export class TailSampler {
    /** The processor receiving kept spans. */
    readonly #spans: SpanProcessor;
    /** The processor receiving kept log records. */
    readonly #logs: LogRecordProcessor;
    /** How long a local root runs before its trace is kept, in milliseconds. */
    readonly #slowMilliseconds: number;
    /** The local traces whose root runs, oldest first: pending until decided, then kept. */
    readonly #traces = new Map<string, PendingTrace | "kept">();

    /** The span processor: sampled spans pass, unsampled ones wait for their trace's decision. */
    readonly spans: SpanProcessor = {
        onStart: (span: Span, context: Context) => this.#start(span, context),
        onEnd: (span: ReadableSpan) => this.#end(span),
        forceFlush: () => this.#spans.forceFlush(),
        shutdown: () => this.#spans.shutdown(),
    };

    /** The log record processor. */
    readonly logs: LogRecordProcessor = {
        onEmit: (record: ReadWriteLogRecord, context?: Context) => this.#emit(record, context),
        forceFlush: () => this.#logs.forceFlush(),
        shutdown: () => this.#logs.shutdown(),
    };

    /** Pass kept spans and log records to downstream processors. */
    constructor(
        spans: SpanProcessor,
        logs: LogRecordProcessor,
        slowMilliseconds = SLOW_MILLISECONDS,
    ) {
        this.#spans = spans;
        this.#logs = logs;
        this.#slowMilliseconds = slowMilliseconds;
    }

    /** Start buffering an unsampled local root's trace, and pass sampled spans on. */
    #start(span: Span, context: Context): void {
        // pass sampled spans
        const { traceId, traceFlags } = span.spanContext();
        if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#spans.onStart(span, context);
        }
        // buffer a trace from its unsampled local root
        else if (!this.#traces.has(traceId)) {
            const startedAt = hrTimeToMilliseconds(span.startTime);
            this.#traces.set(traceId, { startedAt, spans: [], logs: [] });
            this.#evict();
        }
    }

    /** Pass on a sampled span or one of a kept trace. */
    #end(span: ReadableSpan): void {
        // find the span's trace and whether it is the trace's local root
        const { traceId, traceFlags } = span.spanContext();
        const traced = this.#traces.get(traceId);
        const isLocalRoot =
            span.parentSpanContext === undefined || span.parentSpanContext.isRemote === true;
        const isFailed = span.status.code === SpanStatusCode.ERROR;
        // pass sampled spans
        if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#spans.onEnd(span);
        }
        // pass a kept trace's span, forgetting the trace once its root ends
        else if (traced === "kept") {
            this.#spans.onEnd(sampled(span));
            if (isLocalRoot) {
                this.#traces.delete(traceId);
            }
        }
        // decide a pending trace as its root ends: keep it when the root failed or ran slow
        else if (traced !== undefined && isLocalRoot) {
            this.#traces.delete(traceId);
            traced.spans.push(span);
            const milliseconds = hrTimeToMilliseconds(span.duration);
            if (isFailed || milliseconds >= this.#slowMilliseconds) {
                this.#pass(traced);
            }
        }
        // buffer another span of a pending trace, keeping the trace once it fails or runs slow
        else if (traced !== undefined) {
            this.#buffer(traced.spans, span, "span");
            const age = hrTimeToMilliseconds(span.endTime) - traced.startedAt;
            if (isFailed || age >= this.#slowMilliseconds) {
                this.#keep(traceId, traced);
            }
        }
    }

    /** Pass on warnings, records outside pending traces and records of kept traces. */
    #emit(record: ReadWriteLogRecord, context?: Context): void {
        // find the record's trace
        const span =
            record.spanContext ??
            (context === undefined ? undefined : trace.getSpanContext(context));
        const isWarning =
            (record.severityNumber ?? SeverityNumber.UNSPECIFIED) >= SeverityNumber.WARN;
        const traced = span === undefined ? undefined : this.#traces.get(span.traceId);
        // pass warnings, records outside any trace and records of sampled or kept traces
        if (
            isWarning ||
            span === undefined ||
            (span.traceFlags & TraceFlags.SAMPLED) !== 0 ||
            traced === "kept"
        ) {
            this.#logs.onEmit(record, context);
        }
        // buffer a pending trace's record, keeping the trace once it runs slow
        else if (traced !== undefined) {
            this.#buffer(traced.logs, record, "log");
            if (hrTimeToMilliseconds(record.hrTime) - traced.startedAt >= this.#slowMilliseconds) {
                this.#keep(span.traceId, traced);
            }
        }
    }

    /** Buffer a signal of a pending trace within its bound, counting one beyond it. */
    #buffer<Signal>(buffered: Signal[], signal: Signal, kind: "span" | "log"): void {
        if (buffered.length < PENDING_SIGNALS) {
            buffered.push(signal);
        } else {
            dropped.add(1, { signal: kind });
        }
    }

    /** Keep a pending trace whose root still runs, passing its buffer and its later signals. */
    #keep(traceId: string, pending: PendingTrace): void {
        this.#traces.set(traceId, "kept");
        this.#pass(pending);
    }

    /** Pass a kept trace's buffered spans on as sampled, followed by its log records. */
    #pass(pending: PendingTrace): void {
        for (const span of pending.spans) {
            this.#spans.onEnd(sampled(span));
        }
        for (const record of pending.logs) {
            this.#logs.onEmit(record);
        }
    }

    /** Drop the oldest traces beyond the bound, counting each. */
    #evict(): void {
        for (const traceId of this.#traces.keys()) {
            if (this.#traces.size <= PENDING_TRACES) {
                break;
            }
            this.#traces.delete(traceId);
            evicted.add(1);
        }
    }
}

/** View an ended span as sampled, so downstream processors export it. */
function sampled(span: ReadableSpan): ReadableSpan {
    // mark the context sampled
    const context = {
        ...span.spanContext(),
        traceFlags: span.spanContext().traceFlags | TraceFlags.SAMPLED,
    };

    // copy every readable field, keeping the parent only when the span has one
    const parent = span.parentSpanContext;
    const view: ReadableSpan = {
        name: span.name,
        kind: span.kind,
        spanContext: () => context,
        ...(parent === undefined ? {} : { parentSpanContext: parent }),
        startTime: span.startTime,
        endTime: span.endTime,
        status: span.status,
        attributes: span.attributes,
        links: span.links,
        events: span.events,
        duration: span.duration,
        ended: span.ended,
        resource: span.resource,
        instrumentationScope: span.instrumentationScope,
        droppedAttributesCount: span.droppedAttributesCount,
        droppedEventsCount: span.droppedEventsCount,
        droppedLinksCount: span.droppedLinksCount,
    };

    return view;
}
