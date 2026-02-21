# Associated Types: Modules

Cross module associated type projection tests live here.

## modules

### export-star barrels preserve associated type projections

> `export *` barrels should preserve associated type projections.
> Projecting from the barrel should keep implementor specialization.

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

### extension implementations satisfy imported associated type contracts

> Extensions can satisfy associated type contracts declared in another module.
> Imported owner projections should expose extension-provided associated members.

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

### type-only and namespace imports agree on associated type projections

> Type-only imports and namespace imports should preserve the same owner semantics.
> Both access paths should project the same specialized associated type.

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

### type-only cycles keep associated type projections available

> Type-only import cycles should not erase associated projection semantics.
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

### type-only cycles preserve namespace associated type projections

> Type-only cycles should preserve namespace-qualified associated projections.
> Owner specialization should not be lost when projecting through a namespace import.

```ds:a.ds
import type { Right } from "./b";

export interface Left<T> {
    type Item = T;
}

export type LeftItem = Right<string>.Item;
```

```ds:b.ds
import type { Left } from "./a";

export class Right<T> implements Left<T> {}
```

```ds:main.ds
import * as api from "./b";

// namespace projection should survive the cyclic type import graph
declare const value: api.Right<string>.Item;
value satisfies string;
```

### mapped associated defaults stay specialized across imports

> Mapped associated defaults should stay specialized after import.
> Imported projections should preserve key sets and transformed value types.

```ds:profile.ds
export interface Profile<Config> {
    type Flags = { [K in keyof Config]: Config[K] extends boolean ? 1 : 0 };
}
```

```ds:service.ds
import type { Profile } from "./profile";

export class ServiceProfile implements Profile<{ critical: boolean, retries: number }> {}
```

```ds:main.ds
import { ServiceProfile } from "./service";

// imported owner projection should preserve mapped alias substitutions
declare const flags: ServiceProfile.Flags;
flags satisfies { critical: 1, retries: 0 };
```

### conditional associated defaults stay specialized across imports

> Conditional associated defaults should stay specialized after import.
> Branch selection should happen with the substituted owner type at projection time.

```ds:policy.ds
export interface Policy<T> {
    type Item = T extends string ? int32 : int16;
}
```

```ds:owner.ds
import type { Policy } from "./policy";

export class NamePolicy implements Policy<string> {}
```

```ds:main.ds
import { NamePolicy } from "./owner";

// imported owner projection should preserve conditional branch selection
declare const item: NamePolicy.Item;
item satisfies int32;
```
