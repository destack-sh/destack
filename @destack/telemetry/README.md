# @destack/telemetry

Instrument Destack packages with traces, metrics, logs, exceptions and analytics.

## Overview

A package takes its telemetry from its environment, and the host exports every signal as OTLP to the space's observability.

- **TelemetryScope**: one package's `log`, `span`, `metric`, `captureException`, `captureMessage`, `track` and `visit` on an environment's telemetry.
- **Meter**: a declared measure of what a space uses, such as requests or stored bytes, which finance prices; `defineMeter`.
- **OtlpExporter**: exports every signal as OTLP/JSON below an endpoint through a fetch.
- **Session**: one run of a program, errored by a handled exception and crashed by an escaped one.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

async (environment: Environment, text: string) => {
    const scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const { log, span, metric, captureException, track } = scope;
    log.info("page.saved", { length: text.length });
    await span("page.render", { blocks: 12 }, async () => undefined);
    const attributes = { space: ["personal", "shared"] };
    const saves = metric.counter("page.saves", { unit: "{save}", attributes });
    saves.add(1, { space: "shared" });
    captureException(new Error("export failed"), { tags: { feature: "export" } });
    track("page.shared", { audience: "space" });
};
```

## Examples

### Declare a meter

A package declares the meters of what its spaces use, which finance prices and observability records.

```ts
import { t } from "@destack/locale";
import { defineMeter } from "@destack/telemetry/declare";

export const requests = defineMeter({
    name: "api.requests",
    description: t`Requests to the API.`,
    aggregation: "sum",
    unit: "request",
    attributes: {},
});
```

### Export to an endpoint

`OtlpExporter.http` exports every signal below an endpoint, naming only the service and its release.

```ts
import { OtlpExporter } from "@destack/telemetry/otlp";

const exporter = OtlpExporter.http(
    { endpoint: "https://telemetry.example.com/v1", fetch: (request) => fetch(request) },
    (error) => console.error(error),
);
```

## Comparison

### OpenTelemetry

Logs, spans and metrics follow OpenTelemetry's API and OTLP; three things differ.

- `captureException`, `captureMessage` and sessions follow Sentry's, with `level`, `tags` and `fingerprint`.
- `visit` and `track` follow Segment's `page` and `track`.
- Every signal is stamped on the environment's clock, so a simulation's telemetry replays with its seed.
