# Namespace Imports

Tests for namespace imports and export star merging.

## Namespace Imports

### namespace import exposes exported values

> Namespace imports produce a value object with exported members.

```ds:mod.ds
export const count = 1;

export function next(value: number): number {
    return value + 1;
}
```

```ds:main.ds
import * as mod from "./mod.ds";

mod.count satisfies number;
mod.next(1) satisfies number;
```

### namespace import includes export star chains

> Namespace imports include values re-exported via `export *`.

```ds:a.ds
export const left = 1;
```

```ds:b.ds
export const right = "two";
```

```ds:c.ds
export * from "./a.ds";
export * from "./b.ds";
```

```ds:main.ds
import * as all from "./c.ds";

all.left satisfies number;
all.right satisfies string;
```

### namespace import excludes type-only exports

> Type-only exports are not available as namespace values.

```ds:types.ds
export type Alias = number;
export const value = 1;
```

```ds:main.ds
import * as mod from "./types.ds";

mod.value satisfies number;
mod.Alias;
```

- contains: property 'Alias' does not exist

### namespace import resolves module binding exports

> Namespace imports can target declared module bindings.

```ds:bindings.d.ds
export {};

declare module "foo" {
    export const value: number;
    export function make(value: number): string
}
```

```ds:main.ds
import "./bindings.d.ds";
import * as foo from "foo";

foo.value satisfies number;
foo.make(1) satisfies string;
```

### declare module blocks merge exports

> Multiple declare module blocks merge exported members.

```ts:bindings.d.ts
declare module "shape" {
    export interface Box {
        value: string;
    }
}

declare module "shape" {
    export function make(value: string): Box;
}
```

```ts:main.ts
import "./bindings.d.ts";
import { make } from "shape";

const box = make("ok");
box.value satisfies string;
```

## declare namespace restrictions

### declare namespaces reject initializers and bodies

> Declare namespaces cannot include initializers or bodies.

```ts
declare namespace Bad {
    const value = 1;
    function run() {}
    class C {
        field: number = 1;
    }
}
```

- contains: declare bindings cannot have initializers
- contains: invalid function
- contains: invalid member modifier

### ambient const initializers are restricted

> In ambient contexts, const initializers must be literal values or enum references.

```ts
declare module "env" {
    enum E {
        ok = 0,
    }

    export const string = "2";
    export const number = 1.;
    export const bigint = 0n;
    export const negative_bigint = -0n;
    export const negative_number = -1;
    export const template = `-2`;
    export const False = false;
    export const True = true;
    export const E_ok = E.ok;
}
```

> Ambient const initializers reject non-literal expressions.

```ts
declare module "env" {
    export const invalid = globalThis;
}
```

- contains: const initializers in ambient contexts must be literal values or enum references

> Parenthesized literals are not valid ambient const initializers.

```ts
declare module "env" {
    export const parenthesized = (42);
}
```

- contains: const initializers in ambient contexts must be literal values or enum references
