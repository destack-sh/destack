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

extension<T> of Crate<T> implements Container<T> {
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

### namespace imports preserve mapped and conditional associated defaults

> Namespace imports should preserve mapped and conditional associated defaults.
> Projection through a namespace path should match direct import behavior.

```ds:contract.ds
export interface Normalize<Row> {
    type Shape = { [K in keyof Row]: Row[K] extends boolean ? 1 : Row[K] };
    type Value = Shape[keyof Shape];
}
```

```ds:owner.ds
import type { Normalize } from "./contract";

export class RuntimeView implements Normalize<{ enabled: boolean, retries: int32 }> {}
```

```ds:main.ds
import * as api from "./owner";

declare const shape: api.RuntimeView.Shape;
shape satisfies { enabled: 1, retries: int32 };

declare const value: api.RuntimeView.Value;
value satisfies 1 | int32;
```

### renamed re-exports preserve mapped and conditional associated defaults

> Renamed re-exports should preserve mapped and conditional associated defaults.
> Alias forwarding should not lose owner substitutions for projections.

```ds:contract.ds
export interface Normalize<Row> {
    type Shape = { [K in keyof Row]: Row[K] extends string ? string : Row[K] };
}
```

```ds:owner.ds
import type { Normalize } from "./contract";

export class UserShape implements Normalize<{ name: string, age: int32 }> {}
```

```ds:index.ds
export { UserShape as PublicUserShape } from "./owner";
```

```ds:main.ds
import { PublicUserShape } from "./index";

declare const shape: PublicUserShape.Shape;
shape satisfies { name: string, age: int32 };
```

### multi-hop re-exports preserve contract alias projection chains

> Contract-owned associated aliases that reference sibling aliases should stay specialized through multi-hop barrels.
> Imported implementors should preserve the full alias chain without dropping substitutions.

```ds:contract.ds
export interface PacketOwner<Row> {
    comptime const Width: number = Row extends string ? 8 : 2;
    type Lane = uint8[this.Width];
    type Packet = this.Lane;
}
```

```ds:owner.ds
import type { PacketOwner } from "./contract";

export class Packet<Row> implements PacketOwner<Row> {}
```

```ds:barrel1.ds
export { Packet } from "./owner";
```

```ds:barrel2.ds
export * from "./barrel1";
```

```ds:main.ds
import { Packet } from "./barrel2";

declare const lane: Packet<string>.Lane;
lane satisfies uint8[8];

declare const packet: Packet<string>.Packet;
packet satisfies uint8[8];
```

### associated projections stay precise inside imported conditional aliases

> Imported associated projections should preserve substituted precision inside conditional aliases.
> Conditional branches should evaluate against the specialized owner projection.

```ds:contract.ds
export interface Response<Row> {
    type Item = Row;
}
```

```ds:owner.ds
import type { Response } from "./contract";

export class UserResponse implements Response<{ id: string, enabled: boolean }> {}
```

```ds:main.ds
import { UserResponse } from "./owner";

type EnabledFlag = UserResponse.Item extends { enabled: true } ? 1 : 0;

declare const flag: EnabledFlag;
flag satisfies 0;
```

### associated projections stay precise inside imported mapped aliases

> Imported associated projections should preserve key and value precision inside mapped aliases.
> Mapped transforms should run on the specialized associated owner type.

```ds:contract.ds
export interface Response<Row> {
    type Item = Row;
}
```

```ds:owner.ds
import type { Response } from "./contract";

export class UserResponse implements Response<{ id: string, enabled: boolean }> {}
```

```ds:main.ds
import { UserResponse } from "./owner";

type Flags = { [K in keyof UserResponse.Item]: UserResponse.Item[K] extends boolean ? 1 : UserResponse.Item[K] };

declare const value: Flags;
value satisfies { id: string, enabled: 1 };
```

### associated type projections are valid in reference annotations across modules

> Cross-module associated projections should remain valid in reference-typed positions.
> Borrowed references should not erase projection specialization.

```ds:contract.ds
export interface Payload<Row> {
    type Item = Row;
}
```

```ds:owner.ds
import type { Payload } from "./contract";

export class TextPayload implements Payload<string> {}
```

```ds:main.ds
import { TextPayload } from "./owner";

function read(value: &TextPayload.Item): void {
}
```

Template literal mapped conditional projection matrices are owned by `types/template-literals/modules.md`.
