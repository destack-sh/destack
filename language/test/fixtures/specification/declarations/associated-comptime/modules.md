# Associated Comptime Constants: Modules

Cross module associated comptime constant tests live here.

## modules

### associated comptime constants resolve across module boundaries

> Imported classes expose associated comptime projections across module boundaries.
> Defines a class owner in one module and projects its associated constant-driven alias in another.
> The projection must survive import resolution with the same folded value.

```ds:plan.ds
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
    type Segment = uint8[SegmentBytes];
}
```

```ds:main.ds
import { SegmentPlan } from "./plan";

// imported projection should resolve the associated value and alias
declare const segment: SegmentPlan<string>.Segment;
segment satisfies uint8[4096];
```

### associated comptime constants resolve through re export chains

> Re-export chains preserve associated comptime projections.
> Routes the owner through an intermediate re-export module before projection.
> The projected alias must retain associated substitutions across the re-export chain.

```ds:kernel.ds
export newtype Vector<T, comptime N: int> = T;

export interface SimdKernel<T> {
    comptime const LaneWidth: int = 8;
    type Lane = Vector<T, this.LaneWidth>;
}

export class FloatKernel implements SimdKernel<float32> {
    comptime const LaneWidth: int = 16;
}
```

```ds:index.ds
export { FloatKernel } from "./kernel";
export type { SimdKernel } from "./kernel";
```

```ds:main.ds
import { FloatKernel } from "./index";
import type { Vector } from "./kernel";

// re-exported owner should preserve associated substitution metadata
declare const lane: FloatKernel.Lane;
lane satisfies Vector<float32, 16>;
```

### interface abstract associated comptime requirements survive imports

> Imported abstract associated comptime requirements must still be implemented.
> Imports an interface contract and implements it incompletely in another module.
> The missing-associated requirement must still be reported when projecting from the imported class.

```ds:contracts.ds
export interface Tensor2D<T> {
    comptime const Rows: int;
    comptime const Cols: int;
    type Grid = T[this.Rows][this.Cols];
}
```

```ds:impl.ds
import { Tensor2D } from "./contracts";

export class Patch<T> implements Tensor2D<T> {
    comptime const Rows: int = 16;
}
```

```ds:main.ds
import { Patch } from "./impl";

// imported class should still report missing abstract associated comptime members
declare const grid: Patch<float32>.Grid;
```

- contains: missing associated

### associated comptime projections reject unresolved imported generic value usage

> Imported associated comptime projections in value position must remain resolvable.
> Uses an imported owner projection in value space where the generic owner remains unresolved.
> The fixture confirms value-position access is rejected until the projection is compile-time resolvable.

```ds:plan.ds
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}
```

```ds:main.ds
import { SegmentPlan } from "./plan";

function unresolved<Row>() {
    SegmentPlan<Row>.SegmentBytes;
}
```

- contains: resolvable

### associated comptime constants support tensor style shape specialization across modules

> Imported tensor-like shape constants can drive fixed-size nested arrays.
> Specializes tensor-style shape constants in one module and projects the derived alias from another.
> The nested array projection must reflect overridden shape constants after import resolution.

```ds:tensor.ds
export interface PatchShape<T> {
    comptime const Rows: int = 16;
    comptime const Cols: int = 16;
    type Grid = T[this.Rows][this.Cols];
}

export class ImagePatch implements PatchShape<float32> {
    comptime const Rows: int = 32;
    comptime const Cols: int = 32;
}
```

```ds:main.ds
import { ImagePatch } from "./tensor";

// imported owner should project overridden shape constants through the alias
declare const grid: ImagePatch.Grid;
grid satisfies float32[32][32];
```

### associated comptime projections resolve through namespace imports

> Namespace imports preserve associated comptime projection lookup in type and value contexts.
> Accesses associated projections through a namespace-qualified owner path.
> Both type-space and value-space projections must resolve against the same imported owner metadata.

```ds:layout.ds
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
    type Segment = uint8[SegmentBytes];
}
```

```ds:main.ds
import * as layout from "./layout";

// namespace qualification should preserve owner identity for projection
declare const segment: layout.SegmentPlan<string>.Segment;
segment satisfies uint8[4096];

// projection should resolve after module graph resolution
const bytes = layout.SegmentPlan<string>.SegmentBytes;
bytes satisfies number;
```

### cross module codec layout composes associated comptime and associated types

> Cross module projections preserve associated comptime substitutions across re-exported contracts.
> Combines associated constants and aliases inside an interface contract, then projects through re-exported class ownership.
> Header and payload projections must reflect concrete implementor overrides across module boundaries.

```ds:codec.ds
export interface CodecProfile<Frame> {
    comptime const HeaderBytes: number = 16;
    comptime const PayloadRows: number = Frame extends string ? 2 : 4;

    type Header = uint8[this.HeaderBytes];
    type Payload = Frame[this.PayloadRows];
}

export class TextCodec implements CodecProfile<string> {
    comptime const HeaderBytes: number = 24;
}
```

```ds:index.ds
export { TextCodec } from "./codec";
```

```ds:main.ds
import { TextCodec } from "./index";

// value and type projections should survive across re-export chains
declare const header: TextCodec.Header;
header satisfies uint8[24];

// projection should resolve after module graph resolution
declare const payload: TextCodec.Payload;
payload satisfies string[2];
```
