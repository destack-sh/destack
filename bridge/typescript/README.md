# @destack/language

Destack language client for TypeScript and JavaScript.
This package provides unified backend selection across Node-API and WebAssembly environments.

## Installation

```sh
npm install @destack/language
```

## API

```ts
import { openSession } from "@destack/language";

const session = await openSession({
    root: "/workspace",
    source: {
        files: [
            { path: "destack.json", text: "{\"name\":\"@test/app\"}" },
            { path: "src/index.ds", text: "export const value = 1;" },
        ],
    },
});

console.log(session.backend);
console.log(session.files());
```

## Explicit Backends

```ts
import { openNapiPath } from "@destack/language/napi";
import { openWasmSource } from "@destack/language/wasm";

const nativeSession = await openNapiPath(".");
const webSession = await openWasmSource("/workspace", { files: [] });
```

## Testing

Run these from the repository root.

```sh
just bridge/test
just bridge/check-quick
just bridge/check-full
```
