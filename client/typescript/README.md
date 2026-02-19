# @destack/typescript

Destack client for TypeScript and JavaScript.
This package provides a unified backend selection surface across Node-API and WebAssembly environments.

## Installation

```sh
npm install @destack/typescript
```

## API

```ts
import { createClient } from "@destack/typescript";

const client = await createClient("napi");
console.log(client.backend);
console.log(client.version());
```
