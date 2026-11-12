# @destack/test

Write and run Destack tests: unit tests on Vitest, simulated worlds, and scenarios played through drivers.

## Overview

Tests import Vitest from here, run their effects in a seeded world, and play declared scenarios through a driver per interaction.

- **Test**: a Vitest test, with `refusal` reading a refused call and `single` reading the only row.
- **World**: a simulation seeded the same in every run, stopped once its test finishes; `testSimulation`.
- **Driver**: takes a scenario's steps and reads its observations; a `DriverType` starts one per scenario.
- **Runner**: plays a scenario through a driver, and benchmarks one; `observe`, `play` and `bench`.
- **Configuration**: Vitest's configuration with Destack's module transform; `defineConfiguration`.

```ts
import { expect, refusal, single, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("read the only row", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("host");
    const row = single([{ id: "note-1" }]);
    expect(row).toEqual({ id: "note-1" });
    expect(await refusal(Promise.resolve())).toEqual("done");
});
```

## Examples

### Configure a package's tests

`defineConfiguration` adds the module transform, and `scenarioPlugin` plays every scenario a `*.scenario.ts` module exports.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";

export default defineConfiguration(
    {
        plugins: [scenarioPlugin([{ module: "@destack/view/test", name: "ViewDriver" }])],
        test: { include: ["src/**/*.test.ts", SCENARIO_MODULES] },
    },
    BunEnvironment.system,
);
```

### Play a scenario

`Runner.observe` plays a scenario's steps through a new driver and returns what it observed after each.

```ts
import type { Interaction, Scenario } from "@destack/package/declare";
import { type DriverType, Runner } from "@destack/test";

async (scenario: Scenario<Interaction, unknown>, driver: DriverType<Interaction, unknown>) => {
    const { result } = await Runner.observe(scenario, driver);
};
```

### Assert a refused call

`refusal` answers a refused call's code, reason and message, or `"done"` when it succeeded.

```ts
import { expect, refusal } from "@destack/test";

async (read: () => Promise<unknown>) => {
    expect(await refusal(read)).toEqual(["NOT_FOUND", "PAGE_NOT_FOUND", "page 'p1' not found"]);
};
```

## Comparison

### Vitest

Tests, assertions and configuration are Vitest's; three things differ.

- Every test runs its effects in a seeded world, so a failure replays with the same seed.
- Scenarios are declared next to the code they exercise and played through drivers, as Playwright drives a page.
- Benchmarks assert counts of operations, and report time beside them.
