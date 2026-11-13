# @destack/telemetry

Instrument Destack packages with traces, metrics, logs, exceptions and analytics.

## Instrument a package

`TelemetryScope.of` is OpenTelemetry's `getTracer` and `getMeter` for one package, named under its scope, `acme` for `@acme/pages`.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Save a page, recording what happened on every signal. */
export async function savePage(environment: Environment, text: string) {
    const { log, span } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const attributes = { "acme.page.kind": "note" };

    await span(
        "acme.page.save",
        attributes,
        async () => log.info("acme.page.saved", { "acme.page.length": text.length }),
        { duration: { attributes } }, // acme.page.save.duration
    );
}
```

## Conventions

Signals follow OpenTelemetry's semantic conventions, with these Destack additions:

```ts
const { span, measure, log } = TelemetryScope.of(import.meta.destack.package, telemetry); // per package, never a global
await span("destack.controller.pass", { ...bounded, "destack.key": key }, run, { duration: { attributes: bounded } }); // ids stay off metrics
await measure("db.client.operation.duration", attributes, run); // no span; error.type: CONFLICT
waits.record(seconds, bounded); // destack.controller.wait.duration: a computed wait, apart from the work
log.warn("destack.postgres.warned", { "db.response.status_code": "01000" }); // past tense
SpanPath.add(context.active(), "db.client.operation.count", 1); // every running span's totals
```

## Trace a stream

`span` follows work that answers a stream, each step in the span, and ends it once the stream ends.

```ts
const pages = await span("note/list", attributes, () => list(input), { kind: "server", duration: { attributes } });
for await (const page of pages) send(page); // a reader closing early: CLIENT_CLOSED_REQUEST
await span("destack.access.check", attributes, check, { duration: { attributes: () => decided } }); // read at the end
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
