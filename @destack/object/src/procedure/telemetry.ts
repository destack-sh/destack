import type { Clock, Environment } from "@destack/environment";
import {
    type ContextManager,
    type Histogram,
    type Attributes,
    type Span,
    type Context,
    type Tracer,
    SpanKind,
    SpanStatusCode,
    trace,
} from "@destack/telemetry";
import { AsyncIteratorClass } from "@orpc/shared";
import { CallError } from "../error/index.ts";
import type {} from "@destack/package/import-meta";

/** The instrumenting package. */
const manifest = import.meta.destack.package;

/** Record RPC calls, including streams. */
export class CallTelemetry {
    /** The package's tracer. */
    readonly #tracer: Tracer;
    /** The call durations, in seconds. */
    readonly #duration: Histogram;
    /** The RPC span kind. */
    readonly #kind: SpanKind;
    /** The clock calls are timed on. */
    readonly #clock: Clock;
    /** The manager of the context calls run in. */
    readonly #context: ContextManager;

    /** Create the telemetry on an environment's tracers, meters and clock. */
    constructor(kind: "client" | "server", environment: Environment) {
        // create the tracer and the duration histogram
        const { tracers, meters } = environment.telemetry;
        this.#kind = kind === "client" ? SpanKind.CLIENT : SpanKind.SERVER;
        this.#tracer = tracers.getTracer(manifest.name, manifest.version);
        this.#duration = meters
            .getMeter(manifest.name, manifest.version)
            .createHistogram(`rpc.${kind}.call.duration`, { unit: "s" });
        this.#clock = environment.clock;
        this.#context = environment.telemetry.context;
    }

    /** Measure a call. */
    invoke(
        path: readonly string[],
        next: () => Promise<unknown>,
        labels: Attributes = {},
    ): Promise<unknown> {
        // label the call by its path and the caller's labels
        const attributes: Attributes = {
            "rpc.system.name": "orpc",
            "rpc.method": path.join("/"),
            ...labels,
        };

        // run the call in its span, a child of the active one
        const parent = this.#context.active();
        const span = this.#tracer.startSpan(
            path.join("/"),
            { kind: this.#kind, attributes },
            parent,
        );

        return this.#context.with(trace.setSpan(parent, span), () =>
            this.#invoke(next, attributes, span),
        );
    }

    /** Trace a call until its value or stream completes. */
    async #invoke(
        next: () => Promise<unknown>,
        attributes: Attributes,
        span: Span,
    ): Promise<unknown> {
        // start the timer and span
        const started = this.#clock.now();
        const active = this.#context.active();
        try {
            const result = await next();

            // trace a stream until it ends
            if (isAsyncIterable(result)) {
                return this.#watch(result, started, attributes, span, active);
            }

            // record a value call
            this.#record(started, attributes, span);

            return result;
        } catch (error) {
            this.#record(started, attributes, span, error);
            throw error;
        }
    }

    /** Trace a stream until it completes or is cancelled. */
    #watch(
        stream: AsyncIterable<unknown>,
        started: number,
        attributes: Attributes,
        span: Span,
        active: Context,
    ): AsyncIteratorClass<unknown, unknown, void> {
        // record the failure until the stream closes
        let failure: unknown;
        const iterator = stream[Symbol.asyncIterator]();

        return new AsyncIteratorClass(
            async () => {
                try {
                    return await this.#context.with(active, () => iterator.next());
                } catch (error) {
                    failure = error;
                    throw error;
                }
            },
            async (reason) => {
                // record the stream on close
                try {
                    if (reason !== "next") {
                        attributes["error.type"] = "cancelled";
                        await this.#context.with(active, () => iterator.return?.());
                    }
                } catch (error) {
                    failure = error;
                    throw error;
                } finally {
                    this.#record(started, attributes, span, failure);
                }
            },
        );
    }

    /** Record a call's outcome and duration. */
    #record(started: number, attributes: Attributes, span: Span, error?: unknown): void {
        // label the failure and end the span
        if (error !== undefined) {
            const known = CallError.of(error);
            attributes["error.type"] = known === undefined ? "internal" : String(known.status);
        }

        // mark failed calls before recording the call
        if (attributes["error.type"] !== undefined) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }
        span.setAttributes(attributes);
        span.end();
        this.#duration.record((this.#clock.now() - started) / 1000, attributes);
    }
}

/** Report whether a call's result is a stream. */
function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
    return value !== null && typeof value === "object" && Symbol.asyncIterator in value;
}
