// export signals over standard OTLP HTTP with explicit URLs and headers
//  select browser package conditions for Web bundles, including Workers, to use fetch
export { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
export {
    AggregationTemporalityPreference,
    OTLPMetricExporter,
} from "@opentelemetry/exporter-metrics-otlp-http";
export type { OTLPMetricExporterOptions } from "@opentelemetry/exporter-metrics-otlp-http";
export { OTLPLogExporter } from "@opentelemetry/exporter-logs-otlp-http";

/** Batch exports and configure their intervals and queue bounds. */
export { BatchSpanProcessor } from "@opentelemetry/sdk-trace";
export { PeriodicExportingMetricReader } from "@opentelemetry/sdk-metrics";
export { BatchLogRecordProcessor } from "@opentelemetry/sdk-logs";
