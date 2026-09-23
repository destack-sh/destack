import type { LogRecordExporter, ReadableLogRecord } from "@destack/telemetry/log";
import { type ExportResult, ExportResultCode } from "@destack/telemetry/sdk";

/** Export host log records as newline-delimited JSON. */
export class LogExporter implements LogRecordExporter {
    /** Write complete structured records to the daemon's output. */
    export(records: ReadableLogRecord[], complete: (result: ExportResult) => void): void {
        try {
            for (const record of records) {
                console.log(
                    JSON.stringify({
                        signal: "log",
                        time: record.hrTime,
                        severity: record.severityText,
                        body: record.body,
                        attributes: record.attributes,
                        scope: record.instrumentationScope,
                        traceId: record.spanContext?.traceId,
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
