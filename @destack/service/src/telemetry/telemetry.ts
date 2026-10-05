import {
    context,
    propagation,
    trace,
    telemetry,
    type Histogram,
    type Attributes,
    type Span,
    type Context,
    SpanKind,
    SpanStatusCode,
} from "@destack/telemetry";
import { AsyncIteratorClass, setGlobalOtelConfig } from "@orpc/shared";
import { isServiceError } from "../error/index.ts";
import type {} from "@destack/package/import-meta";

/** The instrumenting package. */
const manifest = import.meta.destack.package;

/** Record RPC calls, including streams. */
export class ServiceTelemetry {
    /** The call durations, in seconds. */
    readonly #duration: Histogram;
    /** The RPC span kind. */
    readonly #kind: SpanKind;

    /** Create the telemetry. */
    constructor(kind: "client" | "server") {
        // create the duration histogram
        instrumentService();
        this.#kind = kind === "client" ? SpanKind.CLIENT : SpanKind.SERVER;
        this.#duration = telemetry
            .scope(manifest)
            .meter.createHistogram(`rpc.${kind}.call.duration`, { unit: "s" });
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

        return telemetry
            .scope(manifest)
            .tracer.startActiveSpan(path.join("/"), { kind: this.#kind, attributes }, (span) =>
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
        const started = performance.now();
        const active = context.active();
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
        // keep the failure until the stream closes
        let failure: unknown;
        const iterator = stream[Symbol.asyncIterator]();

        return new AsyncIteratorClass(
            async () => {
                try {
                    return await context.with(active, () => iterator.next());
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
                        await context.with(active, () => iterator.return?.());
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
            attributes["error.type"] = isServiceError(error) ? String(error.status) : "internal";
        }

        // mark failed calls before recording the call
        if (attributes["error.type"] !== undefined) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }
        span.setAttributes(attributes);
        span.end();
        this.#duration.record((performance.now() - started) / 1000, attributes);
    }
}

/** Enable procedure and HTTP tracing. */
function instrumentService(): void {
    setGlobalOtelConfig({
        tracer: trace.getTracer("@destack/service"),
        trace,
        context,
        propagation,
    });
}

/** Report whether a call's result is a stream. */
function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
    return value !== null && typeof value === "object" && Symbol.asyncIterator in value;
}
