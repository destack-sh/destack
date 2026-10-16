# @destack/test

`@destack/test` is Vitest, `defineConfiguration` is Vitest's `defineConfig` with Destack's module metadata, and `Runner` plays declared scenarios through a driver per interaction, as Playwright drives a page.

```ts
import { expect, test } from "@destack/test"; // vitest
// vitest/config's defineConfig
export default defineConfiguration({ test: { include: ["src/**/*.test.ts"] } });
// the code and message of a refused call
expect(await refusal(client.read(pageId))).toEqual(["NOT_FOUND", `no page ${pageId}`]);
const row = single(await database.select().from(page)); // the only row, or a throw
// picks the driver by the interaction's name
Runner.play(createTallyMarkTwice, [TallyDriver, ViewDriver]);
```

## Scenarios

A `DriverType` starts a driver that takes a scenario's steps and reads its observations, and `Runner.observe` returns what it observed.

```ts
import { type DriverType, Runner } from "@destack/test";

const TallyDriver: DriverType<Tallying, Tally> = {
    interaction: tallyInteraction,
    start: (given) => ({
        act: (step) => tally.apply(step),
        observe: (observation) => tally.read(observation),
    }),
};
// [{ marks: "3" }, { marks: "4" }, { marks: "0" }]
await Runner.observe(createTallyMarkTwice, TallyDriver);
```

## Discovery

`scenarioPlugin` plays every scenario a `*.scenario.ts` module exports through the driver types it names.

```ts
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";

export default defineConfiguration({
    plugins: [scenarioPlugin([{ module: "@destack/view/test", name: "ViewDriver" }])],
    test: { include: ["src/**/*.test.ts", SCENARIO_MODULES] },
});
```
