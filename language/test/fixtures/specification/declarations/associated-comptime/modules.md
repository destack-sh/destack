# Associated Comptime Constants: Modules

Cross module associated comptime constant tests live here.

## modules

### imports preserve associated comptime projections

> Importing an owner keeps its associated comptime aliases and values available.
> Projecting from the imported owner should use the same specialized value as the source module.

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

### re-exports preserve associated comptime projections

> Re-export chains keep associated comptime projections intact.
> Projections through the re-exported owner should stay specialized to the same value.

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

### imports still enforce required associated comptime members

> Abstract associated comptime requirements are still enforced across module boundaries.
> An imported implementor that omits a required member must still report a missing-associated error.

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

### unresolved imported generic value projections are rejected

> Value-space associated comptime projections must be resolvable at the use site.
> If generic substitutions are unresolved, the value projection is rejected.

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

- invalid static argument: associated comptime projection must be resolvable

### unresolved generic projections stay rejected through re-export chains

> Re-export chains do not make unresolved generic value projections admissible.
> Value-space projection still requires concrete substitutions at the use site.

```ds:plan.ds
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}
```

```ds:barrel.ds
export { SegmentPlan } from "./plan";
```

```ds:main.ds
import { SegmentPlan } from "./barrel";

function unresolved<Row>() {
    SegmentPlan<Row>.SegmentBytes;
}
```

- invalid static argument: associated comptime projection must be resolvable

### type-only cycles keep associated comptime projections available

> Type-only import cycles should not erase associated comptime projection semantics.
> Concrete imported projections remain resolvable even when the module graph is cyclic.

```ds:a.ds
import type { Right } from "./b";

export interface Left<T> {
    comptime const Width: number = T extends string ? 4 : 2;
    type Lane = uint8[this.Width];
}

export type LeftLane = Right<string>.Lane;
```

```ds:b.ds
import type { Left } from "./a";

export class Right<T> implements Left<T> {}
```

```ds:main.ds
import { Right } from "./b";

// associated comptime projections should survive the cyclic type import graph
declare const lane: Right<string>.Lane;
lane satisfies uint8[4];
```

### cyclic re-export projections report one static argument cycle

> Cyclic associated comptime projections across re-exports are rejected deterministically.
> The cycle should report as one static argument cycle at the use site.

```ds:a.ds
import type { Right } from "./bridge-a";

export class Left<T> {
    comptime const Width: number = Right<T>.Height;
}
```

```ds:bridge-a.ds
export { Right } from "./bridge-b";
```

```ds:bridge-b.ds
export { Right } from "./b";
```

```ds:b.ds
import type { Left } from "./a";

export class Right<T> {
    comptime const Height: number = Left<T>.Width;
}
```

```ds:main.ds
import { Left } from "./a";

function unresolved() {
    Left<string>.Width;
}
```

- static argument cycle

### generic recursive projections report static cycles before unresolved projection errors

> Recursive generic associated comptime projections should report a static argument cycle.
> The cycle diagnostic should be primary instead of unresolved projection fallback diagnostics.

```ds:a.ds
import type { Right } from "./b";

export class Left<Row> {
    comptime const Width: number = Right<Row>.Height;
}
```

```ds:b.ds
import type { Left } from "./a";

export class Right<Row> {
    comptime const Height: number = Left<Row>.Width;
}
```

```ds:main.ds
import { Left } from "./a";

function unresolved<Row>() {
    Left<Row>.Width;
}
```

- static argument cycle

### imported owners can specialize tensor-style layout aliases

> Imported associated comptime members can drive fixed-size nested array aliases.
> Overridden shape constants must be reflected in projected layout aliases.

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

### namespace imports expose the same associated comptime projections

> Namespace-qualified owner access should preserve associated projections.
> Type projections and value projections should agree on the same imported owner semantics.

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

### re-exported codec owners preserve associated header and payload projections

> Re-exported owners keep associated comptime and associated type projections coherent.
> Header and payload projections should reflect implementor overrides across module boundaries.

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

### export-star barrels preserve associated comptime projections

> `export *` barrels should preserve both associated type and value projections.
> Projections through the barrel must retain owner specialization.

```ds:layout.ds
export class SegmentLayout<Row> {
    comptime const Width: number = Row extends string ? 32 : 16;
    type Segment = uint8[this.Width];
}
```

```ds:barrel.ds
export * from "./layout";
```

```ds:main.ds
import { SegmentLayout } from "./barrel";

// type projection should stay specialized through export-star indirection
declare const segment: SegmentLayout<string>.Segment;
segment satisfies uint8[32];

// value projection should stay specialized through export-star indirection
const width = SegmentLayout<string>.Width;
width satisfies number;
```

### extension implementations satisfy imported associated comptime contracts

> Extensions can satisfy associated comptime contract requirements defined in another module.
> Imported owner projections should include extension-provided associated members in type and value positions.

```ds:contract.ds
export interface RetryPolicy {
    comptime const MaxRetries: number;
    type Budget = uint8[this.MaxRetries];
}
```

```ds:owner.ds
import { RetryPolicy } from "./contract";

export struct HttpRetryPolicy {}

extension of HttpRetryPolicy implements RetryPolicy {
    comptime const MaxRetries: number = 5;
}
```

```ds:main.ds
import { HttpRetryPolicy } from "./owner";

// extension-provided contract members should project from the owner type
declare const budget: HttpRetryPolicy.Budget;
budget satisfies uint8[5];

// extension-provided associated comptime values should project in value space too
const retries = HttpRetryPolicy.MaxRetries;
retries satisfies number;
```

### type-only and namespace imports agree on associated comptime projections

> Type-only and namespace imports should project the same associated owner semantics.
> Both access paths preserve specialization and projection results.

```ds:plan.ds
export class ChunkPlan<Row> {
    comptime const ChunkBytes: number = Row extends string ? 12 : 6;
    type Chunk = uint8[this.ChunkBytes];
}
```

```ds:index.ds
export type { ChunkPlan } from "./plan";
```

```ds:main.ds
import type { ChunkPlan } from "./index";
import * as api from "./plan";

// type-only import path should preserve associated type projection
declare const chunk: ChunkPlan<string>.Chunk;
chunk satisfies uint8[12];

// namespace import path should preserve associated comptime value projection
const bytes = api.ChunkPlan<string>.ChunkBytes;
bytes satisfies number;
```

### extension-provided projections survive re-exports and namespace imports

> Extension-provided associated comptime values should survive re-export indirection.
> Named imports and namespace imports should resolve the same projected value.

```ds:contract.ds
export interface RetryPolicy {
    comptime const MaxRetries: number;
}
```

```ds:owner.ds
import { RetryPolicy } from "./contract";

export struct HttpRetryPolicy {}

extension of HttpRetryPolicy implements RetryPolicy {
    comptime const MaxRetries: number = 5;
}
```

```ds:index.ds
export { HttpRetryPolicy } from "./owner";
```

```ds:main.ds
import { HttpRetryPolicy } from "./index";
import * as owner from "./owner";

// re-export named import path should preserve extension-provided value projection
const retries = HttpRetryPolicy.MaxRetries;
retries satisfies number;

// owner namespace import path should preserve extension-provided value projection
const ownerRetries = owner.HttpRetryPolicy.MaxRetries;
ownerRetries satisfies number;
```

### associated comptime defaults compose with mapped aliases across imports

> Imported owners should preserve associated comptime defaults used by mapped associated aliases.
> Projections should keep both branch selection and mapped transformation semantics.

```ds:profile.ds
export interface ServiceProfile<Config> {
    comptime const RetryBudget: number = Config extends { critical: true } ? 10 : 3;
    type BudgetWindow = uint8[this.RetryBudget];
    type Flags = { [K in keyof Config]: Config[K] extends boolean ? 1 : 0 };
}
```

```ds:owner.ds
import type { ServiceProfile } from "./profile";

export class CriticalProfile implements ServiceProfile<{ critical: true, enabled: boolean }> {}
```

```ds:main.ds
import { CriticalProfile } from "./owner";

// imported owner should preserve associated comptime default projection
declare const window: CriticalProfile.BudgetWindow;
window satisfies uint8[10];

// imported owner should preserve mapped associated alias projection
declare const flags: CriticalProfile.Flags;
flags satisfies { critical: 1, enabled: 1 };
```

### namespace imports preserve conditional associated comptime projections

> Namespace imports should preserve conditional associated comptime projections.
> Value and alias projections should agree through namespace-qualified owners.

```ds:plan.ds
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
    type Segment = uint8[SegmentBytes];
}
```

```ds:main.ds
import * as api from "./plan";

const bytes = api.SegmentPlan<string>.SegmentBytes;
bytes satisfies number;

declare const segment: api.SegmentPlan<string>.Segment;
segment satisfies uint8[4096];
```

### renamed re-exports preserve mapped aliases that use associated comptime defaults

> Renamed re-exports should preserve mapped aliases that depend on associated comptime defaults.
> Alias forwarding should not drop conditional width selection or mapped payload shapes.

```ds:profile.ds
export interface ServiceProfile<Config> {
    comptime const RetryBudget: number = Config extends { critical: true } ? 10 : 3;
    type BudgetWindow = uint8[this.RetryBudget];
    type Envelope = { [K in keyof Config]: [Config[K], this.BudgetWindow] };
}
```

```ds:owner.ds
import type { ServiceProfile } from "./profile";

export class CriticalProfile implements ServiceProfile<{ critical: true, enabled: boolean }> {}
```

```ds:index.ds
export { CriticalProfile as PublicCriticalProfile } from "./owner";
```

```ds:main.ds
import { PublicCriticalProfile } from "./index";

declare const envelope: PublicCriticalProfile.Envelope;

declare const critical: PublicCriticalProfile.Envelope["critical"];
critical satisfies [true, uint8[10 as comptime]];

declare const enabled: PublicCriticalProfile.Envelope["enabled"];
enabled satisfies [boolean, uint8[10 as comptime]];
```

### multi-hop re-exports preserve contract comptime alias chains

> Contract-owned associated comptime defaults should remain specialized through multi-hop barrels.
> Value and alias projections should stay coherent for imported implementors.

```ds:contract.ds
export interface TileShape<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
    comptime const DoubleWidth: number = this.Width * 2;
    type Tile = uint8[this.DoubleWidth];
}
```

```ds:owner.ds
import type { TileShape } from "./contract";

export class Packet<Row> implements TileShape<Row> {}
```

```ds:barrel1.ds
export { Packet } from "./owner";
```

```ds:barrel2.ds
export { Packet } from "./barrel1";
```

```ds:main.ds
import { Packet } from "./barrel2";

const width = Packet<string>.DoubleWidth;
width satisfies number;

declare const tile: Packet<string>.Tile;
tile satisfies uint8[16];
```

### unresolved generic namespace value projections are rejected

> Namespace value projections from imported generic owners must be resolvable.
> Unresolved generic substitutions are rejected even through namespace-qualified access.

```ds:plan.ds
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}
```

```ds:main.ds
import * as api from "./plan";

function unresolved<Row>() {
    api.SegmentPlan<Row>.SegmentBytes;
}
```

- invalid static argument: associated comptime projection must be resolvable

Cross-module fixed-array disambiguation matrices are owned by `types/operators/indexed-access.md`.