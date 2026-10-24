# @destack/telemetry

`TelemetryScope.of` is OpenTelemetry's logs, tracer and meter for one package on an environment's telemetry, `captureException`, `captureMessage` and sessions are Sentry's with its `level`, `tags`, `fingerprint` and errored or crashed sessions, `visit` and `track` are Segment's `page` and `track`, and `/otlp` exports and receives OTLP/JSON.

```ts
const { log, span, metric, track, visit, captureException } = TelemetryScope.of(
    import.meta.destack.package,
    environment.telemetry,
);
log.info("page.saved", { length: text.length }); // an OpenTelemetry LogRecord, stamped on the environment's clock
await span("page.render", { blocks: 12 }, () => render(page)); // tracer.startActiveSpan
// meter.createCounter
metric
    .counter("page.saves", { unit: "{save}", attributes: { space: ["personal", "shared"] } })
    .add(1, { space: "shared" });
captureException(error, { tags: { feature: "export" } }); // Sentry.captureException
track("page.shared", { audience: "space" }); // Segment's analytics.track
```

## Failures

`startTelemetry` records uncaught failures on Bun, in browsers and in Workers, on the environment's clock and randomness, and `exceptionAttributes` sets `error.type` to a failure's service code, `code` or name.

```ts
import { observeProcess, startTelemetry } from "@destack/telemetry/bun";

// every failed or slow trace kept beside the ratio
const telemetry = await startTelemetry(
    exporter.options(runner.package, { ratio: 0.1 }),
    environment,
);
observeProcess(telemetry, runner.package, environment); // process.cpu.time by mode
telemetry.startSession(); // a handled exception marks it errored, an escaped one crashed
```

```ts
import { startTelemetry } from "@destack/telemetry/browser";

// a page's uncaught errors and unhandled rejections, flushed as the page hides
const telemetry = await startTelemetry(
    exporter.options(view.package),
    BrowserEnvironment.system,
    window,
);
```

## Export

`OtlpExporter.http` exports every signal as OTLP/JSON below an endpoint through a target's fetch, naming only the service and its release, and the receiver at the instance's host stamps the scope, installation, instance and build it verified.

```ts
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";

// a workload's signals to its host, under the runner's secret
const exporter = OtlpExporter.http(
    {
        endpoint: new URL(OTLP_ORIGIN_PATH, start.egress).href,
        headers: () => ({ authorization: `Bearer ${start.secret}` }),
        fetch: environment.network.fetch,
    },
    report,
    new Symbolicator(build), // exceptions leave with their frames resolved through the build
);
// a page's signals to /.destack/telemetry on its origin, outliving the page
const browser = OtlpExporter.http(
    { endpoint: OTLP_ORIGIN_PATH, isKeepalive: true, fetch: environment.network.fetch },
    reportToDevtools,
);
// the desktop's signals to its machine's observability package, through the machine's loopback client
const desktop = OtlpExporter.http(
    { endpoint: intake, fetch: (request) => Endpoint.send(home, request) },
    report,
);
```
