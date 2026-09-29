import type { Attributes } from "@opentelemetry/api";
import { type ExportResult, ExportResultCode } from "@opentelemetry/core";
import {
    JsonLogsSerializer,
    JsonMetricsSerializer,
    JsonTraceSerializer,
} from "@opentelemetry/otlp-transformer";
import {
    BatchLogRecordProcessor,
    type LogRecordExporter,
    type ReadableLogRecord,
} from "@opentelemetry/sdk-logs";
import {
    AggregationTemporality,
    AggregationType,
    InstrumentType,
    PeriodicExportingMetricReader,
    type PushMetricExporter,
    type ResourceMetrics,
} from "@opentelemetry/sdk-metrics";
import { BatchSpanProcessor, type ReadableSpan, type SpanExporter } from "@opentelemetry/sdk-trace";
import type { TelemetryOptions } from "../sdk/index.ts";
import { RatioSampler } from "../trace/sampler.ts";
import { TailSampler } from "../trace/tail.ts";

/** How often metrics export: once a minute, the resolution monitors store points at. */
const METRIC_INTERVAL_MILLISECONDS = 60_000;

/** The OTLP signals, named as their HTTP paths name them. */
export type OtlpSignal = "traces" | "logs" | "metrics";

/** Deliver one OTLP/JSON export request of a signal, rejecting a refused delivery. */
export type OtlpSend = (signal: OtlpSignal, body: Uint8Array) => Promise<void>;

/** Exports every signal as OTLP/JSON through one delivery, such as HTTP to a monitor. */
export class OtlpExporter {
    /** Deliver one request. */
    readonly #send: OtlpSend;
    /** Report a failed delivery, which the SDK otherwise drops silently. */
    readonly #report: (error: Error) => void;
    /** The deliveries in flight. */
    readonly #pending = new Set<Promise<void>>();

    /** The log record exporter. */
    readonly logs: LogRecordExporter = {
        export: (records: ReadableLogRecord[], done: (result: ExportResult) => void) =>
            this.#export("logs", JsonLogsSerializer.serializeRequest(records), done),
        shutdown: () => this.#drain(),
        forceFlush: () => this.#drain(),
    };

    /** The span exporter. */
    readonly traces: SpanExporter = {
        export: (spans: ReadableSpan[], done: (result: ExportResult) => void) =>
            this.#export("traces", JsonTraceSerializer.serializeRequest(spans), done),
        shutdown: () => this.#drain(),
        forceFlush: () => this.#drain(),
    };

    /** The metric exporter, exporting deltas and histograms as base-2 exponential histograms. */
    readonly metrics: PushMetricExporter = {
        export: (metrics: ResourceMetrics, done: (result: ExportResult) => void) =>
            this.#export("metrics", JsonMetricsSerializer.serializeRequest(metrics), done),
        shutdown: () => this.#drain(),
        forceFlush: () => this.#drain(),
        selectAggregationTemporality: () => AggregationTemporality.DELTA,
        selectAggregation: (instrument: InstrumentType) =>
            instrument === InstrumentType.HISTOGRAM
                ? { type: AggregationType.EXPONENTIAL_HISTOGRAM }
                : { type: AggregationType.DEFAULT },
    };

    /** Export through a delivery, reporting its failures. */
    constructor(send: OtlpSend, report: (error: Error) => void) {
        this.#send = send;
        this.#report = report;
    }

    /** Export over OTLP/HTTP to an endpoint with a current authorization header. */
    static http(
        endpoint: string,
        authorization: () => string,
        report: (error: Error) => void,
    ): OtlpExporter {
        return new OtlpExporter(async (signal, body) => {
            // post the request to the signal's path
            const response = await fetch(`${endpoint}/v1/${signal}`, {
                method: "POST",
                headers: { "content-type": "application/json", authorization: authorization() },
                body: body as Uint8Array<ArrayBuffer>,
            });

            // refuse a delivery the endpoint refused
            const text = await response.text();
            if (!response.ok) {
                throw new Error(`otlp ${signal} export failed with ${response.status}: ${text}`);
            }

            // refuse a delivery the endpoint accepted only in part
            const partial = text === "" ? undefined : JSON.parse(text).partialSuccess;
            if (partial?.errorMessage) {
                throw new Error(`otlp ${signal} export partly rejected: ${partial.errorMessage}`);
            }
        }, report);
    }

    /** Build telemetry options sampling an owner's traces and batching its signals through this exporter. */
    options(
        owner: { readonly name: string; readonly version: string },
        options: { readonly attributes?: Attributes; readonly ratio?: number } = {},
    ): TelemetryOptions {
        // keep a ratio of traces, and every failed or slow one
        const tail = new TailSampler(
            new BatchSpanProcessor({ exporter: this.traces }),
            new BatchLogRecordProcessor({ exporter: this.logs }),
        );

        return {
            name: owner.name,
            version: owner.version,
            attributes: options.attributes ?? {},
            report: this.#report,
            traces: {
                sampler: new RatioSampler(options.ratio ?? 1),
                spanProcessors: [tail.spans],
            },
            logs: { processors: [tail.logs] },
            metrics: {
                readers: [
                    new PeriodicExportingMetricReader({
                        exporter: this.metrics,
                        exportIntervalMillis: METRIC_INTERVAL_MILLISECONDS,
                    }),
                ],
            },
        };
    }

    /** Deliver one serialized request and complete the export with its outcome. */
    #export(
        signal: OtlpSignal,
        body: Uint8Array | undefined,
        done: (result: ExportResult) => void,
    ): void {
        // fail a request the serializer could not encode
        if (body === undefined) {
            const error = new Error(`otlp ${signal} request could not be serialized`);
            this.#report(error);
            done({ code: ExportResultCode.FAILED, error });

            return;
        }

        // deliver it, reporting a failure
        const delivery = this.#send(signal, body).then(
            () => done({ code: ExportResultCode.SUCCESS }),
            (cause: unknown) => {
                const error = cause instanceof Error ? cause : new Error(String(cause));
                this.#report(error);
                done({ code: ExportResultCode.FAILED, error });
            },
        );
        this.#pending.add(delivery);
        void delivery.finally(() => this.#pending.delete(delivery));
    }

    /** Wait for the deliveries in flight. */
    async #drain(): Promise<void> {
        await Promise.all(this.#pending);
    }
}
