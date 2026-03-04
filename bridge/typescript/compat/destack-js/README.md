# destack-js

Destack compatibility package for JavaScript.
This package forwards exports from `@destack/runtime`.

## Installation

```sh
npm install destack-js
```

## API

```ts
import { createClient } from "destack-js";

const client = await createClient("napi");
console.log(client.backend);
console.log(client.version());
```
