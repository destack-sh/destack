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
