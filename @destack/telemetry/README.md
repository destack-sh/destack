# @destack/telemetry

`telemetry.scope` is OpenTelemetry's logs, tracer and meter for one package, `captureException`, `captureMessage` and sessions are Sentry's with its `level`, `tags`, `fingerprint` and errored or crashed sessions, `visit` and `track` are Segment's `page` and `track`, and `/otlp` exports and receives OTLP/JSON.

```ts
const { log, span, metric, track, visit, captureException } = telemetry.scope(import.meta.destack.package);
log.info("page.saved", { length: text.length }); // an OpenTelemetry LogRecord
await span("page.render", { blocks: 12 }, () => render(page)); // tracer.startActiveSpan
metric.counter("page.saves", { unit: "{save}", attributes: { space: ["personal", "shared"] } }).add(1, { space: "shared" }); // meter.createCounter
captureException(error, { tags: { feature: "export" } }); // Sentry.captureException
track("page.shared", { audience: "space" }); // Segment's analytics.track
```

## Failures

`startTelemetry` records uncaught failures on Bun, in browsers and in Workers, and `exceptionAttributes` sets `error.type` to a failure's service code, `code` or name.

```ts
import { startTelemetry } from "@destack/telemetry/bun";

await startTelemetry(exporter.options(runner.package, { ratio: 0.1 })); // every failed or slow trace kept beside the ratio
captureMessage("quota nearly reached", { level: "warning", fingerprint: ["quota"] });
telemetry.startSession(); // a handled exception marks it errored, an escaped one crashed
```

## Export

`OtlpExporter.http` exports every signal as OTLP/JSON naming only the service and its release, and the receiver at the instance's host stamps the scope, installation, instance and build it verified.

```ts
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.http(new URL(OTLP_ORIGIN_PATH, start.egress).href, () => `Bearer ${start.secret}`, report);
const browser = OtlpExporter.origin(reportToDevtools); // a page's signals to /.destack/telemetry on its origin
```

## Trace identifiers

`TraceIdGenerator` writes the creation time into the first 6 bytes of a trace identifier, as UUIDv7 does.

```ts
import { TraceIdGenerator } from "@destack/telemetry/trace";

const id = new TraceIdGenerator().generateTraceId(); // "0199a3f2c1b0" + 10 random bytes
```
