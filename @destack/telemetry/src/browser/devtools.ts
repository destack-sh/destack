import type { Attributes } from "@opentelemetry/api";
import { type ExportResult, ExportResultCode } from "@opentelemetry/core";
import {
    type LogRecordExporter,
    type ReadableLogRecord,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";
import {
    type ReadableSpan,
    SimpleSpanProcessor,
    type SpanExporter,
} from "@opentelemetry/sdk-trace";
import type { TelemetryOptions } from "../sdk/index.ts";

/** The lowest OpenTelemetry severity numbers of warnings and errors. */
const SEVERITY = { warn: 13, error: 17 } as const;

/** Exports a browser's log records and spans to the developer tools console. */
export class DevtoolsExporter {
    /** The log record exporter, writing each record at its severity. */
    readonly logs: LogRecordExporter = {
        export: (records: ReadableLogRecord[], done: (result: ExportResult) => void) => {
            for (const record of records) {
                const body =
                    typeof record.body === "string" ? record.body : JSON.stringify(record.body);
                const name = record.eventName ?? body;
                const severity = record.severityNumber ?? 0;
                if (severity >= SEVERITY.error) {
                    console.error(name, record.attributes);
                } else if (severity >= SEVERITY.warn) {
                    console.warn(name, record.attributes);
                } else {
                    console.info(name, record.attributes);
                }
            }
            done({ code: ExportResultCode.SUCCESS });
        },
        shutdown: () => Promise.resolve(),
        forceFlush: () => Promise.resolve(),
    };

    /** The span exporter, writing each span with its duration in milliseconds. */
    readonly traces: SpanExporter = {
        export: (spans: ReadableSpan[], done: (result: ExportResult) => void) => {
            for (const span of spans) {
                const milliseconds = span.duration[0] * 1000 + span.duration[1] / 1e6;
                console.debug(`${span.name} ${milliseconds.toFixed(1)}ms`, span.attributes);
            }
            done({ code: ExportResultCode.SUCCESS });
        },
        shutdown: () => Promise.resolve(),
        forceFlush: () => Promise.resolve(),
    };

    /** Build telemetry options writing an owner's log records and spans to the console at once. */
    options(
        owner: { readonly name: string; readonly version: string },
        options: { readonly attributes?: Attributes } = {},
    ): TelemetryOptions {
        return {
            name: owner.name,
            version: owner.version,
            attributes: options.attributes ?? {},
            traces: { spanProcessors: [new SimpleSpanProcessor({ exporter: this.traces })] },
            logs: { processors: [new SimpleLogRecordProcessor({ exporter: this.logs })] },
            metrics: {},
            report: (error) => console.warn("telemetry failed:", error),
        };
    }
}
