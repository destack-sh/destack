# @destack/telemetry

Instrument Destack packages with traces, metrics, logs, exceptions and analytics.

## Overview

`@destack/telemetry` is OpenTelemetry's API with one scope per package: traces, metrics and logs, with exceptions and analytics on the same signals.

You record what a package does:

- `TelemetryScope`: records one package's spans, metrics and logs under its scope.
- `captureException` and `captureMessage`: record a failure the package handled, which joins an issue.
- `track` and `visit`: record what a person did and opened.

You declare what a space uses:

- `defineMeter`: declares a `Meter`, a measure of usage such as `databaseUsage`.

A program exports the signals:

- `startTelemetry`: starts the program's OpenTelemetry SDK on Bun, in a browser or in a worker.
- `OtlpExporter`: exports OTLP/JSON through a fetch.
- `RatioSampler` and `TailSampler`: keep a ratio of traces, and the traces that fail or run slow.

Tests read them:

- `TestTelemetry`: reads the exceptions and logs a simulation recorded.

Scope a package and record a span with its duration and a log:

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Save a page, recording what happened on every signal. */
export async function savePage(environment: Environment, text: string) {
    const { log, span } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const attributes = { "example.page.kind": "note" };

    await span(
        "example.page.save",
        attributes,
        async () => log.info("example.page.saved", { "example.page.length": text.length }),
        { duration: { attributes } }, // example.page.save.duration
    );
}
```

## Reference

### TelemetryScope

`TelemetryScope.of(package, telemetry)` is OpenTelemetry's `getTracer`, `getMeter` and `getLogger` for one package.
It names the package's signals under its scope, such as `example` for `@example/pages`.

| Verb                               | What it does                                                              |
| ---------------------------------- | ------------------------------------------------------------------------- |
| `span(name, attributes, run)`      | runs work in a child span, and records its duration when asked            |
| `measure(name, attributes, run)`   | records the seconds work took without a span, with the failure it ends in |
| `metric.histogram`, `counter`, …   | declares an instrument whose attributes take listed values                |
| `log.info(name, attributes)`       | emits a log record under an event name, also `debug` and `warn`           |
| `captureException(error, context)` | records a handled failure as an exception in the active trace             |
| `track(name, attributes)`          | records an action a person took                                           |
| `visit(route, detail)`             | records a view a person opened at a route                                 |

### Meter

A meter folds a month's events into one usage per space and attribute value.

| Aggregation | Usage                            |
| ----------- | -------------------------------- |
| `sum`       | the events' values added up      |
| `count`     | the number of events             |
| `max`       | the largest value                |
| `last`      | the latest value                 |
| `average`   | the values' mean over the period |

## Examples

### Trace a stream

`span` follows work that answers a stream, each step in the span, and ends it once the stream ends.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Answer a list call with a stream of pages, and check access with its decision read at the end. */
export async function* listPages(environment: Environment, pages: AsyncIterable<string>) {
    const { span } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const attributes = { "rpc.method": "note/list" };
    let decided = { "destack.access.decision": "denied" };

    await span(
        "destack.access.check",
        attributes,
        async () => (decided = { "destack.access.decision": "allowed" }),
        {
            duration: { attributes: () => decided }, // read at the end
        },
    );
    yield* await span("note/list", attributes, () => pages, {
        kind: "server",
        duration: { attributes },
    }); // a reader closing early: CLIENT_CLOSED_REQUEST
}
```

### Capture an exception

`captureException` records a handled failure in the active trace.

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Record a failed export as a warning, grouped with every other failed export. */
export function reportExport(environment: Environment, error: unknown) {
    const { captureException } = TelemetryScope.of(
        import.meta.destack.package,
        environment.telemetry,
    );
    captureException(error, {
        level: "warn",
        tags: { feature: "export" },
        fingerprint: ["export"],
    });
}
```

### Track an action

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

### Declare a meter

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

### Start telemetry in a program

`startTelemetry` starts a program's OpenTelemetry SDK once, and `OtlpExporter.http` exports OTLP/JSON through a fetch.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { startTelemetry } from "@destack/telemetry/bun";

/** Start the program's telemetry, exporting to an OTLP endpoint until shut down. */
export async function startExporting(endpoint: string) {
    const exporter = OtlpExporter.http({ endpoint, fetch }, (error) => console.error(error));

    return await startTelemetry(
        exporter.options(import.meta.destack.package),
        BunEnvironment.system,
    );
}
```

### Assert a captured exception

`TestTelemetry` reads a simulation's recorded exceptions.

```ts
import { TelemetryScope } from "@destack/telemetry";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("capture a failed export", async () => {
    await using simulation = testSimulation();
    using telemetry = TestTelemetry.of(simulation.telemetry);
    const { captureException } = TelemetryScope.of(
        import.meta.destack.package,
        simulation.environment("host").telemetry,
    );
    captureException(new Error("export failed"));

    expect(telemetry.take().length).toBe(1);
});
```

### Read a test world's telemetry

`SimulatedTelemetry.counts()` gives each histogram's count and seconds, in all and by target.

```ts
import { TelemetryScope } from "@destack/telemetry";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("save a page once", async () => {
    await using simulation = testSimulation();
    const { span } = TelemetryScope.of(
        import.meta.destack.package,
        simulation.environment("host").telemetry,
    );
    const before = await simulation.telemetry.counts();
    await span("example.page.save", {}, async () => "saved");
    const after = await simulation.telemetry.counts();

    expect(
        (after["example.page.save.duration.count"] ?? 0) -
            (before["example.page.save.duration.count"] ?? 0),
    ).toBe(1);
});
```

### Print a test world's telemetry

`DESTACK_TELEMETRY=tree` prints each test world's span tree with world and host time, and its histograms with p50 and p95.

```sh
DESTACK_TELEMETRY=tree bun run test tests/apply.test.ts
```

### Benchmark an operation

`destack test --tier bench` reports host seconds per operation with p50 and p95, and each histogram's count per operation, for benchstat.

```sh
destack test --tier bench
benchstat node_modules/.cache/destack/bench/main/database-sqlite.txt node_modules/.cache/destack/bench/current/database-sqlite.txt
```

## Conventions

### Signal names

Signals follow OpenTelemetry's semantic conventions, and a package's own names start with its scope.
Spans and logs carry identifiers and metrics do not, so each metric has a bounded set of series.

| Signal            | Name                                        | Such as                            |
| ----------------- | ------------------------------------------- | ---------------------------------- |
| span              | `<scope>.<noun>.<verb>`                     | `destack.controller.pass`          |
| a span's duration | the span's name and `.duration`, in seconds | `destack.controller.pass.duration` |
| wait time         | `<scope>.<noun>.wait_time`, zero included   | `destack.controller.wait_time`     |
| log event         | `<scope>.<noun>.<verb in the past tense>`   | `destack.postgres.warned`          |
| metric attribute  | `<scope>.<noun>`, which lists its values    | `destack.controller` of `page`     |
| span attribute    | `<scope>.<noun>`, any value                 | `destack.key`                      |
| failure           | `error.type`, the failure's code            | `CONFLICT`                         |
| failure reason    | `destack.error.reason`, its reason          | `REVISION_STALE`                   |

### Levels

Each log record has one of four levels.

| Level   | Meaning                                               | Such as                                                      |
| ------- | ----------------------------------------------------- | ------------------------------------------------------------ |
| `ERROR` | a failure someone acts on, captured once by its owner | an unexpected throw, a key that keeps failing, a lost export |
| `WARN`  | a degraded state that recovers on its own             | `destack.replica.silent`, `destack.replica.refused`          |
| `INFO`  | a lifecycle phase or change, once each                | `destack.space.serve`, `destack.database.migrated`           |
| `DEBUG` | detail per request or per iteration, off by default   | a page sent, a statement run                                 |

Spans take the same levels: a lifecycle phase is an info span, and a request is a debug span, the default.
The package's `TelemetryScope` records both:

```ts
import type { Environment } from "@destack/environment";
import { TelemetryScope } from "@destack/telemetry";

/** Start a space's servers in an info span, which a process prints with its duration once it ends. */
export async function startSpace(
    environment: Environment,
    space: string,
    serve: () => Promise<void>,
): Promise<void> {
    const { span } = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    await span("destack.space.serve", { "destack.space.id": space }, serve, { level: "info" });
}
```

A process prints its log records and spans from the level in `DESTACK_LOG_LEVEL` on, `info` when absent, and exports every level.

Each case uses one kind of record:

| Case                                       | Record                                              |
| ------------------------------------------ | --------------------------------------------------- |
| a process goes through a phase             | an info span through the package's `TelemetryScope` |
| a server answers a request                 | a span, with the request's outcome as attributes    |
| a failure reaches its owner                | one exception, captured by the owner                |
| a record refers to a user, space or object | the identifier, without secrets or personal content |

Program output, such as a command's result or a checker's report, writes to standard output.
