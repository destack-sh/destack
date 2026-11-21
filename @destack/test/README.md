# @destack/test

Write and run Destack tests: unit tests on Vitest, simulated worlds, and scenarios played through drivers.

## Overview

`@destack/test` is Vitest with seeded worlds: tests, simulations, and scenarios played through drivers.

Tests use Vitest's API and a few checks:

- `test`, `expect`, `describe`, `vi`, …: Vitest's, imported from here.
- `refusal` and `single`: read a refused call, and the only item of a list.

Tests run in worlds:

- `testSimulation`: a simulation with the same seed in every run, stopped when its test ends.
- `testHost` and `testTarget`: the real host, and the target a test project runs against, such as its database.

Declared scenarios run through drivers:

- `Driver` and `DriverType`: take a scenario's steps and read its observations, one driver per interaction.
- `Runner`: plays scenarios as tests, and measures their operations in the bench tier.
- `TestPlan`: the targets of a package's tests, derived from its files and `destack.json`, which `destack test` runs.

Write a test whose effects run in a simulation:

```ts
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("sleep a minute of simulated time at once", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("host");

    // sleep a minute, which takes no real time
    const before = environment.clock.now();
    await simulation.sleep(60_000);

    expect(environment.clock.now() - before).toBe(60_000);
});
```

## Reference

### Checks and worlds

These check outcomes and open worlds.

| Function                         | What it does                                                     |
| -------------------------------- | ---------------------------------------------------------------- |
| `refusal(call)`                  | answers `[code, reason, message]` of a refused call, or `"done"` |
| `single(items)`                  | reads the only item of a list, refusing none or several          |
| `testSimulation(start, options)` | opens a seeded simulation, stopped when its test or file ends    |
| `testHost()`                     | reads the real host, for tests that need the operating system    |
| `testTarget(kind)`               | reads the target of a kind the test's project runs against       |

### Runner

`Runner` plays scenarios through their interaction's driver, checking each step's telemetry and observations.

| Verb                                               | What it does                                                            |
| -------------------------------------------------- | ----------------------------------------------------------------------- |
| `Runner.playModule(exports, types)`                | plays each scenario a module exports as a test                          |
| `Runner.play(scenario, types)`                     | plays a scenario once, replaying its seed under `DESTACK_REPLAY=1`      |
| `Runner.playthrough(scenario, type)`               | plays a scenario through a new driver, measuring its operations         |
| `Runner.checkDeterminism(scenario, first, second)` | names the first step after which two plays differ                       |
| `Runner.save(…)`, `Runner.format(…)`               | writes the bench tier's results in Go's benchmark format, for benchstat |

### Driver

A `DriverType` has an `interaction`, and `start(given)` starts a driver on a scenario's examples.

| Member                 | What it is                                                              |
| ---------------------- | ----------------------------------------------------------------------- |
| `act(step)`            | takes one step                                                          |
| `observe(observation)` | reads one observation                                                   |
| `count()`              | reads the world's counters since the driver started, by name            |
| `fingerprint()`        | reads a fingerprint of what the world did, equal across plays of a seed |
| `clock`                | the clock of the driver's world, which times each operation             |

### Plan

`TestPlan.of(directory, files)` derives a package's `TestTarget`s from its file names, and `destack test` runs them with no configuration file.

| File                     | Tier       | Runs in                                      |
| ------------------------ | ---------- | -------------------------------------------- |
| `*.test.ts`              | `unit`     | Bun                                          |
| `*.test.tsx`             | `unit`     | a DOM, through `@destack/view/test`          |
| `*.server.test.ts(x)`    | `unit`     | a server render                              |
| `*.hydration.test.tsx`   | `unit`     | a hydration                                  |
| `*.node.test.ts`         | `unit`     | Node, for tools that refuse Bun              |
| `*.test-d.ts`            | `unit`     | the type checker                             |
| `*.browser.scenario.tsx` | `scenario` | Chromium, through `@destack/view/test`       |
| `*.scenario.test.ts(x)`  | `scenario` | Bun, or a DOM for `.tsx`                     |
| `*.scenario.ts(x)`       | `scenario` | a scenario module, measured again in `bench` |

Every target runs once per database in `destack.json`'s `test.databases`, SQLite alone by default.
A package's global setup is `src/test/setup.ts`, its scenario drivers are the drivers `src/test/index.ts` exports, and a package declaring `@destack/build` as a development dependency transforms its tests as a build does.

| Export                    | What it is                                                     |
| ------------------------- | -------------------------------------------------------------- |
| `TestPlan`                | a package's targets, and their Vitest configuration            |
| `TestTarget`              | one runnable set of files in a tier, environment and database  |
| `testOn(database)`        | `test` for one database alone, skipped under another           |
| `scenarioPlugin(modules)` | plays scenario modules through the drivers some modules export |

## Examples

### Assert a refused call

`refusal` turns a refused call into a value, so one assertion checks code, reason and message.

```ts
import { expect, refusal, test } from "@destack/test";

/** Read a page by its identifier. */
declare function readPage(id: string): Promise<unknown>;

test("refuse reading a missing page", async () => {
    expect(await refusal(readPage("p1"))).toEqual([
        "NOT_FOUND",
        "PAGE_NOT_FOUND",
        "page 'p1' not found",
    ]);
});
```

### Declare a scenario

A scenario plays steps through a driver of its interaction, then checks its observations.

```ts
import { defineScenario, type Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

// a counter's steps, observations and globals, which a driver speaks
const counter = {
    name: "counter",
    step: schema.object({ action: schema.literal("increment") }),
    observation: schema.object({ kind: schema.literal("count") }),
    globals: schema.object({}),
} satisfies Interaction;

// count two increments, then observe the count
export const countTwice = defineScenario({
    of: "counter",
    interaction: counter,
    name: "count-twice",
    description: "count up by one for each increment",
    given: { examples: [] },
    when: [{ action: "increment" }, { action: "increment" }],
    parameters: {},
    then: [{ observe: { kind: "count" }, is: 2 }],
});
```

### Bound an operation

An operation step repeats a step and checks the telemetry it recorded; metric checks hold in a seeded world alone, as an external server such as PostgreSQL races, and the bench tier measures time beside the counts.

```ts
import { defineScenario, type Interaction } from "@destack/package/declare";

/** The counter interaction of the scenario above. */
declare const counter: Interaction;

// allow at most one database operation per increment, without errors
export const incrementOnce = defineScenario({
    of: "counter",
    interaction: counter,
    name: "increment",
    description: "increment a counter with one database operation",
    given: { examples: [] },
    parameters: {},
    when: [
        {
            operation: "increment",
            times: 10,
            step: { action: "increment" },
            then: [
                { metric: { name: "db.client.operation.duration" }, atMost: 1 },
                { logs: { severity: "ERROR" }, count: 0 },
            ],
        },
    ],
});
```

### Play scenarios

A `*.scenario.ts` module plays its exported scenarios through the drivers its package's `src/test/index.ts` exports.

```text
src/note/note.scenario.ts    the scenarios, each a defineScenario export
src/test/index.ts            the drivers, such as NotesDriver, beside the package's other test kit
```

## Conventions

### Titles

A title states the scenario and its outcome as one imperative clause, in the code's own nouns.

| Write                                                 | Instead of                                                                                                  |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `key destinations by channel and address`             | `key each destination by its channel and address, matching one address across its other fields`             |
| `rotate an endpoint's secret and release the old one` | `let the installation consume an endpoint's new secret as the manager who sets it, and release the old one` |
| `unsubscribe a browser its push service answers gone` | `forget a browser its push service forgot, as the space, once its push fails as gone`                       |
| `read transport refusals as outcomes`                 | `refuse an email message as its transport refuses it: %#`                                                   |
| `refuse a put under another consumer's name`          | `let installations put their own secrets, refusing another consumer their replacement`                      |
| `conceal a missing version from a stranger`           | `reveal a missing version only to readers of the secret`                                                    |

### Verbs

A title's verb says what the caller observes.

| Verb                   | Outcome                                                |
| ---------------------- | ------------------------------------------------------ |
| `refuse`               | The caller gets an error.                              |
| `conceal`              | The caller gets "not found" for something that exists. |
| `fail`                 | A job ends failed, and no caller gets an error.        |
| `keep`                 | State survives what happened.                          |
| `read`, `list`, `find` | The caller gets one, many, or those matching a filter. |
| a method's own verb    | The test calls that method.                            |

### Scope

A test fails only when its own package is wrong.

### Bodies

A test sets up, acts once and asserts the whole outcome, with one commented block for each.

### Tiers

A test file runs in the tier of what it starts, as Google's test sizes classify tests by the resources they use.

| Tier       | File                    | Starts                            | Slow after | Fails after |
| ---------- | ----------------------- | --------------------------------- | ---------- | ----------- |
| `unit`     | `page.test.ts`          | nothing, or a simulation          | 100 ms     | 5 s         |
| `scenario` | `page.scenario.test.ts` | a prepared image or a server      | 5 s        | 60 s        |
| `bench`    | `page.scenario.ts`      | a scenario's operations, measured | none       | 600 s       |

## Comparison

### Vitest

Tests, assertions and configuration are Vitest's, imported from here.

| Vitest                    | `@destack/test`   | Difference                                                        |
| ------------------------- | ----------------- | ----------------------------------------------------------------- |
| `test`, `expect`, `vi`, … | the same          | The same.                                                         |
| `defineConfig`            | `TestPlan`        | Derives the projects from file names, with no configuration file. |
| `vi.useFakeTimers`        | `testSimulation`  | Simulates every effect of every node, from one seed.              |
| `bench`                   | an operation step | Checks its telemetry, and the bench tier reports time beside it.  |

### Go benchmarks

Benchmark results follow Go's format, so benchstat compares two runs.

| Go               | `@destack/test`   | Difference                                    |
| ---------------- | ----------------- | --------------------------------------------- |
| `BenchmarkX(b)`  | an operation step | Declares the steps it repeats as data.        |
| `b.ReportMetric` | its `then` checks | Counts come from the world's telemetry.       |
| benchstat        | `Runner.save`     | Saves results under a baseline for benchstat. |
