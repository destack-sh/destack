import {
    type Attributes,
    type Counter,
    type Gauge,
    type Histogram,
    type Meter,
    metrics,
    type Span,
    SpanStatusCode,
    trace,
    type Tracer,
} from "@opentelemetry/api";
import { type Logger, logs, SeverityNumber } from "@opentelemetry/api-logs";
import type { Package } from "@destack/package";

/** Trace, metric, and log instruments attributed to one package. */
export interface TelemetryScope {
    /** Create spans attributed to the package. */
    readonly tracer: Tracer;
    /** Create metric instruments attributed to the package. */
    readonly meter: Meter;
    /** Emit logs attributed to the package. */
    readonly logger: Logger;
    /** Emit structured log records named by literal event names. */
    readonly log: Log;
    /** Run work in a child span, or read the active span. */
    readonly span: SpanRunner;
    /** Declare metric instruments whose attributes take bounded values. */
    readonly metric: MetricFactory;
}

/** The values each metric attribute takes: bounded lists, so a metric's series stay bounded. */
export type AttributeDomain = Readonly<Record<string, readonly (string | boolean)[]>>;

/** The attributes a domain allows on one measurement. */
export type DomainAttributes<Domain extends AttributeDomain> = {
    readonly [Key in keyof Domain]?: Domain[Key][number];
};

/** A declared metric: its unit, description and bounded attributes. */
export interface MetricDefinition<Domain extends AttributeDomain> {
    /** The unit, such as ms or {block}. */
    readonly unit?: string;
    /** What the metric measures. */
    readonly description?: string;
    /** The values each attribute takes. */
    readonly attributes: Domain;
}

/** Declares a package's metric instruments with bounded attributes, checked by the compiler. */
export class MetricFactory {
    /** The package's meter. */
    readonly #meter: Meter;

    /** Declare instruments on a package's meter. */
    constructor(meter: Meter) {
        this.#meter = meter;
    }

    /** Declare a counter of increments. */
    counter<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): Counter<DomainAttributes<Domain>> {
        return this.#meter.createCounter(name, options(definition));
    }

    /** Declare a histogram of measurements. */
    histogram<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): Histogram<DomainAttributes<Domain>> {
        return this.#meter.createHistogram(name, options(definition));
    }

    /** Declare a gauge of current values. */
    gauge<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): Gauge<DomainAttributes<Domain>> {
        return this.#meter.createGauge(name, options(definition));
    }
}

/** Run work in a child span named by a literal name, or read the active span to add attributes. */
export interface SpanRunner {
    /** Run work in a child span of the active one, recording its failure. */
    <Result>(
        name: string,
        attributes: Attributes,
        run: (span: Span) => Result | Promise<Result>,
    ): Promise<Result>;
    /** Read the active span, such as the call the platform traces. */
    current(): Span | undefined;
}

/** Structured log records of one package, each named by a literal event name. */
export class Log {
    /** The package's logger. */
    readonly #logger: Logger;

    /** Emit through a package's logger. */
    constructor(logger: Logger) {
        this.#logger = logger;
    }

    /** Emit a debug record. */
    debug(name: string, attributes: Attributes = {}): void {
        this.#emit(SeverityNumber.DEBUG, "DEBUG", name, attributes);
    }

    /** Emit an informational record. */
    info(name: string, attributes: Attributes = {}): void {
        this.#emit(SeverityNumber.INFO, "INFO", name, attributes);
    }

    /** Emit a warning record. */
    warn(name: string, attributes: Attributes = {}): void {
        this.#emit(SeverityNumber.WARN, "WARN", name, attributes);
    }

    /** Emit an error record. */
    error(name: string, attributes: Attributes = {}): void {
        this.#emit(SeverityNumber.ERROR, "ERROR", name, attributes);
    }

    /** Emit one record in the active span's context. */
    #emit(
        severityNumber: SeverityNumber,
        severityText: string,
        eventName: string,
        attributes: Attributes,
    ): void {
        this.#logger.emit({ eventName, severityNumber, severityText, attributes });
    }
}

/** Obtain package instruments from the providers registered by the host. */
export function scope(source: Package): TelemetryScope {
    return instrument(
        trace.getTracer(source.name, source.version),
        metrics.getMeter(source.name, source.version),
        logs.getLogger(source.name, source.version),
    );
}

/** Bundle a package's tracer, meter and logger with its log records and spans. */
export function instrument(tracer: Tracer, meter: Meter, logger: Logger): TelemetryScope {
    return {
        tracer,
        meter,
        logger,
        log: new Log(logger),
        span: runner(tracer),
        metric: new MetricFactory(meter),
    };
}

/** Run work in child spans of a tracer. */
function runner(tracer: Tracer): SpanRunner {
    // run the work, recording a failure on the span before ending it
    const run = <Result>(
        name: string,
        attributes: Attributes,
        work: (span: Span) => Result | Promise<Result>,
    ): Promise<Result> =>
        tracer.startActiveSpan(name, { attributes }, async (span) => {
            try {
                return await work(span);
            } catch (error) {
                span.recordException(error instanceof Error ? error : String(error));
                span.setStatus({ code: SpanStatusCode.ERROR });
                throw error;
            } finally {
                span.end();
            }
        });

    return Object.assign(run, { current: () => trace.getActiveSpan() });
}

/** Read the instrument options of a metric definition. */
function options(definition: MetricDefinition<AttributeDomain>) {
    return {
        ...(definition.unit === undefined ? {} : { unit: definition.unit }),
        ...(definition.description === undefined ? {} : { description: definition.description }),
    };
}

/** Describe a failure by the OpenTelemetry exception attributes. */
export function exceptionAttributes(error: unknown): Attributes {
    const exception = error instanceof Error ? error : new Error(String(error));

    return {
        "exception.type": exception.name,
        "exception.message": exception.message,
        ...(exception.stack === undefined ? {} : { "exception.stacktrace": exception.stack }),
    };
}
