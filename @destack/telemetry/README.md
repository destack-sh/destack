# @destack/telemetry

Instrument Destack packages with logs, spans and metrics over [OpenTelemetry](https://opentelemetry.io/docs/languages/js/).

## Logs

`log` emits a structured record under a literal event name, and attributes under `sensitive.` keys hold values the monitor masks.

```ts
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

const { log } = telemetry.scope(import.meta.destack.package);

log.info("note.saved", { length: text.length });
log.info("user.invited", { "sensitive.email": invite.email, role: "editor" });
log.error("note.sync.failed", telemetry.exceptionAttributes(error));
```

## Spans

`span` runs a function in a child span of the active span, and `span.current` returns the active span, such as the call the platform traces.

```ts
const { span } = telemetry.scope(import.meta.destack.package);

span.current()?.setAttributes({ blocks: note.blocks.length });
const html = await span("note.render", { blocks: 12 }, () => render(note));
```

## Metrics

`metric.counter` declares the values each attribute takes, so the compiler bounds the series of the metric.

```ts
const { metric } = telemetry.scope(import.meta.destack.package);
const saves = metric.counter("note.saves", {
    unit: "{save}",
    attributes: { notebook: ["personal", "shared"] },
});

saves.add(1, { notebook: "shared" });
```

## Sampling

The sampler keeps the `ratio` of traces the exporter options name, and every trace that failed or ran slow.

```text
a root whose random bits fall below the ratio   kept
a child span                                    kept as its parent decided
a local trace that failed or ran slow           buffered with its records until its root ends, then kept
a buffered trace evicted undecided              dropped, and counted in telemetry.tail.evicted
```

## Trace identifiers

The trace identifier generator writes the creation millisecond into the first 6 bytes, so a lookup by identifier knows the time.

```text
0199a3f2c1b0 7a4e9f2d6c8b1e3a5f70
└ Unix ms ─┘ └ 10 random bytes ─┘
```

## Export

`OtlpExporter.http` exports every signal as OTLP/JSON to the scope's monitor and passes refused deliveries and SDK failures to `report`.

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

## Browser export

`OtlpExporter.origin` exports a page's signals to `/.destack/telemetry` on the page's origin with `keepalive`, so a delivery outlives the page, and the host records them for the installation.

```ts
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.origin(reportToDevtools);
await startTelemetry(exporter.options(view.package, { manifest: bootstrap.manifest }));
```

## Release

The resource of every signal names the release and the build manifest that emitted it, which the monitor uses to group failures by release and map stacks through source maps.

```json
{ "service.version": "2026.10.0", "destack.build.manifest": "e23ace0a…" }
```

## Capture

Each runtime records an uncaught failure as an `exception` log record at `ERROR` in the active trace, with `exception.type`, `exception.message` and `exception.stacktrace`, and exports it at once.

```text
/host     uncaughtException and unhandledRejection
          waits up to two seconds for the export, then raises the failure again,
          ending the process as it would have ended, unless another listener handles it
/browser  the window's error and unhandledrejection
          leaves the browser's reporting, and exports again on pagehide and on visibilitychange to hidden
/worker   a failure withTelemetry's operation throws
          throws it on, exporting before the invocation ends
```

## Runtimes

Each runtime's `startTelemetry` propagates context through async local storage on hosts, through the call stack in browsers and through the given context manager in workers.

```ts
import { startTelemetry } from "@destack/telemetry/host";
import { startTelemetry as startBrowser } from "@destack/telemetry/browser";
import { startTelemetry as startWorker } from "@destack/telemetry/worker";

await startTelemetry(options);
await startBrowser(options, window); // DevtoolsExporter writes to the developer tools
await startWorker(options, manager); // or withTelemetry per invocation
```
