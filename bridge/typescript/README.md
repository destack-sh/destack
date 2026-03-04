# @destack/runtime

Destack runtime client for TypeScript and JavaScript.
This package provides unified backend selection across Node-API and WebAssembly environments.
Compatibility packages also ship in [`compat/destack-js`](compat/destack-js/README.md) and [`compat/destack-ts`](compat/destack-ts/README.md).

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
