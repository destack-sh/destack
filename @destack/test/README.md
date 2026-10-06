# @destack/test

Write, run and inspect Destack tests with [Vitest](https://vitest.dev/).

## Tests

`@destack/test` exports `test`, `expect` and the other Vitest functions a test file imports.

```ts
import { expect, test } from "@destack/test";
import { decode, encode } from "./message.ts";

test("roundtrip a message", () => {
    const message = { text: "Hello, Destack!" };

    expect(decode(encode(message))).toEqual(message);
});
```

## Refusals

`refusal` awaits a call and returns its error code and message, or `"done"` when the call succeeds.

```ts
import { expect, refusal, test } from "@destack/test";

test("refuse a stranger", async () => {
    expect(await refusal(client.read(noteId))).toEqual(["NOT_FOUND", `no note ${noteId}`]);
});
```

## Single items

`single` returns the only item of a list and throws when the list has none or several.

```ts
import { single } from "@destack/test";

const row = single(await database.select().from(note));
```

## Scenarios

`Runner.play` plays a scenario as a test through the driver type of its interaction, expecting the observations it names.

```ts
import { type DriverType, Runner } from "@destack/test";

const TallyDriver: DriverType<Tallying, Tally> = {
    interaction: tallyInteraction,
    start: (given) => ({ act: (step) => …, observe: (observation) => … }),
};
Runner.play(MarkTwice, [TallyDriver, ViewDriver]); // picks TallyDriver by the interaction's name
await Runner.observe(MarkTwice, TallyDriver); // [{ marks: "3" }, { marks: "4" }, { marks: "0" }]
```

## Scenario discovery

`scenarioPlugin` plays every scenario a `*.scenario.ts` module exports through the driver types it names, and a configuration includes `SCENARIO_MODULES`.

```ts
import { defineConfiguration, SCENARIO_MODULES, scenarioPlugin } from "@destack/test/config";

export default defineConfiguration({
    plugins: [scenarioPlugin([{ module: "@destack/view/test", name: "ViewDriver" }])],
    test: { include: ["src/**/*.test.ts", SCENARIO_MODULES] },
});
```

## Configuration

`defineConfiguration` and `defineProject` add Destack module metadata to package sources and set the `expect.poll` interval to 5 ms.

```ts
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({ test: { include: ["src/**/*.test.ts"] } });
```

## Inspection

`TestDeclaration` parses a test or suite declaration with its title, file, source offsets and modifiers.

```ts
import { TestDeclaration } from "@destack/test/inspect";

const declaration = TestDeclaration.parse(json); // { kind: "test", name, file, start, end, modifiers }
```
