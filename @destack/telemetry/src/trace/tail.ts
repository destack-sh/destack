import { type Context, SpanStatusCode, trace, TraceFlags } from "@opentelemetry/api";
import { SeverityNumber } from "@opentelemetry/api-logs";
import type { LogRecordProcessor, ReadWriteLogRecord } from "@opentelemetry/sdk-logs";
import type { ReadableSpan, Span, SpanProcessor } from "@opentelemetry/sdk-trace";
import * as telemetry from "../scope/scope.ts";
import type {} from "@destack/package/import-meta";

/** How long a local root runs before its trace counts as slow and is kept: a second. */
const SLOW_MILLISECONDS = 1000;

/** The most unsampled local traces held at once: older ones are dropped beyond it. */
const HELD_TRACES = 1000;

/** The unsampled traces dropped undecided because too many were held. */
const evicted = telemetry
    .scope(import.meta.destack.package)
    .metric.counter("telemetry.tail.evicted", {
        unit: "{trace}",
        description: "Unsampled traces dropped undecided because too many were held.",
        attributes: {},
    });

/** The unsampled spans and log records of one local trace, held until its local root ends. */
interface HeldTrace {
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
    /** The held traces, by trace identifier, oldest first. */
    readonly #held = new Map<string, HeldTrace>();

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

    /** Start holding an unsampled local root's trace, and pass sampled spans on. */
    #start(span: Span, context: Context): void {
        // pass sampled spans, and hold a trace from its unsampled local root
        const { traceId, traceFlags } = span.spanContext();
        if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#spans.onStart(span, context);
        } else if (!this.#held.has(traceId)) {
            this.#held.set(traceId, { spans: [], logs: [], isFailed: false });
            this.#evict();
        }
    }

    /** Pass a sampled span on, or hold an unsampled one and decide its trace when its root ends. */
    #end(span: ReadableSpan): void {
        // pass sampled spans
        const { traceId, traceFlags } = span.spanContext();
        if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#spans.onEnd(span);

            return;
        }

        // hold the span, noting a failure
        const held = this.#held.get(traceId);
        if (held === undefined) {
            return;
        }
        held.spans.push(span);
        held.isFailed ||= span.status.code === SpanStatusCode.ERROR;

        // decide the trace once its local root ends: keep it when it failed or ran slow
        const isLocalRoot =
            span.parentSpanContext === undefined || span.parentSpanContext.isRemote === true;
        if (isLocalRoot) {
            this.#held.delete(traceId);
            const milliseconds = span.duration[0] * 1000 + span.duration[1] / 1e6;
            if (held.isFailed || milliseconds >= this.#slowMilliseconds) {
                this.#keep(held);
            }
        }
    }

    /** Pass warnings and records outside held traces on, and hold the rest with their trace. */
    #emit(record: ReadWriteLogRecord, context?: Context): void {
        // pass warnings, records outside any trace and records of sampled traces
        const span =
            record.spanContext ??
            (context === undefined ? undefined : trace.getSpanContext(context));
        const isWarning = (record.severityNumber ?? 0) >= SeverityNumber.WARN;
        const held = span === undefined ? undefined : this.#held.get(span.traceId);
        if (isWarning || span === undefined || (span.traceFlags & TraceFlags.SAMPLED) !== 0) {
            this.#logs.onEmit(record, context);
        }
        // hold the rest with their trace, dropping those of traces already decided
        else if (held !== undefined) {
            held.logs.push(record);
        }
    }

    /** Pass a kept trace's spans on as sampled, then its log records. */
    #keep(held: HeldTrace): void {
        for (const span of held.spans) {
            this.#spans.onEnd(sampled(span));
        }
        for (const record of held.logs) {
            this.#logs.onEmit(record);
        }
    }

    /** Drop the oldest held traces beyond the bound, counting each. */
    #evict(): void {
        for (const traceId of this.#held.keys()) {
            if (this.#held.size <= HELD_TRACES) {
                break;
            }
            this.#held.delete(traceId);
            evicted.add(1);
        }
    }
}

/** View an ended span as sampled, so downstream processors export it. */
function sampled(span: ReadableSpan): ReadableSpan {
    const context = {
        ...span.spanContext(),
        traceFlags: span.spanContext().traceFlags | TraceFlags.SAMPLED,
    };

    return Object.create(span, { spanContext: { value: () => context } }) as ReadableSpan;
}
