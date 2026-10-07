# @destack/telemetry

Instrument Destack packages with [OpenTelemetry](https://opentelemetry.io/docs/languages/js/) logs, spans, metrics and exceptions.

## Logs

`log` emits a structured record under a literal event name.

```ts
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

const { log } = telemetry.scope(import.meta.destack.package);

log.info("note.saved", { length: text.length });
log.info("user.invited", { "sensitive.email": invite.email, role: "editor" });
```

## Spans

`span` runs a function in a child span of the active span.

```ts
const { span } = telemetry.scope(import.meta.destack.package);

span.current()?.setAttributes({ blocks: note.blocks.length });
const html = await span("note.render", { blocks: 12 }, () => render(note));
```

## Metrics

`metric.counter` declares a counter and the values of each attribute.

```ts
const { metric } = telemetry.scope(import.meta.destack.package);
const saves = metric.counter("note.saves", {
    unit: "{save}",
    attributes: { notebook: ["personal", "shared"] },
});

saves.add(1, { notebook: "shared" });
```

## Handled failures

`captureException` and `captureMessage` record a handled failure as an `exception`.

```ts
const { captureException, captureMessage } = telemetry.scope(import.meta.destack.package);

captureException(error, { tags: { feature: "export" }, attributes: { "export.id": id } });
captureMessage("quota nearly reached", { level: "warning", fingerprint: ["quota"] });
captureException(error, { isEscaped: true, tags: { controller: "deliveries" } });
```

## Uncaught failures

`startTelemetry` records uncaught failures and exports them at once.

```ts
import { startTelemetry } from "@destack/telemetry/bun"; // uncaughtException, unhandledRejection
import { startTelemetry as startBrowser } from "@destack/telemetry/browser"; // error, unhandledrejection
import { withTelemetry } from "@destack/telemetry/worker"; // a failure the operation throws

await startTelemetry(options);
await startBrowser(options, window);
await withTelemetry(options, () => handle(request), context);
```

## Error types

`exceptionAttributes` sets `error.type` to a failure's service code, `code` or name.

```ts
import { ServiceError } from "@destack/service";

const missing = Object.assign(new Error("gone"), { code: "ENOENT" });

telemetry.exceptionAttributes(new ServiceError("NOT_FOUND"), false)["error.type"]; // "NOT_FOUND"
telemetry.exceptionAttributes(missing, false)["error.type"]; // "ENOENT"
```

## Sessions

`startSession` and `endSession` mark one run of the application.

```ts
telemetry.startSession();
await saveNote(note); // a handled exception marks the session errored, an escaped one crashed
telemetry.endSession(); // session.end { session.status }, session.count { session.status }
```

## Sampling

`exporter.options` samples a `ratio` of traces and keeps every failed or slow trace.

```ts
await startTelemetry(exporter.options(runner.package, { ratio: 0.1 }));
```

## Trace identifiers

`TraceIdGenerator` writes the creation time into the first 6 bytes of a trace identifier.

```ts
import { TraceIdGenerator } from "@destack/telemetry/trace";

const id = new TraceIdGenerator().generateTraceId(); // "0199a3f2c1b0" + 10 random bytes
const createdAt = Number.parseInt(id.slice(0, 12), 16);
```

## Export

`OtlpExporter.http` exports every signal as OTLP/JSON naming only the service and its release, and the receiver at the instance's host (`/.destack/telemetry`) stamps the scope, installation, instance and build it verified.

```ts
import { startTelemetry } from "@destack/telemetry/bun";
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";

const host = new URL(OTLP_ORIGIN_PATH, start.egress).href;
const exporter = OtlpExporter.http(host, () => `Bearer ${start.secret}`, report);
await startTelemetry(exporter.options(runner.package, { ratio: start.sampling }));
```

## Browser export

`OtlpExporter.origin` exports a page's signals to `/.destack/telemetry` on its origin.

```ts
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.origin(reportToDevtools);
await startTelemetry(exporter.options(view.package));
```

## Receive

`OtlpReceiver` takes an export under the `OtlpEmitter` its caller verified, and `OtlpSignal.at` reads the signal of an export's path.

```ts
import { OtlpSignal } from "@destack/telemetry/otlp";

const signal = OtlpSignal.at("/.destack/telemetry/v1/logs"); // "logs"
await observability.receive({ scope, installation, instance, build }, signal, body); // an OtlpReceiver
```
