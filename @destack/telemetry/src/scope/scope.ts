import {
    type Attributes,
    SpanKind,
    type Counter,
    type Gauge,
    type UpDownCounter,
    type Histogram,
    type Logger,
    type Meter,
    SeverityNumber,
    type Span,
    SpanStatusCode,
    trace,
    type Tracer,
} from "@opentelemetry/api";
import { type Environment, SpanPath } from "@destack/environment";
import type { PackageVersion } from "@destack/package";
import { DomainError, Failure } from "@destack/error";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    VISIT_EVENT,
    type VisitDetail,
} from "../otlp/analytics.ts";

/** The bucket boundaries of a duration in seconds: from 100 µs, as statements take, to 10 s. */
const DURATION_BUCKETS = [
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10,
];

/** The OpenTelemetry span kind of each end of a call. */
const SPAN_KINDS = { client: SpanKind.CLIENT, server: SpanKind.SERVER } as const;

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The event name of an exception on a log record or a span, as OpenTelemetry names it. */
const EXCEPTION_EVENT = "exception";

/** The causes a captured failure's stack trace lists at most, below its own stack. */
const MAX_CAUSES = 8;

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
    readonly measure: <Value>(
        name: string,
        attributes: Attributes,
        run: () => Promise<Value>,
    ) => Promise<Value>;
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

/** How a span runs: its kind, and the duration it records. */
export interface SpanOptions {
    /** The span's kind: the client or server end of a call, internal when absent. */
    readonly kind?: "client" | "server";
    /** The duration the span records in seconds, `<span>.duration` unless named otherwise. */
    readonly duration?: SpanDuration;
}

/** The duration a span records: its metric's name and its bounded attributes, never identifiers. */
export interface SpanDuration {
    /** The metric's name, such as `rpc.server.call.duration`, else `<span>.duration`. */
    readonly name?: string;
    /** The bounded attributes, or a function reading them once the work ends. */
    readonly attributes: Attributes | (() => Attributes);
}

/** Run work in a child span named by a literal name, or read the active span to add attributes. */
export interface SpanRunner {
    /** Run work in a child span of the active one, recording its failure and, if asked, seconds. */
    <Result>(
        name: string,
        attributes: Attributes,
        run: (span: Span) => Result | Promise<Result>,
        options?: SpanOptions,
    ): Promise<Result>;
    /** Read the active span, such as the call the platform traces. */
    readonly current: () => Span | undefined;
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
    // read the telemetry's context and clocks
    const { context, clock, monotonic } = telemetry;

    // declare each span's duration once, on its first record
    const declared = new Map<string, Histogram>();
    const durations = (name: string): Histogram => {
        const histogram =
            declared.get(name) ??
            meter.createHistogram(name, {
                unit: "s",
                advice: { explicitBucketBoundaries: DURATION_BUCKETS },
            });
        declared.set(name, histogram);

        return histogram;
    };

    return {
        tracer,
        meter,
        logger,
        log: new Log(source, logger, telemetry),
        span: runner(tracer, telemetry, durations),
        measure: async (name, attributes, run) => {
            // time the work on the host, noting the failure it ends in by its call error code
            const started = monotonic.now();
            let failure: unknown;
            try {
                return await run();
            } catch (error) {
                failure = error;
                throw error;
            } finally {
                const seconds = (monotonic.now() - started) / MILLISECONDS_PER_SECOND;
                const failed =
                    failure === undefined ? {} : { "error.type": Failure.of(failure).code };
                durations(name).record(seconds, { ...attributes, ...failed });
            }
        },
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

/** Run work in child spans of a tracer, each active in a context manager's flow. */
function runner(
    tracer: Tracer,
    telemetry: Pick<Environment["telemetry"], "context" | "clock" | "monotonic">,
    durations: (name: string) => Histogram,
): SpanRunner {
    // run the work in a child span on the clock, ending it with the work or with the stream it answers
    const { context, clock, monotonic } = telemetry;
    const run = async <Result>(
        name: string,
        attributes: Attributes,
        work: (span: Span) => Result | Promise<Result>,
        how: SpanOptions = {},
    ): Promise<Result> => {
        // start the span below the active one, entered in the span path
        const parent = context.active();
        const kind = how.kind === undefined ? SpanKind.INTERNAL : SPAN_KINDS[how.kind];
        const span = tracer.startSpan(name, { attributes, kind, startTime: clock.now() }, parent);
        const entered = SpanPath.enter(
            trace.setSpan(parent, span),
            SpanPath.label(name, attributes),
        );
        const started = monotonic.now();
        let isEnded = false;

        // end once, writing the totals and recording the duration
        const end = (failure?: unknown) => {
            if (isEnded) {
                return;
            }
            isEnded = true;
            const failed = failure === undefined ? {} : { "error.type": Failure.of(failure).code };
            if (failure !== undefined) {
                recordSpanException(span, failure);
                span.setStatus({ code: SpanStatusCode.ERROR });
            }
            span.setAttributes({ ...failed, ...SpanPath.totals(entered) });
            span.end(clock.now());
            if (how.duration !== undefined) {
                const seconds = (monotonic.now() - started) / MILLISECONDS_PER_SECOND;
                const bounded = how.duration.attributes;
                const recorded = typeof bounded === "function" ? bounded() : bounded;
                durations(how.duration.name ?? `${name}.duration`).record(seconds, {
                    ...recorded,
                    ...failed,
                });
            }
        };

        // run the work, following a stream it answers until the stream ends
        try {
            const result = await context.with(entered, () => work(span));
            if (isAsyncIterable(result)) {
                return follow(result, (step) => context.with(entered, step), end);
            }
            end();

            return result;
        } catch (error) {
            end(error);
            throw error;
        }
    };

    return Object.assign(run, { current: () => trace.getSpan(context.active()) });
}

/** Pass a stream through in its span, ending the span once it ends, fails or closes early. */
function follow<Stream extends AsyncIterable<unknown>>(
    stream: Stream,
    inSpan: <Value>(step: () => Value) => Value,
    end: (failure?: unknown) => void,
): Stream {
    // step the stream's iterator in the span, ending it at the stream's end or failure
    let iterator: AsyncIterator<unknown> | undefined;
    const source = () => (iterator ??= stream[Symbol.asyncIterator]());
    const step = async (next: () => Promise<IteratorResult<unknown>>) => {
        try {
            const result = await inSpan(next);
            if (result.done === true) {
                end();
            }

            return result;
        } catch (error) {
            end(error);
            throw error;
        }
    };
    const steps = {
        next: (...values: [] | [unknown]) => step(() => source().next(...values)),
        throw: (error?: unknown) => step(() => source().throw?.(error) ?? Promise.reject(error)),
        return: async (value?: unknown) => {
            // close the stream early, as a reader closing it before its end
            try {
                return await inSpan(
                    () => source().return?.(value) ?? Promise.resolve({ done: true, value }),
                );
            } finally {
                end(
                    Failure.error(
                        Failure.http("CLIENT_CLOSED_REQUEST", "the reader closed the stream"),
                    ),
                );
            }
        },
    };

    // answer the stream itself, its iterator and its steps in the span
    return new Proxy(stream, {
        get(target, key, receiver: Stream) {
            if (key === Symbol.asyncIterator) {
                return () => receiver;
            } else if (key === "next" || key === "throw" || key === "return") {
                return steps[key];
            }

            return Reflect.get(target, key, receiver);
        },
    });
}

/** Report whether a value is a stream. */
function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
    return value !== null && typeof value === "object" && Symbol.asyncIterator in value;
}

/** Read the instrument options of a metric definition. */
function options(definition: MetricDefinition<AttributeDomain>) {
    return {
        ...(definition.unit === undefined ? {} : { unit: definition.unit }),
        ...(definition.description === undefined ? {} : { description: definition.description }),
    };
}

/** Describe a failure by the OpenTelemetry exception attributes, its causes below its stack. */
export function exceptionAttributes(error: unknown, isEscaped: boolean): Attributes {
    // write each cause below the stack as a `Caused by:` section, as Java's traces do
    const exception = error instanceof Error ? error : new Error(String(error));
    const sections = [exception.stack ?? `${exception.name}: ${exception.message}`];
    let cause = exception.cause;
    while (cause !== undefined && sections.length <= MAX_CAUSES) {
        // write an error's stack, else a value's JSON, ending at a value
        const section =
            cause instanceof Error
                ? (cause.stack ?? `${cause.name}: ${cause.message}`)
                : JSON.stringify(cause);
        sections.push(`Caused by: ${section}`);
        cause = cause instanceof Error ? cause.cause : undefined;
    }

    return {
        "error.type": errorType(exception),
        "exception.type": exception.name,
        "exception.message": exception.message,
        "exception.stacktrace": sections.join("\n"),
        "exception.escaped": isEscaped,
    };
}

/** Annotate a span with an escaping failure, as the exception event its captured log carries. */
export function recordSpanException(span: Span, error: unknown): void {
    span.addEvent(EXCEPTION_EVENT, exceptionAttributes(error, true));
}

/** Read a failure's error type: its call error code, else its own code, else its name. */
function errorType(exception: Error): string {
    // the call error code a domain error reports
    if (DomainError.is(exception)) {
        return exception.toCallError().code;
    }
    // the failure's own code, such as a system error's
    else if ("code" in exception && typeof exception.code === "string") {
        return exception.code;
    }
    // the failure's name
    else {
        return exception.name;
    }
}
