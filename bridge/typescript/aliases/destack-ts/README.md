# destack-ts

Destack legacy alias package for TypeScript.
This package forwards exports from `@destack/runtime`.

## Installation

```sh
npm install destack-ts
```

## API

```ts
import { createClient } from "destack-ts";

const client = await createClient("napi");
console.log(client.backend);
console.log(client.version());
```
