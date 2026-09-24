import { type ExportResult, ExportResultCode } from "@opentelemetry/core";
import type { ReadableSpan, SpanExporter } from "@opentelemetry/sdk-trace";
import type { LogRecordExporter, ReadableLogRecord } from "@opentelemetry/sdk-logs";
import type { PushMetricExporter, ResourceMetrics } from "@opentelemetry/sdk-metrics";
import { describeSpan, type SpanDescription } from "./span.ts";
import { describeLog, type LogDescription } from "./log.ts";
import { describeMetrics, type MetricCollectionDescription } from "./metric.ts";

/** Deliver exported signals to an inspector and track outstanding deliveries. */
export class Inspector<Value> {
    /** The application's signal consumer. */
    private readonly receive: (value: Value) => void | Promise<void>;
    /** Deliveries awaiting completion. */
    private readonly pending = new Set<Promise<void>>();
    /** Whether this exporter has stopped accepting signals. */
    private isClosed = false;

    /** Inspect signals without retaining their history. */
    constructor(receive: (value: Value) => void | Promise<void>) {
        this.receive = receive;
    }

    /** Deliver signals and report consumer failures through the exporter callback. */
    export(value: Value, callback: (result: ExportResult) => void): void {
        // reject exports after shutdown
        if (this.isClosed) {
            callback({ code: ExportResultCode.FAILED, error: new Error("Inspector is closed.") });
            return;
        }

        // report both synchronous and asynchronous consumer failures
        const operation = Promise.resolve().then(() => this.receive(value));
        this.pending.add(operation);
        operation.then(
            () => {
                this.pending.delete(operation);
                callback({ code: ExportResultCode.SUCCESS });
            },
            (cause: unknown) => {
                this.pending.delete(operation);
                const error = cause instanceof Error ? cause : new Error(String(cause), { cause });
                callback({ code: ExportResultCode.FAILED, error });
            },
        );
    }

    /** Wait for every outstanding delivery and report all failures. */
    async forceFlush(): Promise<void> {
        const results = await Promise.allSettled(this.pending);
        const errors = results
            .filter((result) => result.status === "rejected")
            .map((result) => result.reason);

        if (errors.length > 0) {
            throw new AggregateError(errors, "telemetry inspection failed");
        }
    }

    /** Stop accepting signals and finish outstanding deliveries. */
    shutdown(): Promise<void> {
        this.isClosed = true;

        return this.forceFlush();
    }
}

/** Inspect completed spans through the standard trace exporter API. */
export class TraceInspector extends Inspector<ReadableSpan[]> implements SpanExporter {
    /** Deliver schema-defined span descriptions. */
    constructor(receive: (spans: SpanDescription[]) => void | Promise<void>) {
        super((spans) => receive(spans.map(describeSpan)));
    }
}

/** Inspect structured logs through the standard log exporter API. */
export class LogInspector extends Inspector<ReadableLogRecord[]> implements LogRecordExporter {
    /** Deliver schema-defined log descriptions. */
    constructor(receive: (records: LogDescription[]) => void | Promise<void>) {
        super((records) => receive(records.map(describeLog)));
    }
}

/** Inspect metric collections through the standard metric exporter API. */
export class MetricInspector extends Inspector<ResourceMetrics> implements PushMetricExporter {
    /** Deliver a schema-defined metric collection. */
    constructor(receive: (collection: MetricCollectionDescription) => void | Promise<void>) {
        super((collection) => receive(describeMetrics(collection)));
    }
}
