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
import { ServiceError } from "../error/index.ts";
import type {} from "@destack/package/import-meta";

/** The package declaring service instrumentation. */
const manifest = import.meta.destack.package;

/** Record complete RPC calls, including streamed results. */
export class ServiceTelemetry {
    /** Call durations in seconds. */
    readonly #duration: Histogram;
    /** The RPC span kind. */
    readonly #kind: SpanKind;

    /** Connect tracing and metrics to the host's providers. */
    constructor(kind: "client" | "server") {
        instrumentService();
        this.#kind = kind === "client" ? SpanKind.CLIENT : SpanKind.SERVER;
        this.#duration = telemetry
            .scope(manifest)
            .meter.createHistogram(`rpc.${kind}.call.duration`, {
                unit: "s",
                advice: {
                    explicitBucketBoundaries: [
                        0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1, 2.5, 5, 7.5, 10,
                    ],
                },
            });
    }

    /** Measure a call without retaining its inputs or results. */
    invoke(path: readonly string[], next: () => Promise<unknown>): Promise<unknown> {
        const attributes: Attributes = { "rpc.system.name": "orpc", "rpc.method": path.join("/") };

        return telemetry
            .scope(manifest)
            .tracer.startActiveSpan(path.join("/"), { kind: this.#kind, attributes }, (span) =>
                this.#invoke(next, attributes, span),
            );
    }

    /** Keep the span open until the value or stream completes. */
    async #invoke(
        next: () => Promise<unknown>,
        attributes: Attributes,
        span: Span,
    ): Promise<unknown> {
        // use declared procedure names as bounded metric labels
        const started = performance.now();
        const active = context.active();
        try {
            const result = await next();
            if (result !== null && typeof result === "object" && Symbol.asyncIterator in result) {
                return this.#watch(
                    result as AsyncIterable<unknown>,
                    started,
                    attributes,
                    span,
                    active,
                );
            }

            this.#record(started, attributes, span);

            return result;
        } catch (error) {
            this.#record(started, attributes, span, error);
            throw error;
        }
    }

    /** Retain timing until a consumer completes or cancels the stream. */
    #watch(
        stream: AsyncIterable<unknown>,
        started: number,
        attributes: Attributes,
        span: Span,
        active: Context,
    ): AsyncIteratorClass<unknown, unknown, void> {
        // retain failures until iterator cleanup records the final outcome
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
                // close even streams cancelled before their first value
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

    /** Record bounded failure labels and elapsed seconds. */
    #record(started: number, attributes: Attributes, span: Span, error?: unknown): void {
        // label the failure and end the span
        if (error !== undefined) {
            attributes["error.type"] =
                error instanceof ServiceError ? String(error.status) : "internal";
        }
        if (attributes["error.type"]) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }
        span.setAttributes(attributes);
        span.end();
        this.#duration.record((performance.now() - started) / 1000, attributes);
    }
}

/** Enable procedure and HTTP tracing through the application's telemetry providers. */
function instrumentService(): void {
    setGlobalOtelConfig({
        tracer: trace.getTracer("@destack/service"),
        trace,
        context,
        propagation,
    });
}
