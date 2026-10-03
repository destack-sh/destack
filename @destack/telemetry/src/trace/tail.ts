import { type Context, SpanStatusCode, trace, TraceFlags } from "@opentelemetry/api";
import { SeverityNumber } from "@opentelemetry/api-logs";
import type { LogRecordProcessor, ReadWriteLogRecord } from "@opentelemetry/sdk-logs";
import type { ReadableSpan, Span, SpanProcessor } from "@opentelemetry/sdk-trace";
import * as telemetry from "../scope/scope.ts";
import type {} from "@destack/package/import-meta";

/** How long a local root runs before its trace counts as slow and is kept: a second. */
const SLOW_MILLISECONDS = 1000;

/** The most unsampled local traces pending at once: older ones are dropped beyond it. */
const PENDING_TRACES = 1000;

/** The unsampled traces dropped undecided because too many were pending. */
const evicted = telemetry
    .scope(import.meta.destack.package)
    .metric.counter("telemetry.tail.evicted", {
        unit: "{trace}",
        description: "Unsampled traces dropped undecided because too many were pending.",
        attributes: {},
    });

/** The unsampled spans and log records of one local trace, buffered until its local root ends. */
interface PendingTrace {
    /** The ended spans. */
    readonly spans: ReadableSpan[];
    /** The informational and debug log records. */
    readonly logs: ReadWriteLogRecord[];
    /** Whether a span failed. */
    isFailed: boolean;
}

/** Keeps unsampled local traces that failed or ran slow, and drops the rest when their root ends. */
export class TailSampler {
    /** The processor receiving kept spans. */
    readonly #spans: SpanProcessor;
    /** The processor receiving kept log records. */
    readonly #logs: LogRecordProcessor;
    /** How long a local root runs before its trace is kept, in milliseconds. */
    readonly #slowMilliseconds: number;
    /** The pending traces, by trace identifier, oldest first. */
    readonly #pending = new Map<string, PendingTrace>();

    /** The span processor: sampled spans pass, unsampled ones wait for their local root. */
    readonly spans: SpanProcessor = {
        onStart: (span: Span, context: Context) => this.#start(span, context),
        onEnd: (span: ReadableSpan) => this.#end(span),
        forceFlush: () => this.#spans.forceFlush(),
        shutdown: () => this.#spans.shutdown(),
    };

    /** The log record processor: warnings and sampled records pass, the rest wait with their trace. */
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
        // pass sampled spans, and buffer a trace from its unsampled local root
        const { traceId, traceFlags } = span.spanContext();
        if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#spans.onStart(span, context);
        } else if (!this.#pending.has(traceId)) {
            this.#pending.set(traceId, { spans: [], logs: [], isFailed: false });
            this.#evict();
        }
    }

    /** Pass a sampled span on, or buffer an unsampled one and decide its trace when its root ends. */
    #end(span: ReadableSpan): void {
        // pass sampled spans
        const { traceId, traceFlags } = span.spanContext();
        if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#spans.onEnd(span);

            return;
        }

        // buffer the span, noting a failure
        const pending = this.#pending.get(traceId);
        if (pending === undefined) {
            return;
        }
        pending.spans.push(span);
        pending.isFailed ||= span.status.code === SpanStatusCode.ERROR;

        // decide the trace once its local root ends: keep it when it failed or ran slow
        const isLocalRoot =
            span.parentSpanContext === undefined || span.parentSpanContext.isRemote === true;
        if (isLocalRoot) {
            this.#pending.delete(traceId);
            const milliseconds = span.duration[0] * 1000 + span.duration[1] / 1e6;
            if (pending.isFailed || milliseconds >= this.#slowMilliseconds) {
                this.#keep(pending);
            }
        }
    }

    /** Pass warnings and records outside pending traces on, and buffer the rest with their trace. */
    #emit(record: ReadWriteLogRecord, context?: Context): void {
        // pass warnings, records outside any trace and records of sampled traces
        const span =
            record.spanContext ??
            (context === undefined ? undefined : trace.getSpanContext(context));
        const isWarning =
            (record.severityNumber ?? SeverityNumber.UNSPECIFIED) >= SeverityNumber.WARN;
        const pending = span === undefined ? undefined : this.#pending.get(span.traceId);
        if (isWarning || span === undefined || (span.traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#logs.onEmit(record, context);
        }
        // buffer the rest with their trace, dropping those of traces already decided
        else if (pending !== undefined) {
            pending.logs.push(record);
        }
    }

    /** Pass a kept trace's spans on as sampled, then its log records. */
    #keep(pending: PendingTrace): void {
        for (const span of pending.spans) {
            this.#spans.onEnd(sampled(span));
        }
        for (const record of pending.logs) {
            this.#logs.onEmit(record);
        }
    }

    /** Drop the oldest pending traces beyond the bound, counting each. */
    #evict(): void {
        for (const traceId of this.#pending.keys()) {
            if (this.#pending.size <= PENDING_TRACES) {
                break;
            }
            this.#pending.delete(traceId);
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
