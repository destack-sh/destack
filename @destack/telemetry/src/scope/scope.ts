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
import { DomainError } from "@destack/error";

/** The log severity of each capture level. */
const LEVELS: Readonly<Record<CaptureLevel, readonly [SeverityNumber, string]>> = {
    fatal: [SeverityNumber.FATAL, "FATAL"],
    error: [SeverityNumber.ERROR, "ERROR"],
    warning: [SeverityNumber.WARN, "WARN"],
    info: [SeverityNumber.INFO, "INFO"],
};

/** The failures captured already: here, or by the service that answered with them. */
const CAPTURED = new WeakSet<object>();

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
    /** Record a failure the package handled as an exception in the active trace. */
    readonly captureException: (error: unknown, context?: CaptureContext) => void;
    /** Record a message as an exception-level event in the active trace. */
    readonly captureMessage: (message: string, context?: CaptureContext) => void;
}

/** How bad a captured failure is, after Sentry's levels. */
export type CaptureLevel = "fatal" | "error" | "warning" | "info";

/** What a capture adds to the exception it records. */
export interface CaptureContext {
    /** How bad the failure is, error by default. */
    readonly level?: CaptureLevel;
    /** Bounded labels to filter and group issues by, such as the feature. */
    readonly tags?: Readonly<Record<string, string>>;
    /** The fingerprint grouping the failure into an issue, replacing the one its frames give. */
    readonly fingerprint?: readonly string[];
    /** Details beside the tags that issues never group by, such as the failing key. */
    readonly attributes?: Attributes;
    /** Whether the failure escaped the code it ran in, such as a request handler, false by default. */
    readonly isEscaped?: boolean;
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
        captureException: (error, context) => {
            // skip a failure captured where it happened
            if (isCaptured(error)) {
                return;
            }
            markCaptured(error);
            emitException(logger, exceptionAttributes(error, context?.isEscaped === true), context);
        },
        captureMessage: (message, context) =>
            emitException(
                logger,
                { "exception.message": message, "exception.escaped": false },
                context,
            ),
    };
}

/** Mark a failure captured where it happened, such as a service's answer the service captured, so no capture records it again. */
export function markCaptured(error: unknown): void {
    if (typeof error === "object" && error !== null) {
        CAPTURED.add(error);
    }
}

/** Report whether a failure was captured already. */
function isCaptured(error: unknown): boolean {
    return typeof error === "object" && error !== null && CAPTURED.has(error);
}

/** Emit an exception record in the active span's context with its level, details, tags and fingerprint. */
export function emitException(
    logger: Logger,
    attributes: Attributes,
    context: CaptureContext = {},
): void {
    const [severityNumber, severityText] = LEVELS[context.level ?? "error"];
    logger.emit({
        eventName: "exception",
        severityNumber,
        severityText,
        attributes: {
            ...context.attributes,
            ...attributes,
            ...Object.fromEntries(
                Object.entries(context.tags ?? {}).map(([key, value]) => [`tag.${key}`, value]),
            ),
            ...(context.fingerprint === undefined
                ? {}
                : { "exception.fingerprint": [...context.fingerprint] }),
        },
    });
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

/** Describe a failure by the OpenTelemetry exception attributes, its error type and whether it escaped the code recording it. */
export function exceptionAttributes(error: unknown, isEscaped: boolean): Attributes {
    const exception = error instanceof Error ? error : new Error(String(error));

    return {
        "error.type": errorType(exception),
        "exception.type": exception.name,
        "exception.message": exception.message,
        ...(exception.stack === undefined ? {} : { "exception.stacktrace": exception.stack }),
        "exception.escaped": isEscaped,
    };
}

/** Read a failure's error type: the service error code it reports, else its own code, else its name. */
function errorType(exception: Error): string {
    if (DomainError.is(exception)) {
        return exception.toServiceError().code;
    } else if ("code" in exception && typeof exception.code === "string") {
        return exception.code;
    }

    return exception.name;
}
