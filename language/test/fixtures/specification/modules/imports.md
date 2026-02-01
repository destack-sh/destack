# Imports

## type-only imports

### type-only imports cannot mix default and named bindings

> Type-only imports must not mix default and named bindings.

```ts:main.ts
import type Foo, { Bar } from "./mod";
```

```ts:mod.ts
export default class Foo {}
export type Bar = string;
```

- contains: type-only imports cannot mix default and named bindings
