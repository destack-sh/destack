# @destack/telemetry

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

| Noun | Role |
|---|---|
| `TraceIdGenerator` | Starts each trace identifier with its creation millisecond, so lookups by identifier know the time. |
| `RatioSampler` | Samples a ratio of root traces by their random bits; children follow their parent, the rest record. |
| `TailSampler` | Holds a local trace's recorded spans and informational records until its root ends, keeping them on a failure or slowness, and counts traces it drops undecided in `telemetry.tail.evicted`. |

## Export

A host exports every signal as OTLP/JSON to its scope's monitor, reporting refused deliveries and the SDK's own failures.

```ts
import { startTelemetry } from "@destack/telemetry/host";
import { OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.http(start.monitor, () => `Bearer ${credential}`, report);
const running = await startTelemetry(
    exporter.options(runner.package, {
        attributes: { "service.instance.id": instance },
        ratio: 0.1,
    }),
);
```

## Runtimes

Each runtime starts telemetry with its own context propagation.

| Export | Starts | Context |
|---|---|---|
| `/host` | `startTelemetry(options)` | async local storage |
| `/browser` | `startTelemetry(options)`, with `DevtoolsExporter` writing to the developer tools | the call stack |
| `/worker` | `startTelemetry(options, manager)`, or `withTelemetry` per invocation | the worker's context manager |
