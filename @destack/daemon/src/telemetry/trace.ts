import type { ReadableSpan, SpanExporter } from "@destack/telemetry/trace";
import { type ExportResult, ExportResultCode } from "@destack/telemetry/sdk";

/** Export completed host spans as newline-delimited JSON. */
export class TraceExporter implements SpanExporter {
    /** Preserve operation timing, relationships, events, and errors. */
    export(spans: ReadableSpan[], complete: (result: ExportResult) => void): void {
        try {
            for (const span of spans) {
                console.log(
                    JSON.stringify({
                        signal: "trace",
                        name: span.name,
                        kind: span.kind,
                        context: span.spanContext(),
                        parent: span.parentSpanContext,
                        start: span.startTime,
                        duration: span.duration,
                        status: span.status,
                        attributes: span.attributes,
                        events: span.events,
                        links: span.links,
                        scope: span.instrumentationScope,
                    }),
                );
            }
            complete({ code: ExportResultCode.SUCCESS });
        } catch (error) {
            complete({
                code: ExportResultCode.FAILED,
                error: error instanceof Error ? error : new Error(String(error)),
            });
        }
    }

    /** Complete synchronous exports before shutdown. */
    shutdown(): Promise<void> {
        return Promise.resolve();
    }

    /** Complete synchronous exports before a flush returns. */
    forceFlush(): Promise<void> {
        return Promise.resolve();
    }
}
