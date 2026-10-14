import { schema } from "@destack/schema";
import { logs } from "@opentelemetry/api-logs";
import {
    InMemoryLogRecordExporter,
    LoggerProvider,
    SimpleLogRecordProcessor,
} from "@opentelemetry/sdk-logs";

/** An exception a package captured, as a test reads it back. */
export interface CapturedException {
    /** The capturing package's name, its records' instrumentation scope. */
    readonly scope: string;
    /** The error's name. */
    readonly name: string;
    /** The error's message. */
    readonly message: string;
}

/** A message a package captured, such as a warning, as a test reads it back. */
export interface CapturedMessage {
    /** The capturing package's name, its records' instrumentation scope. */
    readonly scope: string;
    /** The record's severity text, such as WARN. */
    readonly severity: string;
    /** The message. */
    readonly message: string;
}

/** The telemetry a test reads back: the log records packages emit, kept in memory. */
export class TestTelemetry implements Disposable {
    /** The records emitted since registering. */
    readonly #records = new InMemoryLogRecordExporter();
    /** The provider registered for the test. */
    readonly #provider: LoggerProvider;

    /** Keep the records packages emit, until disposed. */
    private constructor() {
        const processor = new SimpleLogRecordProcessor({ exporter: this.#records });
        this.#provider = new LoggerProvider({ processors: [processor] });
    }

    /** Register a logger provider keeping every record packages emit in memory. */
    static register(): TestTelemetry {
        // NOTE #Architecture: packages emit through the one global logger provider OpenTelemetry keeps, so tests register theirs there
        const telemetry = new TestTelemetry();
        logs.disable();
        logs.setGlobalLoggerProvider(telemetry.#provider);

        return telemetry;
    }

    /** List the exceptions packages captured, oldest first. */
    exceptions(): CapturedException[] {
        return this.#records
            .getFinishedLogRecords()
            .filter(
                (record) =>
                    record.eventName === "exception" && "exception.type" in record.attributes,
            )
            .map((record) => ({
                scope: record.instrumentationScope.name,
                name: schema.string().parse(record.attributes["exception.type"]),
                message: schema.string().parse(record.attributes["exception.message"]),
            }));
    }

    /** List the messages packages captured without an error, oldest first. */
    messages(): CapturedMessage[] {
        return this.#records
            .getFinishedLogRecords()
            .filter(
                (record) =>
                    record.eventName === "exception" && !("exception.type" in record.attributes),
            )
            .map((record) => ({
                scope: record.instrumentationScope.name,
                severity: schema.string().parse(record.severityText),
                message: schema.string().parse(record.attributes["exception.message"]),
            }));
    }

    /** List the event names of the records packages emitted, oldest first. */
    events(): (string | undefined)[] {
        return this.#records.getFinishedLogRecords().map((record) => record.eventName);
    }

    /** Read the exceptions packages captured and stop keeping the records. */
    take(): CapturedException[] {
        const captured = this.exceptions();
        this.#records.reset();

        return captured;
    }

    /** Unregister the provider if still current, and refuse exceptions no test took. */
    [Symbol.dispose](): void {
        // leave a later registration's provider in place
        if (logs.getLoggerProvider() === this.#provider) {
            logs.disable();
        }

        // refuse the exceptions no test consumed
        const untaken = this.take();
        if (untaken.length > 0) {
            throw new TypeError(`the test took no captured exceptions: ${JSON.stringify(untaken)}`);
        }
    }
}
