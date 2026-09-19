Write, run, and inspect Destack tests with [Vitest](https://vitest.dev/).

## Usage

Import test declarations and assertions from `@destack/test`.

```ts
import { expect, test } from "@destack/test";
import { encode, decode } from "./message.ts";

test("roundtrip a message", () => {
    const message = { text: "Hello, Destack!" };

    expect(decode(encode(message))).toEqual(message);
});
```
