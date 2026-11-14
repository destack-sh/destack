import {
    type Attributes,
    type Histogram,
    type Link,
    type Meter,
    type MeterProvider,
    type Span,
    SpanKind,
    SpanStatusCode,
    trace,
    type Tracer,
} from "@opentelemetry/api";
import { Failure } from "@destack/error";
import { recordSpanException } from "./exception.ts";
import type { ReadableSpan } from "@opentelemetry/sdk-trace";
import { SpanPath } from "./span.ts";
import type { Telemetry } from "./telemetry.ts";

/** The instrumentation scope of the environment's boundary spans and durations: this package. */
export const ENVIRONMENT_SCOPE = "@destack/environment";

/** The bucket boundaries of a duration in seconds: from 100 µs, as statements take, to 10 s. */
export const DURATION_BUCKETS = [
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10,
];

/** The OpenTelemetry span kind of each end of a call. */
const SPAN_KINDS = { client: SpanKind.CLIENT, server: SpanKind.SERVER } as const;

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The nanoseconds in a second, as span times count seconds and nanoseconds. */
const NANOSECONDS_PER_SECOND = 1_000_000_000;

/** How a span runs: its kind, and the duration it records. */
export interface SpanOptions {
    /** The span's kind: the client or server end of a call, internal when absent. */
    readonly kind?: "client" | "server";
    /** The duration's name and bounded attributes, `<span>.duration` by default, false for none. */
    readonly duration?: SpanDuration | false;
    /** The spans that caused this one apart from its parent, such as the commits waking a pass. */
    readonly links?: readonly Link[];
}

/** The duration a span records: its metric's name and its bounded attributes, never identifiers. */
export interface SpanDuration {
    /** The metric's name, such as `rpc.server.call.duration`, else `<span>.duration`. */
    readonly name?: string;
    /** The bounded attributes, or a function reading them once the work ends, none by default. */
    readonly attributes?: Attributes | (() => Attributes);
}

/** Run work in a child span named by a literal name, or read the active span to add attributes. */
export interface SpanRunner {
    /** Run work in a child span of the active one, recording its failure and its seconds. */
    <Result>(
        name: string,
        attributes: Attributes,
        run: (span: Span) => Result | Promise<Result>,
        options?: SpanOptions,
    ): Promise<Result>;
    /** Read the active span, such as the call the platform traces. */
    readonly current: () => Span | undefined;
    /** Time work apart from any span, recording its seconds and failure, and handing them over. */
    readonly measure: <Result>(
        name: string,
        attributes: Attributes,
        run: () => Promise<Result>,
        measured?: (seconds: number) => void,
    ) => Promise<Result>;
}

/** The runners of spans: the one way work runs in a span, its duration and its totals. */
export const SpanRunner = {
    /** Run work in child spans of a tracer, each active in the telemetry's context flow. */
    of(
        tracer: Tracer,
        meter: Pick<Meter, "createHistogram">,
        telemetry: Pick<Telemetry, "context" | "clock" | "monotonic">,
    ): SpanRunner {
        // declare each span's duration once, on its first record
        const { context, clock, monotonic } = telemetry;
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

        // run the work in a child span on the clock, ending it with the work or with the stream it answers
        const run = async <Result>(
            name: string,
            attributes: Attributes,
            work: (span: Span) => Result | Promise<Result>,
            how: SpanOptions = {},
        ): Promise<Result> => {
            // start the span below the active one, entered in the span path
            const parent = context.active();
            const kind = how.kind === undefined ? SpanKind.INTERNAL : SPAN_KINDS[how.kind];
            const startTime = clock.now();
            const links = [...(how.links ?? [])];
            const span = tracer.startSpan(name, { attributes, kind, startTime, links }, parent);
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
                const failed =
                    failure === undefined ? {} : { "error.type": Failure.of(failure).code };
                if (failure !== undefined) {
                    recordSpanException(span, failure);
                    span.setStatus({ code: SpanStatusCode.ERROR });
                }
                span.setAttributes({ ...failed, ...SpanPath.totals(entered) });
                span.end(clock.now());
                if (how.duration !== false) {
                    const seconds = (monotonic.now() - started) / MILLISECONDS_PER_SECOND;
                    const bounded = how.duration?.attributes ?? {};
                    const recorded = typeof bounded === "function" ? bounded() : bounded;
                    durations(how.duration?.name ?? `${name}.duration`).record(seconds, {
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

        // time work apart from a span on the host, noting the failure it ends in by its call error code
        const measure = async <Result>(
            name: string,
            attributes: Attributes,
            work: () => Promise<Result>,
            measured?: (seconds: number) => void,
        ): Promise<Result> => {
            const started = monotonic.now();
            let failure: unknown;
            try {
                return await work();
            } catch (error) {
                failure = error;
                throw error;
            } finally {
                const seconds = (monotonic.now() - started) / MILLISECONDS_PER_SECOND;
                const failed =
                    failure === undefined ? {} : { "error.type": Failure.of(failure).code };
                durations(name).record(seconds, { ...attributes, ...failed });
                measured?.(seconds);
            }
        };

        return Object.assign(run, { current: () => trace.getSpan(context.active()), measure });
    },
};

/** Record an ended span's `<span>.duration` on its scope's meter, as the runner's spans do. */
export function recordSpanDuration(meters: MeterProvider, span: ReadableSpan): void {
    // read its seconds and its failure
    const [seconds, nanoseconds] = span.duration;
    const failure = span.attributes["error.type"];
    const failed = typeof failure === "string" ? { "error.type": failure } : {};

    // record it on the meter of the scope that emitted it
    const { name, version } = span.instrumentationScope;
    meters
        .getMeter(name, version)
        .createHistogram(`${span.name}.duration`, {
            unit: "s",
            advice: { explicitBucketBoundaries: DURATION_BUCKETS },
        })
        .record(seconds + nanoseconds / NANOSECONDS_PER_SECOND, failed);
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
