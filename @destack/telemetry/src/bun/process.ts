import { SeverityNumber } from "@opentelemetry/api";
import { type ExportResult, ExportResultCode } from "@opentelemetry/core";
import {
    type LogRecordExporter,
    type LogRecordProcessor,
    type ReadableLogRecord,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";

/** Exports a process's log records as lines to its standard error, its own local log. */
export class ProcessExporter {
    /** Write one line to the process's standard error. */
    readonly #write: (line: string) => void;
    /** The processor writing each log record as it is emitted. */
    readonly processor: LogRecordProcessor;

    /** Export through a writer of the process's standard error. */
    constructor(write: (line: string) => void) {
        this.#write = write;
        this.processor = new SimpleLogRecordProcessor({ exporter: this.logs });
    }

    /** The log record exporter, writing each record as one line, an exception with its stack. */
    readonly logs: LogRecordExporter = {
        export: (records: ReadableLogRecord[], done: (result: ExportResult) => void) => {
            for (const record of records) {
                this.#write(`${ProcessExporter.line(record)}\n`);
            }
            done({ code: ExportResultCode.SUCCESS });
        },
        shutdown: () => Promise.resolve(),
        forceFlush: () => Promise.resolve(),
    };

    /** Format a record as a line: its severity, its name, and any exception with its stack. */
    static line(record: ReadableLogRecord): string {
        // name the record by its event, else its body
        const name = record.eventName ?? textOf(record.body);
        const severity = record.severityText ?? SeverityNumber[record.severityNumber ?? 0];

        // add an exception's type, message and stack
        const { attributes } = record;
        const message = attributes["exception.message"];
        const stack = attributes["exception.stacktrace"];
        const type = textOf(attributes["exception.type"] ?? "Error");
        const thrown = message === undefined ? "" : ` ${type}: ${textOf(message)}`;
        const trace = stack === undefined ? "" : `\n${textOf(stack)}`;

        return `${severity} ${name}${thrown}${trace}`;
    }
}

/** Read a log value as text: a string as it is, anything else as JSON. */
function textOf(value: unknown): string {
    return typeof value === "string" ? value : JSON.stringify(value);
}
