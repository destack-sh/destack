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
import { StackFrame, type Symbolicator } from "@destack/package/stack";

/** How often metrics export: once a minute, the resolution observability stores points at. */
const METRIC_INTERVAL_MILLISECONDS = 60_000;

/** An OTLP/HTTP export response, reporting rejected items in its partial success. */
const ExportResponse = schema.looseObject({
    partialSuccess: schema
        .looseObject({ errorMessage: schema.string().exactOptional() })
        .exactOptional(),
});

/** The attribute an exception record keeps its symbolicated frames in, as JSON. */
export const FRAMES_ATTRIBUTE = "destack.exception.frames";

/** The path below which an origin receives OTLP/HTTP exports. */
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

/** The emitter of an OTLP export, as its receiver verified it. */
export interface OtlpEmitter {
    /** The space of the installation, or the machine whose own telemetry it is. */
    readonly scope: string;
    /** The installation, absent for the scope's own telemetry. */
    readonly installation?: Identifier<"installation">;
    /** The workload instance, absent for a page. */
    readonly instance?: Identifier<"instance">;
    /** The digest of the emitting build's manifest, absent for a release no host resolved. */
    readonly build?: Digest;
    /** The browser a page's export came from, as the forwarding server saw it. */
    readonly visitor?: {
        /** The browser's address. */
        readonly address: string;
        /** The browser's user agent. */
        readonly userAgent: string;
    };
    /** The person signed in to the page of the export, as the forwarding server verified them. */
    readonly person?: Identifier<"user">;
}

/** A receiver of OTLP/JSON exports, recording each under the emitter its caller verified. */
export interface OtlpReceiver {
    /** Take an export of a signal from a verified emitter. */
    receive(emitter: OtlpEmitter, signal: OtlpSignal, body: unknown): Promise<void>;
}

/** Where an exporter posts OTLP/HTTP requests, as OpenTelemetry's endpoint and headers. */
export interface OtlpTarget {
    /** The base URL each signal posts below at `/v1/<signal>`, as OTEL_EXPORTER_OTLP_ENDPOINT. */
    readonly endpoint: string;
    /** Read the headers of each request, such as a current authorization. */
    readonly headers?: () => Record<string, string>;
    /** Whether a request outlives its page, as a page's exports must when it hides. */
    readonly isKeepalive?: boolean;
    /** Send a request, `fetch` by default, or a local client such as a machine's. */
    readonly fetch?: (url: string, options: RequestInit) => Promise<Response>;
}

/** Deliver one OTLP/JSON export request of a signal, rejecting a refused delivery. */
export type OtlpSend = (signal: OtlpSignal, body: Uint8Array) => Promise<void>;

/** Exports every signal as OTLP/JSON through one delivery. */
export class OtlpExporter {
    /** Deliver one request. */
    readonly #send: OtlpSend;
    /** Report a failed delivery, which the SDK otherwise drops silently. */
    readonly #report: (error: Error) => void;
    /** The symbolicator of the build the exceptions ran, absent where none is at hand. */
    readonly #symbolicator: Pick<Symbolicator, "frames"> | undefined;
    /** The deliveries in flight. */
    readonly #pending = new Set<Promise<void>>();

    /** The log record exporter, symbolicating exceptions first. */
    readonly logs: LogRecordExporter = {
        export: (records: ReadableLogRecord[], done: (result: ExportResult) => void) => {
            const exporting = this.#symbolicate(records).then(() =>
                this.#export("logs", JsonLogsSerializer.serializeRequest(records), done),
            );
            this.#pending.add(exporting);
            void exporting.finally(() => this.#pending.delete(exporting));
        },
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

    /** Export through a delivery, reporting its failures, symbolicating through a build. */
    constructor(
        send: OtlpSend,
        report: (error: Error) => void,
        symbolicator?: Pick<Symbolicator, "frames">,
    ) {
        this.#send = send;
        this.#report = report;
        this.#symbolicator = symbolicator;
    }

    /** Export over OTLP/HTTP below a target's endpoint, each signal at its `/v1/<signal>` path. */
    static http(
        target: OtlpTarget,
        report: (error: Error) => void,
        symbolicator?: Pick<Symbolicator, "frames">,
    ): OtlpExporter {
        return new OtlpExporter((signal, body) => post(target, signal, body), report, symbolicator);
    }

    /** Build telemetry options that sample an owner's traces. */
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

    /** Attach each exception record's frames, symbolicated through the build it ran. */
    async #symbolicate(records: readonly ReadableLogRecord[]): Promise<void> {
        // leave records as they are without a build
        const symbolicator = this.#symbolicator;
        if (symbolicator === undefined) {
            return;
        }

        // resolve each stack trace, keeping a record's raw trace when its resolution fails
        for (const record of records) {
            const stack = record.attributes["exception.stacktrace"];
            if (typeof stack !== "string") {
                continue;
            }
            try {
                const frames = await symbolicator.frames(StackFrame.read(stack));
                Object.assign(record.attributes, { [FRAMES_ATTRIBUTE]: JSON.stringify(frames) });
            } catch (cause) {
                this.#report(cause instanceof Error ? cause : new Error(String(cause)));
            }
        }
    }

    /** Wait for the deliveries in flight. */
    async #drain(): Promise<void> {
        await Promise.all(this.#pending);
    }
}

/** Post one OTLP/JSON request of a signal to its path below a target. */
async function post(target: OtlpTarget, signal: OtlpSignal, body: Uint8Array): Promise<void> {
    // view the serialized bytes over their ArrayBuffer
    const buffer = body.buffer;
    if (!(buffer instanceof ArrayBuffer)) {
        throw new TypeError("otlp request bytes must sit in an ArrayBuffer");
    }
    const bytes = new Uint8Array(buffer, body.byteOffset, body.byteLength);

    // post the request to the signal's path
    const send = target.fetch ?? fetch;
    const response = await send(`${target.endpoint}/v1/${signal}`, {
        method: "POST",
        headers: { "content-type": "application/json", ...target.headers?.() },
        body: bytes,
        ...(target.isKeepalive === true ? { keepalive: true } : {}),
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
