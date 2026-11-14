import {
    type Attributes,
    type Counter,
    type Gauge,
    type UpDownCounter,
    type Histogram,
    type Logger,
    type Meter,
    SeverityNumber,
    type Tracer,
} from "@opentelemetry/api";
import {
    DURATION_BUCKETS,
    EXCEPTION_EVENT,
    type Environment,
    exceptionAttributes,
    SpanRunner,
} from "@destack/environment";
import type { PackageVersion } from "@destack/package";

import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    VISIT_EVENT,
    type VisitDetail,
} from "../otlp/analytics.ts";

/** The log severity of each capture level. */
const LEVELS: Readonly<Record<CaptureLevel, readonly [SeverityNumber, string]>> = {
    fatal: [SeverityNumber.FATAL, "FATAL"],
    error: [SeverityNumber.ERROR, "ERROR"],
    warning: [SeverityNumber.WARN, "WARN"],
    info: [SeverityNumber.INFO, "INFO"],
};

/** The instruments a package creates on its meter: the synchronous ones. */
export type PackageMeter = Pick<
    Meter,
    "createCounter" | "createUpDownCounter" | "createHistogram" | "createGauge"
>;

/** The property a failure carries once captured: here, or by the service that answered with it. */
const CAPTURED = Symbol.for("destack.telemetry.captured");

/** Trace, metric, and log instruments attributed to one package. */
export interface TelemetryScope {
    /** Create spans attributed to the package. */
    readonly tracer: Tracer;
    /** Create metric instruments attributed to the package. */
    readonly meter: PackageMeter;
    /** Emit logs attributed to the package. */
    readonly logger: Logger;
    /** Emit structured log records named by literal event names. */
    readonly log: Log;
    /** Run work in a child span, or read the active span. */
    readonly span: SpanRunner;
    /** Run work, recording its seconds on the named duration, with the failure it ends in. */
    readonly measure: SpanRunner["measure"];
    /** Declare metric instruments whose attributes take bounded values. */
    readonly metric: MetricFactory;
    /** Record a failure the package handled as an exception in the active trace. */
    readonly captureException: (error: unknown, context?: CaptureContext) => void;
    /** Record a message as an exception-level event in the active trace. */
    readonly captureMessage: (message: string, context?: CaptureContext) => void;
    /** Record an action a person took under a literal name, with its properties. */
    readonly track: (name: string, attributes?: Attributes) => void;
    /** Record a view a person opened at a route. */
    readonly visit: (route: string, detail?: VisitDetail) => void;
}

/** Telemetry scopes. */
export const TelemetryScope = {
    /** Attribute instrumentation to a package on an environment's telemetry. */
    of(source: PackageVersion, telemetry: Environment["telemetry"]): TelemetryScope {
        return instrument(
            source,
            telemetry.tracers.getTracer(source.name, source.version),
            telemetry.meters.getMeter(source.name, source.version),
            telemetry.loggers.getLogger(source.name, source.version),
            telemetry,
        );
    },
};

/** How bad a captured failure is. */
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
    /** Whether the failure escaped the code it ran in, such as a request handler. */
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
    readonly #meter: PackageMeter;

    /** Declare instruments on a package's meter. */
    constructor(meter: PackageMeter) {
        this.#meter = meter;
    }

    /** Declare a counter of increments. */
    counter<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): Counter<DomainAttributes<Domain>> {
        return this.#meter.createCounter(name, options(definition));
    }

    /** Declare a counter of things open, which rises and falls. */
    upDownCounter<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): UpDownCounter<DomainAttributes<Domain>> {
        return this.#meter.createUpDownCounter(name, options(definition));
    }

    /** Declare a histogram of measurements, a duration in seconds on the conventions' buckets. */
    histogram<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): Histogram<DomainAttributes<Domain>> {
        const advice =
            definition.unit === "s"
                ? { advice: { explicitBucketBoundaries: DURATION_BUCKETS } }
                : {};

        return this.#meter.createHistogram(name, { ...options(definition), ...advice });
    }

    /** Declare a gauge of current values. */
    gauge<const Domain extends AttributeDomain>(
        name: string,
        definition: MetricDefinition<Domain>,
    ): Gauge<DomainAttributes<Domain>> {
        return this.#meter.createGauge(name, options(definition));
    }
}

/** Structured log records of one package, each named by a literal event name under its scope. */
export class Log {
    /** The package's logger. */
    readonly #logger: Logger;
    /** The prefix of the package's event names: its scope, such as `destack.` for `@destack/db`. */
    readonly #prefix: string;
    /** The context the records belong to, and the clock they are stamped on. */
    readonly #telemetry: Pick<Environment["telemetry"], "context" | "clock">;

    /** Emit through a package's logger in a telemetry's active context, on its clock. */
    constructor(
        source: PackageVersion,
        logger: Logger,
        telemetry: Pick<Environment["telemetry"], "context" | "clock">,
    ) {
        this.#logger = logger;
        this.#prefix = `${source.name.replace(/^@/u, "").split("/")[0] ?? source.name}.`;
        this.#telemetry = telemetry;
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

    /** Emit one record in the active span's context. */
    #emit(
        severityNumber: SeverityNumber,
        severityText: string,
        eventName: string,
        attributes: Attributes,
    ): void {
        // refuse a name outside the package's scope
        if (!eventName.startsWith(this.#prefix)) {
            throw new TypeError(`log record names start with '${this.#prefix}': '${eventName}'`);
        }
        const { context, clock } = this.#telemetry;
        this.#logger.emit({
            eventName,
            severityNumber,
            severityText,
            attributes,
            context: context.active(),
            timestamp: clock.now(),
            observedTimestamp: clock.now(),
        });
    }
}

/** Bundle a package's tracer, meter and logger with its log records and spans in a context. */
function instrument(
    source: PackageVersion,
    tracer: Tracer,
    meter: PackageMeter,
    logger: Logger,
    telemetry: Pick<Environment["telemetry"], "context" | "clock" | "monotonic">,
): TelemetryScope {
    // read the telemetry's context and clock
    const { context, clock } = telemetry;

    // run the package's spans and timings through the one span runner
    const span = SpanRunner.of(tracer, meter, telemetry);

    return {
        tracer,
        meter,
        logger,
        log: new Log(source, logger, telemetry),
        span,
        measure: span.measure,
        metric: new MetricFactory(meter),
        captureException: (error, capture) => {
            // skip a failure captured where it happened
            if (isCaptured(error)) {
                return;
            }
            markCaptured(error);
            const attributes = exceptionAttributes(error, capture?.isEscaped === true);
            emitException(logger, attributes, telemetry, capture);
        },
        captureMessage: (message, capture) =>
            emitException(
                logger,
                { "exception.message": message, "exception.escaped": false },
                telemetry,
                capture,
            ),
        track: (name, attributes = {}) =>
            logger.emit({
                eventName: ACTION_EVENT,
                attributes: { ...attributes, [ANALYTICS_ATTRIBUTES.action]: name },
                context: context.active(),
                timestamp: clock.now(),
                observedTimestamp: clock.now(),
            }),
        visit: (route, detail = {}) =>
            logger.emit({
                eventName: VISIT_EVENT,
                context: context.active(),
                timestamp: clock.now(),
                observedTimestamp: clock.now(),
                attributes: {
                    [ANALYTICS_ATTRIBUTES.route]: route,
                    ...Object.fromEntries(
                        (["path", "title", "referrer", "locale"] as const).flatMap((field) => {
                            const value = detail[field];

                            return value === undefined
                                ? []
                                : [[ANALYTICS_ATTRIBUTES[field], value]];
                        }),
                    ),
                },
            }),
    };
}

/** Mark a failure captured where it happened, so no capture records it again. */
export function markCaptured(error: unknown): void {
    if (typeof error === "object" && error !== null) {
        Object.defineProperty(error, CAPTURED, { value: true });
    }
}

/** Report whether a failure was captured already. */
function isCaptured(error: unknown): boolean {
    return typeof error === "object" && error !== null && CAPTURED in error;
}

/** Emit an exception record in the active span's context. */
export function emitException(
    logger: Logger,
    attributes: Attributes,
    telemetry: Pick<Environment["telemetry"], "context" | "clock">,
    capture: CaptureContext = {},
): void {
    const [severityNumber, severityText] = LEVELS[capture.level ?? "error"];
    logger.emit({
        eventName: EXCEPTION_EVENT,
        severityNumber,
        severityText,
        context: telemetry.context.active(),
        timestamp: telemetry.clock.now(),
        observedTimestamp: telemetry.clock.now(),
        attributes: {
            ...capture.attributes,
            ...attributes,
            ...Object.fromEntries(
                Object.entries(capture.tags ?? {}).map(([key, value]) => [`tag.${key}`, value]),
            ),
            ...(capture.fingerprint === undefined
                ? {}
                : { "exception.fingerprint": [...capture.fingerprint] }),
        },
    });
}

/** Read the instrument options of a metric definition. */
function options(definition: MetricDefinition<AttributeDomain>) {
    return {
        ...(definition.unit === undefined ? {} : { unit: definition.unit }),
        ...(definition.description === undefined ? {} : { description: definition.description }),
    };
}

export {
    exceptionAttributes,
    recordSpanException,
    type SpanDuration,
    type SpanOptions,
    SpanRunner,
} from "@destack/environment";
