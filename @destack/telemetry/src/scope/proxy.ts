import {
    type Attributes,
    type Context,
    type Counter,
    type Gauge,
    type Histogram,
    type Meter,
    type MetricOptions,
    metrics,
    type Span,
    type SpanOptions,
    trace,
    type Tracer,
} from "@opentelemetry/api";
import { type Logger, type LogRecord, logs } from "@opentelemetry/api-logs";

/** The instruments a package creates on its meter: the synchronous ones. */
export type PackageMeter = Pick<Meter, "createCounter" | "createHistogram" | "createGauge">;

/** A meter whose instruments record on the meter provider registered at each measurement. */
export class ProxyMeter implements PackageMeter {
    /** The instrumenting package's name. */
    readonly #name: string;
    /** The instrumenting package's version. */
    readonly #version: string;

    /** Create a meter for a package before or after a meter provider registers. */
    constructor(name: string, version: string) {
        this.#name = name;
        this.#version = version;
    }

    /** Create a counter of increments. */
    createCounter<Labels extends Attributes = Attributes>(
        name: string,
        options?: MetricOptions,
    ): Counter<Labels> {
        const counter = this.#instrument((meter) => meter.createCounter<Labels>(name, options));

        return { add: (...measurement) => counter().add(...measurement) };
    }

    /** Create a histogram of measurements. */
    createHistogram<Labels extends Attributes = Attributes>(
        name: string,
        options?: MetricOptions,
    ): Histogram<Labels> {
        const histogram = this.#instrument((meter) => meter.createHistogram<Labels>(name, options));

        return { record: (...measurement) => histogram().record(...measurement) };
    }

    /** Create a gauge of current values. */
    createGauge<Labels extends Attributes = Attributes>(
        name: string,
        options?: MetricOptions,
    ): Gauge<Labels> {
        const gauge = this.#instrument((meter) => meter.createGauge<Labels>(name, options));

        return { record: (...measurement) => gauge().record(...measurement) };
    }

    /** Resolve an instrument on the provider registered now. */
    #instrument<Instrument>(create: (meter: Meter) => Instrument): () => Instrument {
        return resolver(
            () => metrics.getMeterProvider(),
            (provider) => create(provider.getMeter(this.#name, this.#version)),
        );
    }
}

/** A tracer starting each span on the tracer provider registered at that moment. */
export class ProxyTracer implements Tracer {
    /** The instrumenting package's name. */
    readonly #name: string;
    /** The instrumenting package's version. */
    readonly #version: string;

    /** The package's tracer on the provider registered now. */
    readonly #current: () => Tracer;

    /** Create a tracer for a package before or after a tracer provider registers. */
    constructor(name: string, version: string) {
        this.#name = name;
        this.#version = version;
        this.#current = resolver(
            () => trace.getTracerProvider(),
            (provider) => provider.getTracer(this.#name, this.#version),
        );
    }

    /** Start a span. */
    startSpan(name: string, options?: SpanOptions, context?: Context): Span {
        return this.#current().startSpan(name, options, context);
    }

    /** Run a function in a new active span. */
    startActiveSpan<Run extends (span: Span) => unknown>(name: string, run: Run): ReturnType<Run>;
    /** Run a function in a new active span with options. */
    startActiveSpan<Run extends (span: Span) => unknown>(
        name: string,
        options: SpanOptions,
        run: Run,
    ): ReturnType<Run>;
    /** Run a function in a new active span with options in a context. */
    startActiveSpan<Run extends (span: Span) => unknown>(
        name: string,
        options: SpanOptions,
        context: Context,
        run: Run,
    ): ReturnType<Run>;
    /** Run a function in a new active span on the current tracer, its options and context optional. */
    startActiveSpan<Run extends (span: Span) => unknown>(
        name: string,
        ...rest: [Run] | [SpanOptions, Run] | [SpanOptions, Context, Run]
    ): ReturnType<Run> {
        const tracer = this.#current();
        if (rest.length === 1) {
            return tracer.startActiveSpan(name, rest[0]);
        } else if (rest.length === 2) {
            return tracer.startActiveSpan(name, rest[0], rest[1]);
        }

        return tracer.startActiveSpan(name, rest[0], rest[1], rest[2]);
    }
}

/** A logger emitting each record on the global logger provider current at that moment. */
export class ProxyLogger implements Logger {
    /** The instrumenting package's name. */
    readonly #name: string;
    /** The instrumenting package's version. */
    readonly #version: string;

    /** The package's logger on the provider registered now. */
    readonly #current: () => Logger;

    /** Create a logger for a package before or after a logger provider registers. */
    constructor(name: string, version: string) {
        this.#name = name;
        this.#version = version;
        this.#current = resolver(
            () => logs.getLoggerProvider(),
            (provider) => provider.getLogger(this.#name, this.#version),
        );
    }

    /** Emit a record. */
    emit(record: LogRecord): void {
        this.#current().emit(record);
    }

    /** Report whether a record of the given details would be emitted. */
    enabled(options?: Parameters<Logger["enabled"]>[0]): boolean {
        return this.#current().enabled(options);
    }
}

/** Resolve a value on the provider registered now, again once another one registers. */
function resolver<Provider, Value>(
    current: () => Provider,
    create: (provider: Provider) => Value,
): () => Value {
    let provider: Provider | undefined;
    let value: Value | undefined;

    return () => {
        // create the value on a newly registered provider
        const registered = current();
        if (value === undefined || registered !== provider) {
            provider = registered;
            value = create(registered);
        }

        return value;
    };
}
