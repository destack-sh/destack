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
- `defineConfiguration`: Vitest's configuration with Destack's module transform and test targets.

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

### Configuration

`defineConfiguration(config, environment)` creates a package's Vitest configuration with the module transform.
Its `tier` sets the tests' timeouts and slow threshold (`TEST_TIERS`: `unit`, `scenario`, `bench`), and its `targets` run the tests once per combination, such as `{ database: ["sqlite", "postgres"] }`.

| Export                    | What it is                                                               |
| ------------------------- | ------------------------------------------------------------------------ |
| `defineProject(config)`   | creates one project of a configuration, in the tier its project declares |
| `scenarioPlugin(drivers)` | plays the scenarios of matching modules                                  |
| `SCENARIO_MODULES`        | `src/**/*.scenario.{ts,tsx}`, which a bench twin of the project measures |

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

A scenario plays steps through a driver of its interaction, and checks the observations after each step.

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

// count two increments, observing the count after each
export const countTwice = defineScenario({
    of: "counter",
    interaction: counter,
    name: "count-twice",
    description: "count up by one for each increment",
    given: { examples: [] },
    when: [{ action: "increment" }, { action: "increment" }],
    then: { observe: { count: { kind: "count" } }, each: [{ count: 1 }, { count: 2 }] },
});
```

### Bound an operation

An operation step repeats a step and checks the telemetry it recorded; the bench tier measures its time beside the counts.

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

`scenarioPlugin` plays the scenarios of `*.scenario.ts` modules through the named drivers.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";

export default defineConfiguration(
    {
        tier: "unit",
        plugins: [scenarioPlugin([{ module: "@destack/view/test", name: "ViewDriver" }])],
        test: { include: ["src/**/*.test.ts", SCENARIO_MODULES] },
    },
    BunEnvironment.system,
);
```

## Comparison

### Vitest

Tests, assertions and configuration are Vitest's, imported from here.

| Vitest                    | `@destack/test`       | Difference                                                       |
| ------------------------- | --------------------- | ---------------------------------------------------------------- |
| `test`, `expect`, `vi`, … | the same              | The same.                                                        |
| `defineConfig`            | `defineConfiguration` | Adds the module transform and runs a project per target.         |
| `vi.useFakeTimers`        | `testSimulation`      | Simulates every effect of every node, from one seed.             |
| `bench`                   | an operation step     | Checks its telemetry, and the bench tier reports time beside it. |

### Go benchmarks

Benchmark results follow Go's format, so benchstat compares two runs.

| Go               | `@destack/test`   | Difference                                    |
| ---------------- | ----------------- | --------------------------------------------- |
| `BenchmarkX(b)`  | an operation step | Declares the steps it repeats as data.        |
| `b.ReportMetric` | its `then` checks | Counts come from the world's telemetry.       |
| benchstat        | `Runner.save`     | Saves results under a baseline for benchstat. |
