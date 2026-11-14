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
import { type Environment, SpanPath } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Pass a controller over a key that waited since it came due. */
export async function pass(environment: Environment, key: string, waited: number, run: () => Promise<void>) {
    const { telemetry } = environment;
    const { span, measure, metric, log } = TelemetryScope.of(import.meta.destack.package, telemetry); // per package, never a global
    const bounded = { "destack.controller": "page" } as const;
    const waits = metric.histogram("destack.controller.wait_time", { unit: "s", attributes: { "destack.controller": ["page"] } });

    waits.record(waited, bounded); // every take, zero included, as OpenTelemetry's pools record wait_time
    await span("destack.controller.pass", { ...bounded, "destack.key": key }, run, { duration: { attributes: bounded } }); // ids stay off metrics
    await measure("db.client.operation.duration", bounded, run); // no span; error.type: CONFLICT
    log.warn("destack.postgres.warned", { "db.response.status_code": "01000" }); // past tense
    SpanPath.add(telemetry.context.active(), "db.client.operation.count", 1); // every running span's totals
}
```

## Trace a stream

`span` follows work that answers a stream, each step in the span, and ends it once the stream ends.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Answer a list call with a stream of pages, and check access with its decision read at the end. */
export async function* listPages(environment: Environment, pages: AsyncIterable<string>) {
    const { span } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const attributes = { "rpc.method": "note/list" };
    let decided = { "destack.access.decision": "denied" };

    await span("destack.access.check", attributes, async () => (decided = { "destack.access.decision": "allowed" }), {
        duration: { attributes: () => decided }, // read at the end
    });
    yield* await span("note/list", attributes, () => pages, { kind: "server", duration: { attributes } }); // a reader closing early: CLIENT_CLOSED_REQUEST
}
```

## Capture an exception

`captureException` records a handled failure in the active trace.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Record a failed export as a warning, grouped with every other failed export. */
export function reportExport(environment: Environment, error: unknown) {
    const { captureException } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    captureException(error, { level: "warning", tags: { feature: "export" }, fingerprint: ["export"] });
}
```

## Track an action

`track` and `visit` record what a person did and opened, as Segment's `track` and `page`.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Record a person opening a note and sharing it with the space. */
export function shareNote(environment: Environment) {
    const { track, visit } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    visit("/notes/:id", { path: "/notes/1", title: "Groceries" });
    track("page.shared", { audience: "space" });
}
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
import { TelemetryScope } from "@destack/telemetry";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("save a page once", async () => {
    await using simulation = testSimulation();
    const { span } = TelemetryScope.of(import.meta.destack.package, simulation.environment("host").telemetry);
    const before = await simulation.telemetry.counts();
    await span("acme.page.save", {}, async () => "saved");
    const after = await simulation.telemetry.counts();

    expect((after["acme.page.save.duration.count"] ?? 0) - (before["acme.page.save.duration.count"] ?? 0)).toBe(1);
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
