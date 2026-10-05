import {
    type Attributes,
    context,
    type ContextManager,
    type Counter,
    diag,
    DiagLogLevel,
    metrics,
    propagation,
    type TextMapPropagator,
    trace,
    ValueType,
} from "@opentelemetry/api";
import { type Logger, logs, SeverityNumber } from "@opentelemetry/api-logs";
import {
    CompositePropagator,
    W3CBaggagePropagator,
    W3CTraceContextPropagator,
} from "@opentelemetry/core";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { LoggerProvider, type LoggerProviderOptions } from "@opentelemetry/sdk-logs";
import { MeterProvider, type MeterProviderOptions } from "@opentelemetry/sdk-metrics";
import { TracerProvider, type TracerProviderOptions } from "@opentelemetry/sdk-trace";
import type { Package } from "@destack/package";
import { ATTR_DESTACK_BUILD_MANIFEST } from "../convention/source.ts";
import {
    emitException,
    exceptionAttributes,
    instrument,
    type TelemetryScope,
} from "../scope/index.ts";
import { TraceIdGenerator } from "../trace/generator.ts";
import { type Session, SessionProcessor } from "../session/index.ts";

/** Configure one application's identity and its three telemetry providers. */
export interface TelemetryOptions {
    /** The running service or frontend name. */
    name: string;
    /** The deployed package version, the release every signal names. */
    version: string;
    /** The digest of the running build's manifest, naming the exact build on every signal. */
    manifest?: string;
    /** Deployment attributes shared by every exported signal. */
    attributes?: Attributes;
    /** The HTTP propagator, W3C trace context and baggage by default. */
    propagator?: TextMapPropagator;
    /** Trace processors, sampling, and limits. */
    traces: Omit<TracerProviderOptions, "resource">;
    /** Metric readers, aggregation, and limits. */
    metrics: Omit<MeterProviderOptions, "resource">;
    /** Log processors and limits. */
    logs: Omit<LoggerProviderOptions, "resource">;
    /** Report the SDK's own warnings and failures, such as dropped spans. */
    report: (error: Error) => void;
}

/** Install a realm's capture of uncaught failures into running telemetry, returning how to remove it. */
export type Capture = (telemetry: Telemetry) => () => void;

/** Manage one application's trace, metric, and log providers. */
export class Telemetry {
    /** The HTTP trace context and baggage propagator. */
    readonly propagator: TextMapPropagator;
    /** The trace provider. */
    readonly traces: TracerProvider;
    /** The metric provider. */
    readonly metrics: MeterProvider;
    /** The structured log provider. */
    readonly logs: LoggerProvider;
    /** The application's own logger, recording uncaught failures and sessions. */
    private readonly logger: Logger;
    /** The counter of ended sessions by status. */
    private readonly sessionCount: Counter;
    /** The processor stamping signals with the running session. */
    private readonly sessionProcessor = new SessionProcessor();
    /** Remove only registrations made by this instance. */
    private readonly unregister: (() => void)[] = [];
    /** The shared shutdown operation. */
    private shutdownPromise?: Promise<void>;
    /** The flush in progress, settled. */
    private flushing: Promise<void> = Promise.resolve();

    /** Construct providers without changing process globals. */
    constructor(options: TelemetryOptions) {
        // select propagation for both isolated and globally registered providers
        this.propagator =
            options.propagator ??
            new CompositePropagator({
                propagators: [new W3CTraceContextPropagator(), new W3CBaggagePropagator()],
            });

        // identify every signal with the same application resource
        const resource = resourceFromAttributes({
            ...options.attributes,
            "service.name": options.name,
            "service.version": options.version,
            ...(options.manifest === undefined
                ? {}
                : { [ATTR_DESTACK_BUILD_MANIFEST]: options.manifest }),
        });
        this.traces = new TracerProvider({
            idGenerator: new TraceIdGenerator(),
            ...options.traces,
            spanProcessors: [this.sessionProcessor.spans, ...(options.traces.spanProcessors ?? [])],
            resource,
        });
        this.metrics = new MeterProvider({ ...options.metrics, resource });
        this.logs = new LoggerProvider({
            ...options.logs,
            processors: [this.sessionProcessor.logs, ...(options.logs.processors ?? [])],
            resource,
        });
        this.logger = this.logs.getLogger(options.name, options.version);
        this.sessionCount = this.metrics
            .getMeter(options.name, options.version)
            .createCounter("session.count", {
                unit: "{session}",
                description: "The sessions that ended, by how they ended.",
                valueType: ValueType.INT,
            });
    }

    /** Attribute instrumentation to a package using this instance's providers. */
    scope(source: Package): TelemetryScope {
        return instrument(
            this.traces.getTracer(source.name, source.version),
            this.metrics.getMeter(source.name, source.version),
            this.logs.getLogger(source.name, source.version),
        );
    }

    /** Record an uncaught failure as an error log record in the active trace. */
    capture(error: unknown): void {
        emitException(this.logger, exceptionAttributes(error, true));
    }

    /** Start a session of a person using the application, ending the running one, after OpenTelemetry's session events. */
    startSession(): Session {
        // end the running session and start the next one
        this.endSession();
        const session = this.sessionProcessor.start();
        this.logger.emit({
            eventName: "session.start",
            severityNumber: SeverityNumber.INFO,
            severityText: "INFO",
        });

        return session;
    }

    /** End the running session, recording how it ended and counting it by its status. */
    endSession(): Session | undefined {
        // record the end in the session before closing it
        const session = this.sessionProcessor.session;
        if (session === undefined) {
            return undefined;
        }
        this.logger.emit({
            eventName: "session.end",
            severityNumber: SeverityNumber.INFO,
            severityText: "INFO",
            attributes: { "session.status": session.status },
        });
        this.sessionCount.add(1, { "session.status": session.status });

        return this.sessionProcessor.end();
    }

    /** Register providers, the host's context manager and the realm's capture once per application. */
    static async start(
        options: TelemetryOptions,
        manager: ContextManager,
        capture?: Capture,
    ): Promise<Telemetry> {
        const telemetry = new Telemetry(options);

        // reject competing global providers and undo partial registration on failure
        try {
            telemetry.register(context.setGlobalContextManager(manager), () => context.disable());
            manager.enable();
            telemetry.register(propagation.setGlobalPropagator(telemetry.propagator), () =>
                propagation.disable(),
            );
            telemetry.register(trace.setGlobalTracerProvider(telemetry.traces), () =>
                trace.disable(),
            );
            telemetry.register(metrics.setGlobalMeterProvider(telemetry.metrics), () =>
                metrics.disable(),
            );
            telemetry.register(
                logs.setGlobalLoggerProvider(telemetry.logs) === telemetry.logs,
                () => logs.disable(),
            );

            // report the SDK's warnings once no competing application's logger can be replaced
            const report = (message: string, ...details: unknown[]) =>
                options.report(new Error([message, ...details.map(String)].join(" ")));
            const logger = {
                error: report,
                warn: report,
                info: () => {},
                debug: () => {},
                verbose: () => {},
            };
            telemetry.register(diag.setLogger(logger, DiagLogLevel.WARN), () => diag.disable());
            if (capture !== undefined) {
                telemetry.unregister.push(capture(telemetry));
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

    /** Export buffered signals before a request or application finishes, after the flush in progress. */
    flush(): Promise<void> {
        if (this.shutdownPromise) {
            return Promise.reject(new Error("telemetry is shut down"));
        }

        // run after the flush in progress, whose processors skip a concurrent one
        const flushing = this.flushing.then(() =>
            complete([this.traces.forceFlush(), this.metrics.forceFlush(), this.logs.forceFlush()]),
        );
        this.flushing = flushing.catch(() => {});

        return flushing;
    }

    /** Remove global registrations, export buffered signals, and stop providers. */
    shutdown(): Promise<void> {
        if (this.shutdownPromise) {
            return this.shutdownPromise;
        }

        // release registrations in reverse order
        for (const unregister of this.unregister.splice(0).toReversed()) {
            unregister();
        }

        // stop every provider once
        this.shutdownPromise = complete([
            this.traces.shutdown(),
            this.metrics.shutdown(),
            this.logs.shutdown(),
        ]);

        return this.shutdownPromise;
    }

    /** Track a successful registration or fail without replacing another provider. */
    private register(isRegistered: boolean, unregister: () => void): void {
        if (!isRegistered) {
            throw new Error("OpenTelemetry is already initialized in this application");
        }
        this.unregister.push(unregister);
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
