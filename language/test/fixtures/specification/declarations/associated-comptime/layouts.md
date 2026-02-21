# Associated Comptime Constants: Layout Composition

Layout composition tests for associated comptime constants live here.

## layouts

### kernel profile composes lane width with tensor tile aliases

> Associated comptime constants can drive nested tensor style aliases and substitutions.
> Couples lane-width and tile-row associated constants, then projects both through vector and nested-array aliases.
> The assertions lock coordinated substitution across both aliases on one concrete owner.

```ds
newtype Vector<T, comptime N: int> = T;

interface KernelProfile<T> {
    comptime const LaneWidth: int = T extends float32 ? 16 : 8;
    comptime const TileRows: int = this.LaneWidth * 2;

    type Lane = Vector<T, this.LaneWidth>;
    type Tile = T[this.TileRows][this.LaneWidth];
}

class F32Kernel implements KernelProfile<float32> {}

// both aliases should observe the same substituted lane width
declare const lane: F32Kernel.Lane;
lane satisfies Vector<float32, 16>;

// projected member should reflect substituted operator results
declare const tile: F32Kernel.Tile;
tile satisfies float32[32][16];
```

### extension implementor can project associated comptime tile layout

> Extension implementors can satisfy associated comptime contracts used by tensor style aliases.
> Implements tensor layout requirements in an extension owner instead of a class or struct body.
> Projection verifies associated contract fulfillment through extension-based implementation paths.

```ds
struct ImageBatch<T> {}

interface Tiled<T> {
    comptime const TileRows: int;
    comptime const TileCols: int;
    type Tile = T[this.TileRows][this.TileCols];
}

extension<T> for ImageBatch<T> implements Tiled<T> {
    comptime const TileRows: int = 8;
    comptime const TileCols: int = 8;
}

// extension contract values should project through the owner type
declare const tile: ImageBatch<float32>.Tile;
tile satisfies float32[8][8];
```

### layout aliases can compose arithmetic across sibling associated constants

> Sibling associated comptime constants can compose arithmetic used by shape aliases.
> Builds one associated constant from two sibling constants and reuses it in a second alias.
> The projections confirm arithmetic composition remains compile-time and owner-scoped.

```ds
interface WindowLayout<T> {
    comptime const TileRows: int = 4;
    comptime const TileCols: int = 16;
    comptime const TileArea: int = this.TileRows * this.TileCols;

    type Tile = T[this.TileRows][this.TileCols];
    type Flat = T[this.TileArea];
}

class SampleLayout implements WindowLayout<float32> {}

declare const tile: SampleLayout.Tile;
tile satisfies float32[4][16];

declare const flat: SampleLayout.Flat;
flat satisfies float32[64];
```

### generic value parameters can feed associated layout constants

> Associated comptime constants can derive from owner value static parameters for layout sizing.
> Threads outer value static parameters through associated constants and a shape alias in one owner.
> The projected tile type and byte count must both reflect the same substituted value arguments.

```ds
class BlockLayout<T, comptime BlockRows: int, comptime BlockCols: int> {
    comptime const ElementBytes: int = 4;
    comptime const TileBytes: int = BlockRows * BlockCols * this.ElementBytes;

    type Tile = T[BlockRows][BlockCols];
}

declare const tile: BlockLayout<float32, 8, 8>.Tile;
tile satisfies float32[8][8];

// arithmetic over owner value parameters should remain compile-time resolvable
declare const bytes: BlockLayout<float32, 8, 8>.TileBytes;
bytes satisfies 256;
```
