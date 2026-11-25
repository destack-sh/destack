import {
    type Attributes,
    type ContextManager,
    SeverityNumber,
    SpanStatusCode,
} from "@opentelemetry/api";
import { type ReadableSpan, type SpanProcessor, TracerProvider } from "@opentelemetry/sdk-trace";
import {
    SPAN_ELAPSED_ATTRIBUTE,
    SPAN_LEVEL_ATTRIBUTE,
    SPAN_OUTERMOST_ATTRIBUTE,
    SPAN_TOTALS_ATTRIBUTE,
} from "./runner.ts";
import { Level } from "./level.ts";
import { type ExportResult, ExportResultCode } from "@opentelemetry/core";
import {
    type LogRecordExporter,
    type LogRecordProcessor,
    LoggerProvider,
    type ReadableLogRecord,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";
import type { Clock } from "../clock/clock.ts";
import { Telemetry } from "./telemetry.ts";

/** The variable naming the lowest level a process prints locally, info when absent. */
export const LOG_LEVEL_VARIABLE = "DESTACK_LOG_LEVEL";

/** The variable naming how a process prints locally: line for a terminal, json for a collector. */
export const LOG_FORMAT_VARIABLE = "DESTACK_LOG_FORMAT";

/** The formats a process prints its records in: a readable line, or one JSON object per line. */
export const LOG_FORMATS = ["line", "json"] as const;

/** The milliseconds in a second, as span times count seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The nanoseconds in a millisecond, as span times count nanoseconds beside seconds. */
const NANOSECONDS_PER_MILLISECOND = 1_000_000;

/** The width of the longest severity a line shows, ERROR. */
const SEVERITY_WIDTH = 5;

/** How a line exporter prints: from which level, as readable lines or JSON. */
export interface LineExporterOptions {
    /** The lowest level printed. */
    readonly level: Level;
    /** The format each record prints in. */
    readonly format: LogFormat;
}

/** A format a process prints its records in. */
export type LogFormat = (typeof LOG_FORMATS)[number];

/** Exports log records as lines to a runtime's own log, such as standard error or a console. */
export class LineExporter {
    /** Write one line at its record's severity. */
    readonly #write: (line: string, severity: SeverityNumber) => void;
    /** The processor writing each log record as it is emitted. */
    readonly processor: LogRecordProcessor;

    /** The processor writing each info span as one line once it ends, with its duration. */
    readonly spans: SpanProcessor;

    /** The lowest level the log prints, info unless a process asks for more or less. */
    readonly #threshold: SeverityNumber;
    /** The format each record prints in. */
    readonly #format: LogFormat;

    /** Export through a writer of the runtime's own log, printing as the options ask. */
    constructor(
        write: (line: string, severity: SeverityNumber) => void,
        options: LineExporterOptions = LineExporter.options({}),
    ) {
        // keep the writer, the threshold and the format, and process records and spans as they end
        this.#write = write;
        [this.#threshold] = Level.severity(options.level);
        this.#format = options.format;
        this.processor = new SimpleLogRecordProcessor({ exporter: this.logs });
        this.spans = {
            onStart: () => {},
            onEnd: (span) => this.#end(span),
            forceFlush: () => Promise.resolve(),
            shutdown: () => Promise.resolve(),
        };
    }

    /** Make telemetry writing log records and info spans as lines through this exporter. */
    telemetry(clock: Clock, context: ContextManager): Telemetry {
        return {
            ...Telemetry.none(clock, context),
            tracers: new TracerProvider({ spanProcessors: [this.spans] }),
            loggers: new LoggerProvider({ processors: [this.processor] }),
            // write the spans another process ended, such as a compiler's, as lines too
            ingest: (spans) => {
                for (const span of spans) {
                    this.spans.onEnd(span);
                }
            },
        };
    }

    /** Write an ended span from the threshold on as a line, a failed one as an error. */
    #end(span: ReadableSpan): void {
        // leave out spans below the threshold, a span without a level being debug, as requests are
        const {
            [SPAN_LEVEL_ATTRIBUTE]: marked,
            [SPAN_TOTALS_ATTRIBUTE]: totals,
            [SPAN_OUTERMOST_ATTRIBUTE]: _outermost,
            [SPAN_ELAPSED_ATTRIBUTE]: _elapsed,
            ...attributed
        } = span.attributes;
        const counted = new Set(Array.isArray(totals) ? totals : []);
        const attributes = Object.fromEntries(
            Object.entries(attributed).filter(([key]) => !counted.has(key)),
        );
        const level = typeof marked === "string" ? Level.parse(marked) : "debug";
        if (Level.severity(level)[0] < this.#threshold) {
            return;
        }

        // write the span's end with its duration, trace, parent and links, a failed one as an error
        const isFailed = span.status.code === SpanStatusCode.ERROR;
        const { traceId, spanId } = span.spanContext();
        const parent = span.parentSpanContext?.spanId;
        const links = span.links.map((link) => `${link.context.traceId}-${link.context.spanId}`);
        const [seconds, nanoseconds] = span.endTime;
        const at = seconds * MILLISECONDS_PER_SECOND + nanoseconds / NANOSECONDS_PER_MILLISECOND;
        const [spent, spentNanoseconds] = span.duration;
        const duration = Math.round(
            spent * MILLISECONDS_PER_SECOND + spentNanoseconds / NANOSECONDS_PER_MILLISECOND,
        );
        const severity = isFailed ? SeverityNumber.ERROR : SeverityNumber.INFO;
        const line = this.#render(at, isFailed ? "ERROR" : "INFO", span.name, {
            ...attributes,
            ...(isFailed && span.status.message !== undefined
                ? { "error.message": span.status.message }
                : {}),
            duration_ms: duration,
            trace_id: traceId,
            span_id: spanId,
            ...(parent === undefined ? {} : { parent_span_id: parent }),
            ...(links.length === 0 ? {} : { links: links.join(",") }),
        });
        this.#write(`${line}\n`, severity);
    }

    /** The log record exporter, writing each record as one line, an exception with its stack. */
    readonly logs: LogRecordExporter = {
        export: (records: ReadableLogRecord[], done: (result: ExportResult) => void) => {
            for (const record of records) {
                // write each record from the threshold on
                const severity = record.severityNumber ?? SeverityNumber.UNSPECIFIED;
                if (severity >= this.#threshold) {
                    this.#write(`${this.#record(record)}\n`, severity);
                }
            }
            done({ code: ExportResultCode.SUCCESS });
        },
        shutdown: () => Promise.resolve(),
        forceFlush: () => Promise.resolve(),
    };

    /** Read a process's options from its variables: info and readable lines when absent. */
    static options(variables: Readonly<Record<string, string | undefined>>): LineExporterOptions {
        // read the level and the format, refusing any other format
        const level = Level.parse(variables[LOG_LEVEL_VARIABLE] ?? "info");
        const name = variables[LOG_FORMAT_VARIABLE] ?? "line";
        const format = LOG_FORMATS.find((each) => each === name);
        if (format === undefined) {
            throw new TypeError(`log format '${name}' is invalid: use line or json`);
        }

        return { level, format };
    }

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

    /** Render a log record in the exporter's format. */
    #record(record: ReadableLogRecord): string {
        // read the record's name, severity and time
        const name = record.eventName ?? textOf(record.body);
        const severity = String(record.severityText ?? SeverityNumber[record.severityNumber ?? 0]);
        const [seconds] = record.hrTime;

        return this.#render(seconds * MILLISECONDS_PER_SECOND, severity, name, record.attributes);
    }

    /** Render a record in the exporter's format: a readable line, or one JSON object. */
    #render(at: number, severity: string, name: string, attributes: Attributes): string {
        // write a readable line for a terminal
        if (this.#format === "line") {
            return LineExporter.format(at, severity, name, attributes);
        }

        // write one JSON object for a collector, its fields beside the time, level and name
        return JSON.stringify({
            time: new Date(at).toISOString(),
            level: severity,
            name,
            ...attributes,
        });
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
