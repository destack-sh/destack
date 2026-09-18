import {
    type Attributes,
    context,
    type ContextManager,
    metrics,
    propagation,
    type TextMapPropagator,
    trace,
} from "@opentelemetry/api";
import { logs } from "@opentelemetry/api-logs";
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
import type { TelemetryScope } from "../scope/index.ts";

/** Configure one application's identity and its three telemetry providers. */
export interface TelemetryOptions {
    /** The running service or frontend name. */
    name: string;
    /** The deployed package version. */
    version: string;
    /** Deployment attributes shared by every exported signal. */
    attributes?: Attributes;
    /** HTTP propagation; defaults to W3C trace context and baggage. */
    propagator?: TextMapPropagator;
    /** Trace processors, sampling, and limits. */
    traces: Omit<TracerProviderOptions, "resource">;
    /** Metric readers, aggregation, and limits. */
    metrics: Omit<MeterProviderOptions, "resource">;
    /** Log processors and limits. */
    logs: Omit<LoggerProviderOptions, "resource">;
}

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
    /** Remove only registrations made by this instance. */
    private readonly unregister: (() => void)[] = [];
    /** The shared shutdown operation. */
    private shutdownPromise?: Promise<void>;

    /** Construct providers without changing process globals. */
    constructor(options: TelemetryOptions) {
        // select propagation for both isolated and globally registered providers
        this.propagator = options.propagator ?? new CompositePropagator({
            propagators: [new W3CTraceContextPropagator(), new W3CBaggagePropagator()],
        });

        // identify every signal with the same application resource
        const resource = resourceFromAttributes({
            ...options.attributes,
            "service.name": options.name,
            "service.version": options.version,
        });
        this.traces = new TracerProvider({ ...options.traces, resource });
        this.metrics = new MeterProvider({ ...options.metrics, resource });
        this.logs = new LoggerProvider({ ...options.logs, resource });
    }

    /** Attribute instrumentation to a package using this instance's providers. */
    scope(source: Package): TelemetryScope {
        return {
            tracer: this.traces.getTracer(source.name, source.version),
            meter: this.metrics.getMeter(source.name, source.version),
            logger: this.logs.getLogger(source.name, source.version),
        };
    }

    /** Register providers and the host's context manager once per application. */
    static async start(options: TelemetryOptions, manager: ContextManager): Promise<Telemetry> {
        const telemetry = new Telemetry(options);

        // reject competing global providers and undo partial registration on failure
        try {
            telemetry.register(context.setGlobalContextManager(manager), () => context.disable());
            manager.enable();
            telemetry.register(
                propagation.setGlobalPropagator(telemetry.propagator),
                () => propagation.disable(),
            );
            telemetry.register(
                trace.setGlobalTracerProvider(telemetry.traces),
                () => trace.disable(),
            );
            telemetry.register(
                metrics.setGlobalMeterProvider(telemetry.metrics),
                () => metrics.disable(),
            );
            telemetry.register(
                logs.setGlobalLoggerProvider(telemetry.logs) === telemetry.logs,
                () => logs.disable(),
            );
        } catch (error) {
            // preserve the registration failure if cleanup also fails
            try {
                await telemetry.shutdown();
            } catch (cleanupError) {
                throw new AggregateError([error, cleanupError], "Telemetry initialization failed.");
            }

            throw error;
        }

        return telemetry;
    }

    /** Export buffered signals before a request or application finishes. */
    async flush(): Promise<void> {
        if (this.shutdownPromise) throw new Error("Telemetry is shut down.");

        await complete([
            this.traces.forceFlush(),
            this.metrics.forceFlush(),
            this.logs.forceFlush(),
        ]);
    }

    /** Remove global registrations, export buffered signals, and stop providers. */
    shutdown(): Promise<void> {
        if (this.shutdownPromise) return this.shutdownPromise;

        // release registrations in reverse order
        for (const unregister of this.unregister.splice(0).reverse()) unregister();

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
            throw new Error("OpenTelemetry is already initialized in this application.");
        }
        this.unregister.push(unregister);
    }
}

/** Wait for every provider and report every failure. */
async function complete(operations: Promise<void>[]): Promise<void> {
    const results = await Promise.allSettled(operations);
    const errors = results.filter((result) => result.status === "rejected").map((result) =>
        result.reason
    );
    if (errors.length > 0) throw new AggregateError(errors, "Telemetry export failed.");
}
