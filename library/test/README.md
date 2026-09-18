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

## Conventions

Destack follows Vitest with these conventions:

- Test package behavior; avoid tests of dependencies or re-exports.
- Use `@destack/test/config` for configuration and set explicit timeouts.
- Run `vitest run` through a package script on a Node.js host.
- Use `@destack/test/runner` to collect and run tests programmatically.
- Use `@destack/test/inspect` for JSON descriptions of collected tests and results.
- Mocks, spies, fake timers, and browser runners are excluded.
