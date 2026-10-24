import type { Clock } from "../clock/clock.ts";
import {
    type Context,
    type ContextManager,
    type LoggerProvider,
    type MeterProvider,
    ROOT_CONTEXT,
    type TextMapPropagator,
    type TracerProvider,
} from "@opentelemetry/api";
import {
    CompositePropagator,
    W3CBaggagePropagator,
    W3CTraceContextPropagator,
} from "@opentelemetry/core";
import * as logs from "@opentelemetry/sdk-logs";
import * as metrics from "@opentelemetry/sdk-metrics";
import * as traces from "@opentelemetry/sdk-trace";

/** The W3C propagator of trace context and baggage, which keeps no state. */
export const W3C_PROPAGATOR: TextMapPropagator = new CompositePropagator({
    propagators: [new W3CTraceContextPropagator(), new W3CBaggagePropagator()],
});

/** The OpenTelemetry providers, propagator and context an environment emits signals through. */
export interface Telemetry {
    /** The tracers of the packages' spans. */
    readonly tracers: TracerProvider;
    /** The meters of the packages' metrics. */
    readonly meters: MeterProvider;
    /** The loggers of the packages' log records. */
    readonly loggers: LoggerProvider;
    /** The propagator writing and reading trace context and baggage across processes. */
    readonly propagator: TextMapPropagator;
    /** The manager of the context active in the running work. */
    readonly context: ContextManager;
    /** The clock the spans and log records are stamped on. */
    readonly clock: Clock;
}

/** Telemetry sources. */
export const Telemetry = {
    /** Export nothing, until an entry point gives an environment providers that export. */
    none(clock: Clock, context: ContextManager = new RootContextManager()): Telemetry {
        return {
            tracers: new traces.TracerProvider(),
            meters: new metrics.MeterProvider(),
            loggers: new logs.LoggerProvider(),
            propagator: W3C_PROPAGATOR,
            context,
            clock,
        };
    },
};

/** A context manager keeping no context active, for an environment that traces nothing. */
class RootContextManager implements ContextManager {
    /** Answer the root context, as nothing is ever active. */
    active(): Context {
        return ROOT_CONTEXT;
    }

    /** Run a function without making the context active. */
    with<Arguments extends unknown[], Result>(
        _context: Context,
        run: (...parameters: Arguments) => Result,
        self?: ThisParameterType<typeof run>,
        ...parameters: Arguments
    ): Result {
        return run.call(self, ...parameters);
    }

    /** Answer the target unbound. */
    bind<Target>(_context: Context, target: Target): Target {
        return target;
    }

    /** Enable nothing. */
    enable(): this {
        return this;
    }

    /** Disable nothing. */
    disable(): this {
        return this;
    }
}
