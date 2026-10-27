import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import type { ReadableLogRecord } from "@opentelemetry/sdk-logs";
import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** An exception a package captured, as a test reads it back. */
export interface CapturedException {
    /** The capturing package's name, its records' instrumentation scope. */
    readonly scope: string;
    /** The error's name. */
    readonly name: string;
    /** The error's message. */
    readonly message: string;
}

/** A log record a package emitted, as a test reads it back. */
export interface CapturedRecord {
    /** The emitting package's name, its records' instrumentation scope. */
    readonly scope: string;
    /** The record's event name, such as tunnel.open or exception. */
    readonly event: string | undefined;
    /** The record's severity text, such as WARN. */
    readonly severity: string | undefined;
    /** The record's attributes. */
    readonly attributes: Readonly<Record<string, unknown>>;
}

/** The log records a world's telemetry keeps, as a world's SimulatedTelemetry has them. */
export interface RecordedLogs {
    /** The log records emitted so far, oldest first. */
    readonly logs: readonly ReadableLogRecord[];
}

/** The records a test reads back from a world's telemetry, refusing exceptions no test took. */
export class TestTelemetry implements Disposable {
    /** The world's telemetry. */
    readonly #recorded: RecordedLogs;
    /** The records a test took already, which later reads skip. */
    #taken = 0;

    /** Read back the records a world's telemetry keeps, from now on. */
    constructor(recorded: RecordedLogs) {
        this.#recorded = recorded;
        this.#taken = recorded.logs.length;
    }

    /** List the exceptions packages captured, oldest first. */
    exceptions(): CapturedException[] {
        return this.#unread()
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

    /** List the records packages emitted, oldest first. */
    records(): CapturedRecord[] {
        return this.#unread().map((record) => ({
            scope: record.instrumentationScope.name,
            event: record.eventName,
            severity: record.severityText,
            attributes: { ...record.attributes },
        }));
    }

    /** Describe the unread exceptions with the causes their stack traces list. */
    describe(): string {
        const described = this.#unread()
            .filter(
                (record) =>
                    record.eventName === "exception" && "exception.type" in record.attributes,
            )
            .map((record) => {
                // read each cause's first line below the stack
                const stack = record.attributes["exception.stacktrace"];
                const causes =
                    typeof stack === "string"
                        ? stack
                              .split("\nCaused by: ")
                              .slice(1)
                              .map((section) => section.split("\n")[0])
                        : [];

                return {
                    scope: record.instrumentationScope.name,
                    name: record.attributes["exception.type"],
                    message: record.attributes["exception.message"],
                    ...(causes.length === 0 ? {} : { causes }),
                };
            });

        return JSON.stringify(described);
    }

    /** Read the exceptions packages captured and skip the records read so far. */
    take(): CapturedException[] {
        const captured = this.exceptions();
        this.#taken = this.#recorded.logs.length;

        return captured;
    }

    /** Refuse the exceptions no test took. */
    [Symbol.dispose](): void {
        const described = this.describe();
        if (this.take().length > 0) {
            throw new TypeError(`the test took no captured exceptions: ${described}`);
        }
    }

    /** List the records emitted since the last take. */
    #unread(): readonly ReadableLogRecord[] {
        return this.#recorded.logs.slice(this.#taken);
    }
}

/** Start an application's telemetry exporting nothing, on an environment's clock and randomness. */
export function startSilentTelemetry(
    options: TelemetryOptions,
    environment: Pick<Environment, "clock" | "random">,
): Promise<Telemetry> {
    const silent = { ...options, metrics: {}, signals: () => ({ traces: {}, logs: {} }) };

    return Telemetry.start(silent, new AsyncLocalStorageContextManager(), environment);
}
