# Associated Types: Modules

Cross module associated type projection tests live here.

## modules

### associated type projections resolve through export star barrels

> Export-star barrels preserve associated type projections.
> Routes an interface implementor through `export *` and projects an inherited associated type from the barrel consumer.
> The projected type must retain implementor substitutions after module resolution.

```ds:stream.ds
export interface Stream<T> {
    type Item = T;
    next(): Item;
}
```

```ds:counter.ds
import { Stream } from "./stream";

export class Counter implements Stream<int32> {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    next(): Item {
        this.value
    }
}
```

```ds:barrel.ds
export * from "./counter";
```

```ds:main.ds
import { Counter } from "./barrel";

// associated projection should remain specialized through export-star indirection
declare const value: Counter.Item;
value satisfies int32;
```

### owner modules can satisfy imported associated type contracts with extensions

> Owner modules can satisfy imported associated type contracts through extensions.
> Declares the interface in one module and fulfills it in the owner-exporting module.
> The imported owner projection should include the extension-provided associated member.

```ds:contract.ds
export interface Container<T> {
    type Item;
}
```

```ds:owner.ds
import { Container } from "./contract";

export struct Crate<T> {
    value: T;
}

extension<T> for Crate<T> implements Container<T> {
    type Item = T;
}
```

```ds:main.ds
import { Crate } from "./owner";

// extension-provided associated projection should materialize on the owner type
declare const item: Crate<string>.Item;
item satisfies string;
```

### associated type projections remain coherent across namespace and type-only imports

> Type-only and namespace imports should resolve to the same associated type owner semantics.
> Uses `import type` through a barrel and namespace import from the implementor module.
> Both access paths must preserve associated substitution and projection resolution.

```ds:factory.ds
export interface Factory<T> {
    type Output = T;
}
```

```ds:box.ds
import type { Factory } from "./factory";

export class Box<T> implements Factory<T> {}
```

```ds:index.ds
export type { Factory } from "./factory";
```

```ds:main.ds
import * as api from "./box";
import type { Factory } from "./index";

function project<F: Factory<int32>>(value: F.Output): F.Output {
    value
}

// namespace import path should preserve associated default projection
declare const output: api.Box<int32>.Output;
project<api.Box<int32>>(output) satisfies int32;
```

### cyclic module graphs preserve associated type projections

> Type-space cycles across modules should not drop associated projection metadata.
> Creates a two-module cycle where one side imports the interface and the other imports the implementor in type-space.
> Projection from the concrete owner should still resolve in the consumer module.

```ds:a.ds
import type { Right } from "./b";

export interface Left<T> {
    type Item = T;
}

export type LeftItem = Right<int32>.Item;
```

```ds:b.ds
import type { Left } from "./a";

export class Right<T> implements Left<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
```

```ds:main.ds
import { Right } from "./b";

// owner projection should survive the cyclic type import graph
declare const value: Right<int32>.Item;
value satisfies int32;
```
