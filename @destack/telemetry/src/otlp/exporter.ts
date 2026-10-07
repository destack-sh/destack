import { type Digest, type Identifier, schema } from "@destack/schema";
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

/** How often metrics export: once a minute, the resolution observability stores points at. */
const METRIC_INTERVAL_MILLISECONDS = 60_000;

/** An OTLP/HTTP export response, reporting rejected items in its partial success. */
const ExportResponse = schema.looseObject({
    partialSuccess: schema
        .looseObject({ errorMessage: schema.string().exactOptional() })
        .exactOptional(),
});

/** The path below which an origin receives OTLP/HTTP exports: an installation's origin its pages', a host's egress origin its instances'. */
export const OTLP_ORIGIN_PATH = "/.destack/telemetry";

/** The OTLP signals, named as their HTTP paths name them. */
export type OtlpSignal = "traces" | "logs" | "metrics";

/** The OTLP signals by their HTTP paths below the origin path. */
const SIGNAL_PATHS: ReadonlyMap<string, OtlpSignal> = new Map([
    [`${OTLP_ORIGIN_PATH}/v1/traces`, "traces"],
    [`${OTLP_ORIGIN_PATH}/v1/logs`, "logs"],
    [`${OTLP_ORIGIN_PATH}/v1/metrics`, "metrics"],
]);

/** Read the OTLP signals of origin paths. */
export const OtlpSignal = {
    /** Read the signal an OTLP/HTTP path below the origin path takes, absent for any other path. */
    at(pathname: string): OtlpSignal | undefined {
        return SIGNAL_PATHS.get(pathname);
    },
};

/** The emitter of an OTLP export, as its receiver verified it: the export itself claims none of it. */
export interface OtlpEmitter {
    /** The space of the installation, or the machine whose own telemetry it is. */
    readonly scope: string;
    /** The installation, absent for the scope's own telemetry. */
    readonly installation?: Identifier<"installation">;
    /** The workload instance, absent for a page. */
    readonly instance?: Identifier<"instance">;
    /** The digest of the emitting build's manifest, absent for a release no host resolved. */
    readonly build?: Digest;
    /** The browser a page's export came from, as the server forwarding it saw it, absent for a workload. */
    readonly visitor?: {
        /** The browser's address. */
        readonly address: string;
        /** The browser's user agent. */
        readonly userAgent: string;
    };
    /** The person signed in to the page whose export it is, as the server forwarding it verified them. */
    readonly person?: Identifier<"user">;
}

/** A receiver of OTLP/JSON exports, recording each under the emitter its caller verified. */
export interface OtlpReceiver {
    /** Take an export of a signal from a verified emitter. */
    receive(emitter: OtlpEmitter, signal: OtlpSignal, body: unknown): Promise<void>;
}

/** Deliver one OTLP/JSON export request of a signal, rejecting a refused delivery. */
export type OtlpSend = (signal: OtlpSignal, body: Uint8Array) => Promise<void>;

/** Exports every signal as OTLP/JSON through one delivery, such as HTTP to an observability service. */
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
        return new OtlpExporter(
            (signal, body) =>
                post(fetch, `${endpoint}/v1/${signal}`, signal, body, {
                    headers: { authorization: authorization() },
                }),
            report,
        );
    }

    /** Export a page's signals over OTLP/HTTP to its own origin, as its session, delivering as the page unloads. */
    static origin(
        report: (error: Error) => void,
        send: (url: string, options: RequestInit) => Promise<Response> = fetch,
    ): OtlpExporter {
        return new OtlpExporter(
            (signal, body) =>
                post(send, `${OTLP_ORIGIN_PATH}/v1/${signal}`, signal, body, { keepalive: true }),
            report,
        );
    }

    /** Build telemetry options sampling an owner's traces and batching its signals through this exporter. */
    options(
        owner: { readonly name: string; readonly version: string },
        options: { readonly ratio?: number } = {},
    ): TelemetryOptions {
        // keep a ratio of traces, and every failed or slow one
        const tail = new TailSampler(
            new BatchSpanProcessor({ exporter: this.traces }),
            new BatchLogRecordProcessor({ exporter: this.logs }),
        );

        return {
            name: owner.name,
            version: owner.version,
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

/** Post one OTLP/JSON request of a signal, refusing a delivery the endpoint refused in whole or in part. */
async function post(
    send: (url: string, options: RequestInit) => Promise<Response>,
    url: string,
    signal: OtlpSignal,
    body: Uint8Array,
    delivery: { readonly headers?: Record<string, string>; readonly keepalive?: boolean },
): Promise<void> {
    // view the serialized bytes over their ArrayBuffer
    const buffer = body.buffer;
    if (!(buffer instanceof ArrayBuffer)) {
        throw new TypeError("otlp request bytes must sit in an ArrayBuffer");
    }
    const bytes = new Uint8Array(buffer, body.byteOffset, body.byteLength);

    // post the request to the signal's path
    const response = await send(url, {
        method: "POST",
        headers: { "content-type": "application/json", ...delivery.headers },
        body: bytes,
        ...(delivery.keepalive === undefined ? {} : { keepalive: delivery.keepalive }),
    });

    // refuse a delivery the endpoint refused
    const text = await response.text();
    if (!response.ok) {
        throw new Error(`otlp ${signal} export failed with ${response.status}: ${text}`);
    }

    // refuse a delivery the endpoint accepted only in part
    const partial = text === "" ? undefined : ExportResponse.parse(JSON.parse(text)).partialSuccess;
    if (partial?.errorMessage !== undefined && partial.errorMessage !== "") {
        throw new Error(`otlp ${signal} export partly rejected: ${partial.errorMessage}`);
    }
}
