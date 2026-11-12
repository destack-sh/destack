# @destack/telemetry

Instrument Destack packages with traces, metrics, logs, exceptions and analytics.

## Overview

`@destack/telemetry` is OpenTelemetry with Sentry's exceptions and Segment's analytics: package-scoped instruments, meters, sessions and OTLP export.

Packages instrument themselves:

- `TelemetryScope`: one package's logs, spans, metrics, exceptions and analytics.
- `Meter`: a measure of what a space uses, declared with `defineMeter`.

Hosts start and export telemetry:

- `Telemetry`: a program's OpenTelemetry SDK.
- `Session`: one run of a program.
- `OtlpExporter`: exports signals as OTLP/JSON.

Tests:

- `TestTelemetry`: what a simulation's telemetry recorded.

Log, trace, count, capture and track from a package's code:

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Save a page, recording what happened on every signal. */
export async function savePage(environment: Environment, text: string) {
    // attribute the signals to this package
    const { log, span, metric, captureException, track } = TelemetryScope.of(
        import.meta.destack.package,
        environment.telemetry,
    );
    const saves = metric.counter("page.saves", {
        unit: "{save}",
        attributes: { space: ["personal", "shared"] },
    });

    // render in a span, count the save, and log its length
    await span("page.render", { blocks: 12 }, async () => {
        saves.add(1, { space: "shared" });
        log.info("page.saved", { length: text.length });
    });

    // capture the failure, and track the action
    captureException(new Error("export failed"), { tags: { feature: "export" } });
    track("page.shared", { audience: "space" });
}
```

## Reference

### TelemetryScope

`TelemetryScope.of(package, telemetry)` attributes every signal to a package, on the environment's clock.

| Member                                            | What it does                                                                         |
| ------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `log.debug`, `info`, `warn`, `error`              | emits a log record named by an event name, with attributes                           |
| `span(name, attributes, work)`                    | runs work in a child span of the active one, recording its failure                   |
| `span.current()`                                  | reads the active span, such as the call the platform traces                          |
| `metric.counter`, `histogram`, `gauge`            | declares an instrument whose attributes take bounded values, checked by the compiler |
| `captureException(error, context)`                | records a handled failure as an exception in the active trace                        |
| `captureMessage(message, context)`                | records a message as an exception-level event                                        |
| `track(name, attributes)`                         | records an action a person took                                                      |
| `visit(route, { path, title, referrer, locale })` | records a view a person opened                                                       |
| `tracer`, `meter`, `logger`                       | the package's OpenTelemetry tracer, meter and logger                                 |

A capture takes a context:

| Field         | What it is                                                                 |
| ------------- | -------------------------------------------------------------------------- |
| `level`       | `"fatal"`, `"error"`, the default, `"warning"` or `"info"`                 |
| `tags`        | bounded labels issues are filtered and grouped by, such as the feature     |
| `fingerprint` | the fingerprint grouping the failure into an issue, instead of its frames' |
| `attributes`  | details issues never group by, such as the failing key                     |
| `isEscaped`   | whether the failure escaped its code, such as a request handler            |

### Meter

`defineMeter({ name, description, aggregation, unit, attributes })` declares a measure of what a space uses.

| Field         | What it is                                                                                    |
| ------------- | --------------------------------------------------------------------------------------------- |
| `name`        | the name within the package, such as `api.requests`                                           |
| `description` | what it counts, as a message                                                                  |
| `aggregation` | how a period's events fold: `"sum"`, `"count"`, `"max"`, `"last"` or `"average"`              |
| `unit`        | the unit of an event's value, such as `request` or `byte`                                     |
| `attributes`  | each attribute with the values it takes, priced apart, such as `direction: ["read", "write"]` |

### Telemetry

`startTelemetry(options, environment)` starts a program's SDK, from `@destack/telemetry/bun` or `@destack/telemetry/browser`.
It captures uncaught failures.

| Option       | What it is                                                         |
| ------------ | ------------------------------------------------------------------ |
| `name`       | the running service's or frontend's name                           |
| `version`    | the deployed release every signal names                            |
| `propagator` | the HTTP propagator, W3C trace context and baggage by default      |
| `metrics`    | the metric readers, aggregation and limits                         |
| `signals`    | the trace and log processors, sampling and limits                  |
| `report`     | reports the SDK's own warnings and failures, such as dropped spans |

| Verb                             | What it does                                                         |
| -------------------------------- | -------------------------------------------------------------------- |
| `capture(error)`                 | records an escaped failure, crashing the running session             |
| `startSession()`, `endSession()` | starts or ends a session, which every signal carries as `session.id` |
| `flush(signals)`                 | exports the buffered signals                                         |
| `shutdown()`                     | exports what is left and stops                                       |

### OtlpExporter

`OtlpExporter.http({ endpoint, fetch }, report)` exports to `<endpoint>/v1/traces`, `/v1/logs` and `/v1/metrics`.
Metrics are deltas, and histograms base-2 exponential.

## Examples

### Start telemetry in a program

A program starts its SDK once, exporting every signal as OTLP to an endpoint.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { startTelemetry } from "@destack/telemetry/bun";

/** Start the program's telemetry, exporting to an OTLP endpoint until shut down. */
export async function startExporting(endpoint: string) {
    // export over HTTP, reporting the exporter's own failures
    const environment = BunEnvironment.system;
    const exporter = OtlpExporter.http({ endpoint, fetch }, (error) => console.error(error));

    // start the SDK with the exporter, named after this package
    return await startTelemetry(exporter.options(import.meta.destack.package), environment);
}
```

### Declare a meter

Meters can be priced apart by attribute.

```ts
import { t } from "@destack/locale";
import { defineMeter } from "@destack/telemetry/declare";

// count API requests, reads and writes priced apart
export const requests = defineMeter({
    name: "api.requests",
    description: t`Requests to the API.`,
    aggregation: "sum",
    unit: "request",
    attributes: { direction: ["read", "write"] },
});
```

### Assert a captured exception

`TestTelemetry` reads a simulation's recorded exceptions.

```ts
import { TelemetryScope } from "@destack/telemetry";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("capture a failed export", async () => {
    // capture an exception on a simulated node
    await using simulation = testSimulation();
    using telemetry = new TestTelemetry(simulation.telemetry);
    const { captureException } = TelemetryScope.of(
        import.meta.destack.package,
        simulation.environment("host").telemetry,
    );
    captureException(new Error("export failed"));

    expect(telemetry.take().length).toBe(1);
});
```

## Comparison

### OpenTelemetry

Logs, spans, metrics and their export basically follow OpenTelemetry's API and OTLP.
Instruments come from the environment instead of a global.

| OpenTelemetry                         | `@destack/telemetry`                   | Difference                                               |
| ------------------------------------- | -------------------------------------- | -------------------------------------------------------- |
| `trace.getTracer`, `metrics.getMeter` | `TelemetryScope.of`                    | Per package, from the environment.                       |
| `tracer.startActiveSpan(name, fn)`    | `span(name, attributes, work)`         | Records the work's failure on the span.                  |
| `meter.createCounter(name, options)`  | `metric.counter(name, { attributes })` | Attributes take bounded values, checked by the compiler. |
| `logger.emit(record)`                 | `log.info(name, attributes)`           | Named by an event name.                                  |
| the OTLP/HTTP exporters               | `OtlpExporter.http`                    | OTLP/JSON through a fetch.                               |

### Sentry and Segment

Exceptions and sessions follow Sentry's, and analytics Segment's.

| Prior art                                                      | `@destack/telemetry`               | Difference                                             |
| -------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------ |
| Sentry `captureException(error, { level, tags, fingerprint })` | `captureException(error, context)` | Recorded as an exception event in the active trace.    |
| Sentry `captureMessage`                                        | `captureMessage`                   | The same.                                              |
| Sentry release health sessions                                 | `Session`                          | `ok`, `errored` or `crashed`, carried as `session.id`. |
| Segment `track(event, properties)`                             | `track(name, attributes)`          | A log record of the package.                           |
| Segment `page(name, properties)`                               | `visit(route, detail)`             | Keyed by the view's route pattern.                     |
