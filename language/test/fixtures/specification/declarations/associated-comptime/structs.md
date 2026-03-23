# Associated Comptime Constants: Structs

Struct associated comptime constant tests live here.

## structs

### struct can declare associated comptime constants

> Structs can declare associated compile-time constants.
> Declares an associated constant directly on a struct owner and reuses it in a sibling field type.
> Projection must expose the folded literal from the specialized struct owner.

```ds
struct SensorBatch<T> {
    comptime const SampleCount: number = 64;
    data: T[SampleCount];
}

declare const count: SensorBatch<float64>.SampleCount;
count satisfies 64;
```

### struct associated comptime constants can drive sibling associated types

> Struct associated types can project sibling associated comptime constants.
> Feeds a struct associated constant into a struct associated type alias.
> The alias projection confirms type/value composition in one owner scope.

```ds
struct TileMap<T> {
    comptime const Columns: number = 8;
    type Row = T[Columns];
}

declare const row: TileMap<uint8>.Row;
row satisfies uint8[8];
```

### struct associated comptime constants can use static value parameters

> Struct associated comptime constants can depend on outer comptime parameters.
> Uses outer owner value parameters to compute an inner associated constant and alias.
> The projected alias must reflect substituted value arguments after evaluation.

```ds
struct VectorLanes<T, comptime WidthHint: number> {
    comptime const LaneWidth: number = WidthHint * 2;
    type Lane = T[LaneWidth];
}

declare const lane: VectorLanes<uint8, 8>.Lane;
lane satisfies uint8[16];
```

### struct associated comptime constants reject non static initializers

> Struct associated comptime constants reject non static initializers.
> Attempts to initialize a struct associated constant with a runtime-only expression.
> This should fail static evaluation.

```ds
function runtimeColumns(): number {
    8
}

struct BadLayout {
    comptime const Columns: number = runtimeColumns();
}
```

- contains: static expression

### struct associated comptime constants reject declaration without initializer

> Struct associated comptime constants require initializers.
> Declares a struct associated constant without a value.
> The owner declaration must fail because projection requires an initializer-backed compile-time value.

```ds
struct BadLayout {
    comptime const Columns: number;
}
```

- invalid static argument: associated comptime constants require initializer