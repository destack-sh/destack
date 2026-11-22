import { Clock } from "../clock/clock.ts";
import {
    type Context,
    type ContextManager,
    type Link,
    type LoggerProvider,
    type MeterProvider,
    ROOT_CONTEXT,
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
import * as logs from "@opentelemetry/sdk-logs";
import * as metrics from "@opentelemetry/sdk-metrics";
import * as traces from "@opentelemetry/sdk-trace";
import { LineExporter, type LineExporterOptions } from "./line.ts";

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
    readonly ingest: (spans: readonly traces.ReadableSpan[]) => void;
}

/** Telemetry sources, and the trace context HTTP headers carry between processes. */
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
            monotonic: Clock.monotonic,
            // drop another process's spans, as this telemetry exports nothing
            ingest: () => {},
        };
    },

    /** Write log records and info spans as lines to the runtime's own log, exporting nothing. */
    local(
        clock: Clock,
        write: ConstructorParameters<typeof LineExporter>[0],
        context: ContextManager,
        options: LineExporterOptions = LineExporter.options({}),
    ): Telemetry {
        const exporter = new LineExporter(write, options);

        return {
            ...Telemetry.none(clock, context),
            tracers: new traces.TracerProvider({ spanProcessors: [exporter.spans] }),
            loggers: new logs.LoggerProvider({ processors: [exporter.processor] }),
            // write the spans another process ended, such as a compiler's, as lines too
            ingest: (spans) => {
                for (const span of spans) {
                    exporter.spans.onEnd(span);
                }
            },
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
