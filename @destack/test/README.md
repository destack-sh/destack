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
