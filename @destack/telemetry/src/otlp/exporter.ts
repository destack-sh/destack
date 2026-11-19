import { OriginPath } from "@destack/package";
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
import type { LineExporter } from "@destack/environment";

/** How long one export may take by default before it fails: OTLP's OTEL_EXPORTER_OTLP_TIMEOUT. */
const EXPORT_TIMEOUT_MILLISECONDS = 10_000;

/** The base URL OpenTelemetry's exporters post each signal below, as its specification sets. */
const ENDPOINT_VARIABLE = "OTEL_EXPORTER_OTLP_ENDPOINT";

/** The variable setting how long spans batch before an export, as OpenTelemetry names it. */
const DELAY_VARIABLE = "OTEL_BSP_SCHEDULE_DELAY";

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

/** The OTLP signals, named as their HTTP paths name them. */
export type OtlpSignal = "traces" | "logs" | "metrics";

/** The OTLP signals by their HTTP paths below the origin path. */
const SIGNAL_PATHS: ReadonlyMap<string, OtlpSignal> = new Map([
    [`${OriginPath.telemetry}/v1/traces`, "traces"],
    [`${OriginPath.telemetry}/v1/logs`, "logs"],
    [`${OriginPath.telemetry}/v1/metrics`, "metrics"],
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

/** Where an exporter posts OTLP/HTTP requests, as OpenTelemetry's endpoint and headers. */
export interface OtlpTarget {
    /** The base URL each signal posts below at `/v1/<signal>`, as OTEL_EXPORTER_OTLP_ENDPOINT. */
    readonly endpoint: string;
    /** Read the headers of each request, such as a current authorization. */
    readonly headers?: () => Record<string, string>;
    /** Whether a request outlives its page, as a page's exports must when it hides. */
    readonly isKeepalive?: boolean;
    /** Send a request: an environment's network fetch, or a local client such as a machine's. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** Deliver one OTLP/JSON export request of a signal, rejecting a refused or aborted delivery. */
export type OtlpSend = (signal: OtlpSignal, body: Uint8Array, abort: AbortSignal) => Promise<void>;

/** How an exporter reads exceptions and how long it lets a delivery take. */
export interface OtlpExporterOptions {
    /** The symbolicator of the build the exceptions ran, absent where none is at hand. */
    readonly symbolicator?: Pick<Symbolicator, "frames">;
    /** How long one delivery may take before its export fails, in milliseconds. */
    readonly timeout?: number;
}

/** Exports every signal as OTLP/JSON through one delivery. */
export class OtlpExporter {
    /** How long spans and records batch before an export by default: OpenTelemetry's 5 s. */
    static readonly DEFAULT_DELAY = 5000;

    /** Deliver one request. */
    readonly #send: OtlpSend;
    /** Report a failed delivery, which the SDK otherwise drops silently. */
    readonly #report: (error: Error) => void;
    /** The symbolicator of the build the exceptions ran, absent where none is at hand. */
    readonly #symbolicator: Pick<Symbolicator, "frames"> | undefined;
    /** The deliveries in flight. */
    readonly #pending = new Set<Promise<void>>();
    /** How long one delivery may take before its export fails, in milliseconds. */
    readonly #timeout: number;

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
    constructor(send: OtlpSend, report: (error: Error) => void, options: OtlpExporterOptions = {}) {
        // keep the delivery, the report, the symbolicator and the timeout
        this.#send = send;
        this.#report = report;
        this.#symbolicator = options.symbolicator;
        this.#timeout = options.timeout ?? EXPORT_TIMEOUT_MILLISECONDS;
    }

    /** Export over OTLP/HTTP below a target's endpoint, each signal at its `/v1/<signal>` path. */
    static http(
        target: OtlpTarget,
        report: (error: Error) => void,
        symbolicator?: Pick<Symbolicator, "frames">,
    ): OtlpExporter {
        return new OtlpExporter(
            (signal, body, abort) => OtlpExporter.post(target, signal, body, abort),
            report,
            symbolicator === undefined ? {} : { symbolicator },
        );
    }

    /** Add an export of spans and log records to the endpoint a process's variables set. */
    static endpoint(
        options: TelemetryOptions,
        variables: Readonly<Record<string, string | undefined>>,
        fetch: (request: Request) => Promise<Response>,
    ): TelemetryOptions {
        // leave the options without an endpoint
        const endpoint = variables[ENDPOINT_VARIABLE];
        if (endpoint === undefined) {
            return options;
        }

        // export every span and record beside the options' own processors, batched as the variables set
        const exporter = OtlpExporter.http({ endpoint, fetch }, options.report);
        const scheduledDelayMillis = OtlpExporter.delay(variables);

        return {
            ...options,
            signals: (meters) => {
                // add a batch processor per signal to the options' own setup
                const { traces, logs } = options.signals(meters);
                const spans = new BatchSpanProcessor({
                    exporter: exporter.traces,
                    scheduledDelayMillis,
                });
                const records = new BatchLogRecordProcessor({
                    exporter: exporter.logs,
                    scheduledDelayMillis,
                });

                return {
                    traces: {
                        ...traces,
                        spanProcessors: [...(traces.spanProcessors ?? []), spans],
                    },
                    logs: { ...logs, processors: [...(logs.processors ?? []), records] },
                };
            },
        };
    }

    /** Read how long spans and records batch before an export from variables, in milliseconds. */
    static delay(variables: Readonly<Record<string, string | undefined>>): number {
        const set = variables[DELAY_VARIABLE];

        return set === undefined
            ? OtlpExporter.DEFAULT_DELAY
            : schema.number().int().positive().parse(Number(set));
    }

    /** Post spans as one OTLP/JSON request to the endpoint variables set, none without one. */
    static spans(
        variables: Readonly<Record<string, string | undefined>>,
        fetch: (request: Request) => Promise<Response>,
    ): ((spans: readonly ReadableSpan[]) => Promise<void>) | undefined {
        // post nothing without an endpoint
        const endpoint = variables[ENDPOINT_VARIABLE];
        if (endpoint === undefined) {
            return undefined;
        }

        return async (spans) => {
            // serialize the spans, then post them as the traces signal
            const body = JsonTraceSerializer.serializeRequest([...spans]);
            if (body === undefined) {
                throw new TypeError("trace export failed: the spans did not serialize");
            }

            await OtlpExporter.post(
                { endpoint, fetch },
                "traces",
                body,
                new AbortController().signal,
            );
        };
    }

    /** Post one OTLP/JSON export request of a signal to a target, rejecting a refused delivery. */
    static async post(
        target: OtlpTarget,
        signal: OtlpSignal,
        body: Uint8Array,
        abort: AbortSignal,
    ): Promise<void> {
        // view the serialized bytes over their ArrayBuffer
        const buffer = body.buffer;
        if (!(buffer instanceof ArrayBuffer)) {
            throw new TypeError("otlp request bytes must be in an ArrayBuffer");
        }
        const bytes = new Uint8Array(buffer, body.byteOffset, body.byteLength);

        // post the request to the signal's path
        const request = new Request(`${target.endpoint}/v1/${signal}`, {
            method: "POST",
            headers: { "content-type": "application/json", ...target.headers?.() },
            body: bytes,
            signal: abort,
            ...(target.isKeepalive === true ? { keepalive: true } : {}),
        });
        const response = await target.fetch(request);

        // refuse a delivery the endpoint refused
        const text = await response.text();
        if (!response.ok) {
            throw new Error(`otlp '${signal}' export failed with ${response.status}: '${text}'`);
        }

        // refuse a delivery the endpoint accepted only in part
        const partial =
            text === "" ? undefined : ExportResponse.parse(JSON.parse(text)).partialSuccess;
        if (partial?.errorMessage !== undefined && partial.errorMessage !== "") {
            throw new Error(`otlp '${signal}' export partly rejected: ${partial.errorMessage}`);
        }
    }

    /** Build telemetry options that sample an owner's traces. */
    options(
        owner: { readonly name: string; readonly version: string },
        options: {
            readonly ratio?: number;
            /** The process's own log, printing every record beside the export. */
            readonly local?: LineExporter;
            /** How long spans and records batch before an export, in milliseconds, else 5 s. */
            readonly delay?: number;
        } = {},
    ): TelemetryOptions {
        return {
            name: owner.name,
            version: owner.version,
            report: this.#report,
            signals: (meters) => {
                // sample a ratio of traces and every failed or slow one, batched by the delay
                const scheduledDelayMillis = options.delay ?? OtlpExporter.DEFAULT_DELAY;
                const tail = new TailSampler(
                    new BatchSpanProcessor({ exporter: this.traces, scheduledDelayMillis }),
                    new BatchLogRecordProcessor({ exporter: this.logs, scheduledDelayMillis }),
                    meters,
                );

                return {
                    traces: {
                        sampler: new RatioSampler(options.ratio ?? 1),
                        spanProcessors:
                            options.local === undefined
                                ? [tail.spans]
                                : [tail.spans, options.local.spans],
                    },
                    logs: {
                        processors:
                            options.local === undefined
                                ? [tail.logs]
                                : [tail.logs, options.local.processor],
                    },
                };
            },
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
            const error = new Error(`otlp '${signal}' request could not be serialized`);
            this.#report(error);
            done({ code: ExportResultCode.FAILED, error });

            return;
        }

        // deliver it within the export timeout, failing one that outlasts it as OTLP exporters do, and report a failure
        const abort = AbortSignal.timeout(this.#timeout);
        const timedOut = new Promise<never>((_resolve, reject) => {
            abort.addEventListener("abort", () => reject(abort.reason), { once: true });
        });
        const delivery = Promise.race([this.#send(signal, body, abort), timedOut]).then(
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
