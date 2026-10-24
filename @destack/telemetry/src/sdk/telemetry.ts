import {
    type ContextManager,
    type Counter,
    type Logger,
    SeverityNumber,
    type TextMapPropagator,
    ValueType,
} from "@opentelemetry/api";
import {} from "@opentelemetry/core";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { LoggerProvider, type LoggerProviderOptions } from "@opentelemetry/sdk-logs";
import { MeterProvider, type MeterProviderOptions } from "@opentelemetry/sdk-metrics";
import { TracerProvider, type TracerProviderOptions } from "@opentelemetry/sdk-trace";
import { type Environment, TraceIdGenerator, W3C_PROPAGATOR } from "@destack/environment";
import { emitException, exceptionAttributes } from "../scope/index.ts";
import { Diagnostics } from "./diagnostic.ts";
import {
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    type Session,
    SessionProcessor,
} from "../session/index.ts";
/** Configure one application's identity and its three telemetry providers. */
export interface TelemetryOptions {
    /** The running service or frontend name. */
    readonly name: string;
    /** The deployed package version, the release every signal names. */
    readonly version: string;
    /** The HTTP propagator, W3C trace context and baggage by default. */
    readonly propagator?: TextMapPropagator;
    /** Metric readers, aggregation, and limits. */
    readonly metrics: Omit<MeterProviderOptions, "resource">;
    /** Build the trace and log setup, which may count on the application's meters. */
    readonly signals: (meters: MeterProvider) => TelemetrySignals;
    /** Report the SDK's own warnings and failures, such as dropped spans. */
    readonly report: (error: Error) => void;
}

/** The trace and log setup of an application's telemetry. */
export interface TelemetrySignals {
    /** Trace processors, sampling, and limits. */
    readonly traces: Omit<TracerProviderOptions, "resource">;
    /** Log processors and limits. */
    readonly logs: Omit<LoggerProviderOptions, "resource">;
}

/** Install a realm's capture of uncaught failures into running telemetry. */
export type Capture = (telemetry: Telemetry) => () => void;

/** One application's trace, metric and log providers, exporting through its options. */
export class Telemetry {
    /** The HTTP trace context and baggage propagator. */
    readonly propagator: TextMapPropagator;
    /** Report a failure of the SDK or of an export, as the options report it. */
    readonly report: (error: unknown) => void;
    /** The manager of the context active in the running work. */
    readonly context: ContextManager;
    /** The trace provider. */
    readonly tracers: TracerProvider;
    /** The metric provider. */
    readonly meters: MeterProvider;
    /** The structured log provider. */
    readonly loggers: LoggerProvider;
    /** The clock the sessions start on and the spans and log records are stamped on. */
    readonly clock: Environment["clock"];
    /** The application's own logger, recording uncaught failures and sessions. */
    readonly #logger: Logger;
    /** The counter of ended sessions by status. */
    readonly #sessionCount: Counter;
    /** The processor stamping signals with the running session. */
    readonly #sessionProcessor: SessionProcessor;
    /** Remove only registrations made by this instance. */
    readonly #unregister: (() => void)[] = [];
    /** The shared shutdown operation, absent before the first shutdown. */
    #stopped: Promise<void> | undefined;
    /** The flush in progress, settled. */
    #flushing: Promise<void> = Promise.resolve();

    /** Construct providers on an environment's clock and randomness, in a context manager. */
    constructor(
        options: TelemetryOptions,
        context: ContextManager,
        environment: Pick<Environment, "clock" | "random">,
    ) {
        // select propagation, and keep the context and the clock
        this.propagator = options.propagator ?? W3C_PROPAGATOR;
        this.context = context;
        this.report = (error) =>
            options.report(error instanceof Error ? error : new Error(String(error)));
        this.clock = environment.clock;
        this.#sessionProcessor = new SessionProcessor(environment.random);

        // identify every signal with the same application resource
        const resource = resourceFromAttributes({
            "service.name": options.name,
            "service.version": options.version,
        });
        this.meters = new MeterProvider({ ...options.metrics, resource });
        const { traces, logs } = options.signals(this.meters);
        this.tracers = new TracerProvider({
            idGenerator: new TraceIdGenerator(environment),
            ...traces,
            spanProcessors: [this.#sessionProcessor.spans, ...(traces.spanProcessors ?? [])],
            resource,
        });
        this.loggers = new LoggerProvider({
            ...logs,
            processors: [this.#sessionProcessor.logs, ...(logs.processors ?? [])],
            resource,
        });

        // count sessions and record failures on the application's own instruments
        this.#logger = this.loggers.getLogger(options.name, options.version);
        this.#sessionCount = this.meters
            .getMeter(options.name, options.version)
            .createCounter("session.count", {
                unit: "{session}",
                description: "The sessions that ended, by how they ended.",
                valueType: ValueType.INT,
            });
    }

    /** Record an uncaught failure as an error log record in the active trace. */
    capture(error: unknown): void {
        emitException(this.#logger, exceptionAttributes(error, true), this);
    }

    /** Start a session of a person using the application, ending the running one. */
    startSession(): Session {
        // end the running session and start the next one
        this.endSession();
        const session = this.#sessionProcessor.start(this.clock.now());
        this.#logger.emit({
            eventName: SESSION_EVENTS.start,
            severityNumber: SeverityNumber.INFO,
            severityText: "INFO",
            context: this.context.active(),
            timestamp: this.clock.now(),
            observedTimestamp: this.clock.now(),
        });

        return session;
    }

    /** End the running session, recording how it ended and counting it by its status. */
    endSession(): Session | undefined {
        // record the end in the session before closing it
        const session = this.#sessionProcessor.session;
        if (session === undefined) {
            return undefined;
        }
        this.#logger.emit({
            eventName: SESSION_EVENTS.end,
            severityNumber: SeverityNumber.INFO,
            severityText: "INFO",
            attributes: { [SESSION_STATUS_ATTRIBUTE]: session.status },
            context: this.context.active(),
            timestamp: this.clock.now(),
            observedTimestamp: this.clock.now(),
        });
        this.#sessionCount.add(1, { [SESSION_STATUS_ATTRIBUTE]: session.status });

        return this.#sessionProcessor.end();
    }

    /** Start telemetry in a context manager, reporting SDK warnings and capturing failures. */
    static async start(
        options: TelemetryOptions,
        context: ContextManager,
        environment: Pick<Environment, "clock" | "random">,
        capture?: Capture,
    ): Promise<Telemetry> {
        const telemetry = new Telemetry(options, context.enable(), environment);

        // report the SDK's warnings, which reach OpenTelemetry's one diagnostic logger, then capture failures
        try {
            const report = (message: string, ...details: unknown[]) => {
                // keep an error the SDK passes as the cause, so its stack reaches the report
                const cause = details.find((detail) => detail instanceof Error);
                const text = details.filter((detail) => detail !== cause).map(String);
                options.report(
                    new Error([message, ...text].join(" "), cause === undefined ? {} : { cause }),
                );
            };
            telemetry.#unregister.push(Diagnostics.register(report));
            if (capture !== undefined) {
                telemetry.#unregister.push(capture(telemetry));
            }
        } catch (error) {
            // preserve the registration failure if cleanup also fails
            try {
                await telemetry.shutdown();
            } catch (cleanupError) {
                throw new AggregateError([error, cleanupError], "telemetry initialization failed", {
                    cause: cleanupError,
                });
            }

            throw error;
        }

        return telemetry;
    }

    /** Export buffered signals before a request or application finishes. */
    flush(): Promise<void> {
        if (this.#stopped) {
            return Promise.reject(new Error("telemetry is shut down"));
        }

        // run after the flush in progress, whose processors skip a concurrent one
        const flushing = this.#flushing.then(() =>
            complete([
                this.tracers.forceFlush(),
                this.meters.forceFlush(),
                this.loggers.forceFlush(),
            ]),
        );
        this.#flushing = flushing.catch(() => {});

        return flushing;
    }

    /** Remove the diagnostic logger and the capture, export buffered signals, and stop. */
    shutdown(): Promise<void> {
        if (this.#stopped) {
            return this.#stopped;
        }

        // release registrations in reverse order, and stop the context manager
        for (const unregister of this.#unregister.splice(0).toReversed()) {
            unregister();
        }
        this.context.disable();

        // stop every provider once
        this.#stopped = complete([
            this.tracers.shutdown(),
            this.meters.shutdown(),
            this.loggers.shutdown(),
        ]);

        return this.#stopped;
    }
}

/** Wait for every provider and report every failure. */
async function complete(operations: Promise<void>[]): Promise<void> {
    const results = await Promise.allSettled(operations);
    const errors = results.flatMap((result): unknown[] =>
        result.status === "rejected" ? [result.reason] : [],
    );
    if (errors.length > 0) {
        throw new AggregateError(errors, "telemetry export failed");
    }
}
