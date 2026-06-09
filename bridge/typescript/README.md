# @destack/language

Destack language bridge for TypeScript and JavaScript.
This package opens language sessions across Node-API and WebAssembly environments.

## Installation

```sh
npm install @destack/language
```

## API

```ts
import { FileEdit, Source, openSession } from "@destack/language";

const session = await openSession(
    Source.memory("/workspace", [
        FileEdit.setText("destack.json", "{\"name\":\"@test/app\"}"),
        FileEdit.setText("src/index.ds", "export const value = 1;"),
    ]),
);

console.log(session.files());
```

## Explicit Backends

```ts
import { openNapiSession } from "@destack/language/napi";
import { openWasmSession } from "@destack/language/wasm";
import { Source } from "@destack/language";

const nativeSession = await openNapiSession(Source.fileSystem("."));
const webSession = await openWasmSession(Source.memory("/workspace", []));
```

## Testing

Run these from the repository root.

```sh
just bridge/test
just bridge/check-quick
just bridge/check-full
```
