Instrument Destack packages with logs, spans and metrics over [OpenTelemetry](https://opentelemetry.io/docs/languages/js/).

## Logs

A package emits structured records under literal event names, and keeps sensitive values under `sensitive.` keys.

```ts
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

const { log } = telemetry.scope(import.meta.destack.package);

log.info("note.saved", { length: text.length });
log.info("user.invited", { "sensitive.email": invite.email, role: "editor" });
log.error("note.sync.failed", telemetry.exceptionAttributes(error));
```

## Spans

A package runs work in a child span of the active one, or adds attributes to the active span, such as the call the platform traces.

```ts
const { span } = telemetry.scope(import.meta.destack.package);

span.current()?.setAttributes({ blocks: note.blocks.length });
const html = await span("note.render", { blocks: 12 }, () => render(note));
```

## Metrics

A package declares the values each metric attribute takes, so the compiler bounds every metric's series.

```ts
const { metric } = telemetry.scope(import.meta.destack.package);
const saves = metric.counter("note.saves", {
    unit: "{save}",
    attributes: { notebook: ["personal", "shared"] },
});

saves.add(1, { notebook: "shared" });
```

## Sampling

A host keeps a ratio of traces and every trace that failed or ran slow.

| Class              | Role                                                                                                                                                                                           |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `TraceIdGenerator` | Starts each trace identifier with its creation millisecond, so lookups by identifier know the time.                                                                                            |
| `RatioSampler`     | Samples a ratio of root traces by their random bits; children follow their parent, the rest record.                                                                                            |
| `TailSampler`      | Buffers a local trace's recorded spans and informational records until its root ends, keeping them on a failure or slowness, and counts traces it drops undecided in `telemetry.tail.evicted`. |

## Export

A host exports every signal as OTLP/JSON to its scope's monitor, reporting refused deliveries and the SDK's own failures.

```ts
import { startTelemetry } from "@destack/telemetry/host";
import { OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.http(start.monitor, () => `Bearer ${credential}`, report);
const running = await startTelemetry(
    exporter.options(runner.package, {
        attributes: { "service.instance.id": instance },
        manifest: start.manifest,
        ratio: 0.1,
    }),
);
```

A page exports to its own origin below `/.destack/telemetry` as its session, with `keepalive` so a delivery outlives the page, and the host serving the origin keeps the export as its installation's.

```ts
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.origin(reportToDevtools);
await startTelemetry(exporter.options(view.package, { manifest: bootstrap.manifest }));
```

## Release

Every signal's resource names the release that emitted it in `service.version` and its build manifest in `destack.build.manifest`, so the monitor groups failures by release and maps their stacks through the build's source maps.

## Capture

Each runtime records an uncaught failure as an `exception` log record at `ERROR` in the active trace, with `exception.type`, `exception.message` and `exception.stacktrace`, and exports it at once.

| Runtime    | Captures                                      | Then                                                                                                                                                 |
| ---------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/host`    | `uncaughtException` and `unhandledRejection`  | waits up to two seconds for the export, then raises the failure again, ending the process as it would have ended, unless another listener handles it |
| `/browser` | the window's `error` and `unhandledrejection` | leaves the browser's own reporting, and exports again on `pagehide` and on `visibilitychange` to hidden                                              |
| `/worker`  | a failure `withTelemetry`'s operation throws  | throws it on, exporting before the invocation ends                                                                                                   |

## Runtimes

Each runtime starts telemetry with its own context propagation.

| Export     | Starts                                                                                    | Context                      |
| ---------- | ----------------------------------------------------------------------------------------- | ---------------------------- |
| `/host`    | `startTelemetry(options)`                                                                 | async local storage          |
| `/browser` | `startTelemetry(options, window)`, with `DevtoolsExporter` writing to the developer tools | the call stack               |
| `/worker`  | `startTelemetry(options, manager)`, or `withTelemetry` per invocation                     | the worker's context manager |
