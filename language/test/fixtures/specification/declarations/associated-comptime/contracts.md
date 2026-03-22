# Associated Comptime Constants: Owner Contracts

Owner contract tests for associated comptime constants live here.

## contracts

### interface inheritance carries associated comptime defaults

> Interface inheritance preserves associated comptime defaults across owner chains.
> Builds an interface `extends` chain where downstream aliases depend on inherited and overridden associated constants.
> One implementor uses the inherited default while another overrides it, and both projections must remain coherent.

```ds
interface BaseLayout<T> {
    comptime const Width: number = 64;
    type Row = T[this.Width];
}

interface DenseLayout<T> extends BaseLayout<T> {
    comptime const TileRows: number = 4;
    type Tile = Row[this.TileRows];
}

class FloatDense implements DenseLayout<float32> {}

class FloatDense8 implements DenseLayout<float32> {
    comptime const TileRows: number = 8;
}

// inherited default should flow into both aliases from the base interface
declare const row: FloatDense.Row;
row satisfies float32[64];

// nested alias should reuse the already-substituted row shape
declare const tile: FloatDense.Tile;
tile satisfies float32[64][4];

// override should replace the inherited DenseLayout default while keeping BaseLayout width
declare const tile8: FloatDense8.Tile;
tile8 satisfies float32[64][8];
```

### class owner can satisfy abstract base and interface associated comptime contracts

> A concrete class can satisfy one associated comptime contract for multiple owners.
> Combines an abstract class requirement and an interface requirement for the same associated name.
> A single class override should satisfy both contracts and drive both projections.

```ds
abstract class BasePlan<T> {
    abstract comptime const Width: number;
    type Row = T[this.Width];
}

interface PlanContract<T> {
    comptime const Width: number;
    type Tile = T[this.Width][2];
}

class DensePlan extends BasePlan<float32> implements PlanContract<float32> {
    comptime const Width: number = 16;
}

// one declaration should satisfy both owner contracts in the same class body
declare const row: DensePlan.Row;
row satisfies float32[16];

// interface alias should consume the class-provided override through the contract
declare const tile: DensePlan.Tile;
tile satisfies float32[16][2];
```

### class owner rejects incompatible associated comptime contract overrides

> Owner contract overrides must remain type compatible with inherited requirements.
> Overrides a required associated constant with an incompatible declared type.
> The class should fail contract checking at declaration time.

```ds
interface PlanContract {
    comptime const Width: number = 16;
}

class BadPlan implements PlanContract {
    comptime const Width: string = "wide";
}
```

- not assignable

### owner contracts require implementation through cross module abstract chains

> Cross-module abstract owner chains still require concrete associated comptime implementations.
> Creates a base -> mid -> leaf owner chain across files where the abstract requirement is never fulfilled.
> Importing the leaf must still surface the missing-associated contract violation.

```ds:base.ds
export abstract class BasePlan<T> {
    abstract comptime const Width: number;
    type Row = T[this.Width];
}
```

```ds:contract.ds
export interface PlanContract<T> {
    comptime const Width: number;
    type Tile = T[this.Width][2];
}
```

```ds:mid.ds
import { BasePlan } from "./base";
import { PlanContract } from "./contract";

export abstract class MidPlan<T> extends BasePlan<T> implements PlanContract<T> {}
```

```ds:leaf.ds
import { MidPlan } from "./mid";

export class LeafPlan extends MidPlan<float32> {}
```

```ds:main.ds
import { LeafPlan } from "./leaf";

// unresolved abstract contract should remain visible through the full import chain
declare const row: LeafPlan.Row;
```

- missing associated

### compatible multi-contract associated comptime requirements share one implementation

> One associated comptime implementation can satisfy multiple compatible owner contracts.
> Two interfaces require the same associated constant shape and one class implementation satisfies both.
> Projected aliases from both contracts should agree on the same substituted value.

```ds
interface RowsContract<T> {
    comptime const Width: number;
    type Row = T[this.Width];
}

interface TileContract<T> {
    comptime const Width: number;
    type Tile = T[this.Width][2];
}

class DensePlan implements RowsContract<float32>, TileContract<float32> {
    comptime const Width: number = 8;
}

declare const row: DensePlan.Row;
row satisfies float32[8];

declare const tile: DensePlan.Tile;
tile satisfies float32[8][2];
```
