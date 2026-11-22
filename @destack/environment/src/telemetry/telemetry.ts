import { Clock } from "../clock/clock.ts";
import {
    type Context,
    type ContextManager,
    createNoopLogger,
    createNoopMeter,
    type Link,
    INVALID_SPAN_CONTEXT,
    type LoggerProvider,
    type MeterProvider,
    ROOT_CONTEXT,
    type Span,
    type SpanOptions,
    type Tracer,
    type TextMapGetter,
    type TextMapPropagator,
    type TextMapSetter,
    trace,
    type TracerProvider,
} from "@opentelemetry/api";
import {
    CompositePropagator,
    W3CBaggagePropagator,
    W3CTraceContextPropagator,
} from "@opentelemetry/core";
import type { ReadableSpan } from "@opentelemetry/sdk-trace";

/** The setter writing trace fields into HTTP headers. */
const HEADERS_SETTER: TextMapSetter<Headers> = {
    set: (carrier, name, value) => carrier.set(name, value),
};

/** The getter reading trace fields from HTTP headers. */
const HEADERS_GETTER: TextMapGetter<Headers> = {
    keys: (carrier) => [...carrier.keys()],
    get: (carrier, name) => carrier.get(name) ?? undefined,
};

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
    /** The clock durations are measured on: the host's, also in a simulated world. */
    readonly monotonic: Pick<Clock, "now">;
    /** Take spans another process ended, such as a sandboxed worker's, into this pipeline. */
    readonly ingest: (spans: readonly ReadableSpan[]) => void;
}

/** Telemetry sources, and the trace context HTTP headers carry between processes. */
export const Telemetry = {
    /** Record nothing, until an entry point gives an environment providers that export. */
    none(clock: Clock, context: ContextManager = new RootContextManager()): Telemetry {
        return {
            tracers: { getTracer: () => new NoopTracer() },
            meters: { getMeter: () => createNoopMeter() },
            loggers: { getLogger: () => createNoopLogger() },
            propagator: W3C_PROPAGATOR,
            context,
            clock,
            monotonic: Clock.monotonic,
            // drop another process's spans, as this telemetry exports nothing
            ingest: () => {},
        };
    },

    /** Write a trace, the telemetry's active one by default, into an outgoing request's headers. */
    inject(
        headers: Headers,
        telemetry: Pick<Telemetry, "propagator" | "context">,
        parent: Context = telemetry.context.active(),
    ): void {
        telemetry.propagator.inject(parent, headers, HEADERS_SETTER);
    },

    /** Run work with no span active, handing it a link to the span that was running. */
    detach<Result>(telemetry: Pick<Telemetry, "context">, run: (links: Link[]) => Result): Result {
        const running = trace.getSpanContext(telemetry.context.active());
        const links = running === undefined ? [] : [{ context: running }];

        return telemetry.context.with(ROOT_CONTEXT, () => run(links));
    },

    /** Read a remote trace from a request's headers, inheriting no active context. */
    extract(headers: Headers, telemetry: Pick<Telemetry, "propagator">): Context {
        return telemetry.propagator.extract(ROOT_CONTEXT, headers, HEADERS_GETTER);
    },
};

/** A tracer recording nothing, every span it starts being the invalid, non-recording one. */
class NoopTracer implements Tracer {
    /** Start the non-recording span. */
    startSpan(): Span {
        return trace.wrapSpanContext(INVALID_SPAN_CONTEXT);
    }

    /** Run a function with the non-recording span. */
    startActiveSpan<Run extends (span: Span) => unknown>(name: string, run: Run): ReturnType<Run>;
    /** Run a function with the non-recording span, ignoring its options. */
    startActiveSpan<Run extends (span: Span) => unknown>(
        name: string,
        options: SpanOptions,
        run: Run,
    ): ReturnType<Run>;
    /** Run a function with the non-recording span, ignoring its options and context. */
    startActiveSpan<Run extends (span: Span) => unknown>(
        name: string,
        options: SpanOptions,
        context: Context,
        run: Run,
    ): ReturnType<Run>;
    /** Run the last argument with the non-recording span. */
    startActiveSpan<Run extends (span: Span) => ReturnType<Run>>(
        _name: string,
        ...rest: [Run] | [SpanOptions, Run] | [SpanOptions, Context, Run]
    ): ReturnType<Run> {
        // run the function alone, after its options, or after its options and context
        const span = this.startSpan();
        if (rest.length === 1) {
            return rest[0](span);
        } else if (rest.length === 2) {
            return rest[1](span);
        } else {
            return rest[2](span);
        }
    }
}

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
