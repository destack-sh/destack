Write, run and inspect Destack tests with [Vitest](https://vitest.dev/).

## Tests

A test imports its declarations and assertions from `@destack/test`.

```ts
import { expect, test } from "@destack/test";
import { decode, encode } from "./message.ts";

test("roundtrip a message", () => {
    const message = { text: "Hello, Destack!" };

    expect(decode(encode(message))).toEqual(message);
});
```

## Refusals

`refusal` reads a pending call's refusal as its code and message, or `"done"` when the call succeeds.

```ts
import { expect, refusal, test } from "@destack/test";

test("refuse a stranger", async () => {
    expect(await refusal(client.read(noteId))).toEqual(["NOT_FOUND", `no note ${noteId}`]);
});
```

## Single items

`single` reads the only item of a list and throws for none or several.

```ts
import { single } from "@destack/test";

const row = single(await database.select().from(note));
```

## Configuration

`defineConfiguration` and `defineProject` from `@destack/test/config` add Destack module metadata to package sources and poll `expect.poll` every 5 ms.

```ts
import { defineConfiguration } from "@destack/test/config";

export default defineConfiguration({ test: { include: ["src/**/*.test.ts"] } });
```

## Inspection

`TestDeclaration` from `@destack/test/inspect` describes a test or suite a package declares by its title, file, source range and modifiers.

```ts
import { TestDeclaration } from "@destack/test/inspect";

const declaration = TestDeclaration.parse(json); // { kind: "test", name, file, start, end, modifiers }
```
