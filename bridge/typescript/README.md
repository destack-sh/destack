# @destack/runtime

Destack runtime client for TypeScript and JavaScript.
This package provides unified backend selection across Node-API and WebAssembly environments.

## Installation

```sh
npm install @destack/runtime
```

## API

```ts
import { createClient } from "@destack/runtime";

const client = await createClient("napi");
console.log(client.backend);
console.log(client.version());
```

## Explicit backends

```ts
import { createNapiClient } from "@destack/runtime/napi";
import { createWasmClient } from "@destack/runtime/wasm";

const napiClient = await createNapiClient();
const wasmClient = await createWasmClient();
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just bridge/test

# clean check
just bridge/check-quick

# exhaustive check
just bridge/check-full
```
