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

## import aliases

### import aliases accept qualified identifiers

```ts:main.ts
namespace bar {
    export const baz = 1;
}

import Foo = bar.baz;
```

### import aliases reject non-identifier targets

> Import aliases must target a qualified identifier path.

```ts:main.ts
function bar() {
    return 1;
}

import Foo = bar();
```

- contains: import aliases must target a qualified identifier path
