# @destack/telemetry

Instrument Destack packages with traces, metrics, logs, exceptions and analytics.

## Instrument a package

`TelemetryScope.of` is OpenTelemetry's `getTracer` and `getMeter` for one package, named under its scope, `acme` for `@acme/pages`.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Save a page, recording what happened on every signal. */
export async function savePage(environment: Environment, text: string) {
    const { log, span, measure, metric } = TelemetryScope.of(
        import.meta.destack.package,
        environment.telemetry,
    );
    const saves = metric.histogram("acme.page.save.duration", {
        unit: "s",
        attributes: { "acme.page.kind": ["note", "doc"], "error.type": ["NOT_FOUND"] },
    });

    await measure(saves, { "acme.page.kind": "note" }, () =>
        span("acme.page.save", { "acme.page.kind": "note" }, async () => {
            log.info("acme.page.saved", { "acme.page.length": text.length });
        }),
    );
}
```

## Conventions

Signals follow OpenTelemetry's semantic conventions, with these Destack additions:

```ts
const { span, measure, metric, log } = TelemetryScope.of(import.meta.destack.package, telemetry); // per package, never a global
const passes = metric.histogram("destack.controller.pass.duration", { unit: "s", attributes }); // <scope>.<noun>.<verb>
await measure(passes, { "destack.controller.name": "space" }, () => reconcile(rows)); // error.type: CONFLICT
await span("destack.controller.pass", { "destack.controller.kind": "reconcile" }, run); // target in attributes
metric.histogram("destack.controller.wait.duration", { unit: "s", attributes }); // wait apart
metric.histogram("destack.controller.catch_up.duration", { unit: "s", attributes }); // catch-up, not length
log.warn("destack.postgres.warned", { "db.response.status_code": "01000" }); // past tense
```

## Capture an exception

`captureException` records a handled failure in the active trace, grouped into issues as Sentry groups them.

```ts
captureException(error, { level: "warning", tags: { feature: "export" }, fingerprint: ["export"] });
```

## Track an action

`track` and `visit` record what a person did and opened, as Segment's `track` and `page`.

```ts
track("page.shared", { audience: "space" });
visit("/notes/:id", { path: "/notes/1", title: "Groceries" });
```

## Declare a meter

`defineMeter` declares a measure of what a space uses, priced apart by attribute.

```ts
import { t } from "@destack/locale";
import { defineMeter } from "@destack/telemetry/declare";

export const requests = defineMeter({
    name: "api.requests",
    description: t`Requests to the API.`,
    aggregation: "sum",
    unit: "request",
    attributes: { direction: ["read", "write"] },
});
```

## Start telemetry in a program

`startTelemetry` starts a program's OpenTelemetry SDK once, and `OtlpExporter.http` exports OTLP/JSON through a fetch.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { startTelemetry } from "@destack/telemetry/bun";

/** Start the program's telemetry, exporting to an OTLP endpoint until shut down. */
export async function startExporting(endpoint: string) {
    const exporter = OtlpExporter.http({ endpoint, fetch }, (error) => console.error(error));

    return await startTelemetry(exporter.options(import.meta.destack.package), BunEnvironment.system);
}
```

## Assert a captured exception

`TestTelemetry` reads a simulation's recorded exceptions.

```ts
import { TelemetryScope } from "@destack/telemetry";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("capture a failed export", async () => {
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

## Read a test world's telemetry

`SimulatedTelemetry.counts()` gives each histogram's count and seconds, in all and by target.

```ts
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("save a page in one transaction of two statements", async () => {
    await using simulation = testSimulation();
    const before = await simulation.telemetry.counts();
    await savePage(simulation.environment("host"), "hello");
    const after = await simulation.telemetry.counts();

    const count = (key: string) => (after[key] ?? 0) - (before[key] ?? 0);
    expect([count("db.client.operation.duration.count"), count("destack.transaction.duration.count")]).toEqual([2, 1]);
});
```

## Print a test world's telemetry

`DESTACK_TELEMETRY=tree` prints each test world's span tree with world and host time, and its histograms with p50 and p95.

```sh
DESTACK_TELEMETRY=tree bun run test tests/apply.test.ts
```

## Benchmark an operation

`just bench` reports host seconds per operation with p50 and p95, and each histogram's count per operation, for benchstat.

```sh
just bench
benchstat node_modules/.cache/destack/bench/main/database-sqlite.txt node_modules/.cache/destack/bench/current/database-sqlite.txt
```
