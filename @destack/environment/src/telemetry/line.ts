import { type Attributes, SeverityNumber } from "@opentelemetry/api";
import { type ExportResult, ExportResultCode } from "@opentelemetry/core";
import {
    type LogRecordExporter,
    type LogRecordProcessor,
    type ReadableLogRecord,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";

/** The width of the longest severity a line shows, ERROR. */
const SEVERITY_WIDTH = 5;

/** Exports log records as lines to a runtime's own log, such as standard error or a console. */
export class LineExporter {
    /** Write one line at its record's severity. */
    readonly #write: (line: string, severity: SeverityNumber) => void;
    /** The processor writing each log record as it is emitted. */
    readonly processor: LogRecordProcessor;

    /** Export through a writer of the runtime's own log. */
    constructor(write: (line: string, severity: SeverityNumber) => void) {
        this.#write = write;
        this.processor = new SimpleLogRecordProcessor({ exporter: this.logs });
    }

    /** The log record exporter, writing each record as one line, an exception with its stack. */
    readonly logs: LogRecordExporter = {
        export: (records: ReadableLogRecord[], done: (result: ExportResult) => void) => {
            for (const record of records) {
                const severity = record.severityNumber ?? SeverityNumber.UNSPECIFIED;
                this.#write(`${LineExporter.line(record)}\n`, severity);
            }
            done({ code: ExportResultCode.SUCCESS });
        },
        shutdown: () => Promise.resolve(),
        forceFlush: () => Promise.resolve(),
    };

    /** Write a line to the runtime's console, an error's as an error, on any runtime. */
    static console(line: string, severity: SeverityNumber): void {
        const text = line.trimEnd();
        if (severity >= SeverityNumber.ERROR) {
            console.error(text);
        }
        // write any other record as a log
        else {
            console.log(text);
        }
    }

    /** Format a record as a line: time, severity, name, attributes, and any exception. */
    static line(record: ReadableLogRecord): string {
        // read the record's name, severity and time
        const name = record.eventName ?? textOf(record.body);
        const severity = String(record.severityText ?? SeverityNumber[record.severityNumber ?? 0]);
        const [seconds] = record.hrTime;

        return LineExporter.format(seconds * 1000, severity, name, record.attributes);
    }

    /** Format a line: time of day, severity, name, attributes as key=value, and any exception. */
    static format(at: number, severity: string, name: string, attributes: Attributes): string {
        // stamp the time of day and list the attributes beside the exception's, quoting text with spaces
        const time = new Date(at).toISOString().slice(11, 19);
        const pairs = Object.entries(attributes)
            .filter(([key]) => !key.startsWith("exception."))
            .map(([key, value]) => `${key}=${valueOf(value)}`);
        const listed = pairs.length === 0 ? "" : ` ${pairs.join(" ")}`;

        // add an exception's type, message and stack
        const message = attributes["exception.message"];
        const stack = attributes["exception.stacktrace"];
        const type = textOf(attributes["exception.type"] ?? "Error");
        const thrown = message === undefined ? "" : ` ${type}: ${textOf(message)}`;
        const trace = stack === undefined ? "" : `\n${textOf(stack)}`;

        return `${time} ${severity.padEnd(SEVERITY_WIDTH)} ${name}${listed}${thrown}${trace}`;
    }
}

/** Read an attribute's value as a line shows it, quoting text with spaces or quotes. */
function valueOf(value: unknown): string {
    const text = textOf(value);

    return typeof value === "string" && /[\s"=]/u.test(value) ? JSON.stringify(value) : text;
}

/** Read a log value as text: a string as it is, anything else as JSON. */
function textOf(value: unknown): string {
    return typeof value === "string" ? value : JSON.stringify(value);
}
