# Associated Comptime Constants: Interfaces

Interface associated comptime constant tests live here.

## interfaces

### interface can declare abstract associated comptime constants

> Interfaces can declare abstract associated comptime constants.
> Defines a pure requirement on an interface owner without providing a default value.
> This establishes the contract shape implementors must satisfy and project from concrete owners.

```ds
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number;
    type Segment = Row[this.SegmentBytes];
}

class AuditStore implements PartitionedStore<string> {
    comptime const SegmentBytes: number = 1024;
}

declare const segment: AuditStore.Segment;
segment satisfies string[1024];
```

### interface can declare default associated comptime constants

> Interfaces can provide default associated comptime constants.
> Provides an interface default and projects it from a concrete implementor that does not override it.
> The projection must expose the inherited default literal.

```ds
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number = 1024;
    type Segment = Row[this.SegmentBytes];
}

struct MetricStore implements PartitionedStore<float64> {}

declare const width: MetricStore.SegmentBytes;
width satisfies 1024;
```

### interface default can reference sibling associated comptime constants

> Interface defaults can compose through other associated comptime constants.
> Chains one default associated constant through another on the same interface owner.
> The projection verifies sibling default references are evaluated in owner scope.

```ds
interface BatchLayout<Row> {
    comptime const Columns: number = 64;
    comptime const TileBytes: number = this.Columns * 4;
    type Tile = Row[this.Columns];
}

class FloatLayout implements BatchLayout<float32> {}

declare const bytes: FloatLayout.TileBytes;
bytes satisfies 256;
```

### class implementor can override interface associated comptime constants

> Implementors can override interface defaults with compatible values.
> Overrides an interface default in a class implementor with a compatible value.
> The projected constant must come from the implementor, not the interface default.

```ds
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number = 1024;
    type Segment = Row[this.SegmentBytes];
}

class AuditStore implements PartitionedStore<string> {
    comptime const SegmentBytes: number = 4096;
}

// inherited contracts should apply before projection
declare const width: AuditStore.SegmentBytes;
width satisfies 4096;
```

### implementor must define abstract associated comptime constants

> Implementors must satisfy abstract interface associated comptime constants.
> Leaves an abstract interface associated constant unimplemented in a class owner.
> The class declaration should fail with a missing-associated requirement diagnostic.

```ds
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number;
}

class AuditStore implements PartitionedStore<string> {}
```

- contains: missing associated

### implementor override must satisfy interface associated comptime constant type

> Implementor overrides must match interface associated comptime constant type.
> Provides an override with an incompatible annotation for a required interface associated constant.
> The contract check should fail with an assignability diagnostic.

```ds
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number;
}

class BadStore implements PartitionedStore<string> {
    comptime const SegmentBytes: string = "large";
}
```

- contains: not assignable

### extension implementor can satisfy abstract interface associated comptime constants

> Extensions implementing interfaces can provide associated comptime constants.
> Satisfies an interface associated constant requirement from an extension owner.
> The projected constant must resolve through the extension implementation path.

```ds
interface RetryPolicy {
    comptime const MaxRetries: number;
}

struct HttpRetryPolicy {}

extension of HttpRetryPolicy implements RetryPolicy {
    comptime const MaxRetries: number = 5;
}

// inherited contracts should apply before projection
declare const retries: HttpRetryPolicy.MaxRetries;
retries satisfies 5;
```

### interface associated comptime constants can drive vector lane aliases

> Interface associated comptime constants can parameterize vector-style aliases.
> Uses an associated constant to drive a vector-width alias in the same interface.
> Projection validates width substitution into the alias on a concrete implementor.

```ds
newtype Vector<T, comptime N: int> = T;

interface SimdKernel<T> {
    comptime const LaneWidth: int = 8;
    type Lane = Vector<T, this.LaneWidth>;
}

class FloatKernel implements SimdKernel<float32> {}

declare const lane: FloatKernel.Lane;
lane satisfies Vector<float32, 8>;
```

### interface associated comptime constants can describe fixed tensor layouts

> Interface associated comptime constants can encode rank specific tensor layouts.
> Defines two layout constants and uses both in a nested tensor-style alias.
> Projection checks that both dimensions specialize correctly from implementor overrides.

```ds
interface Tensor2D<T> {
    comptime const Rows: int;
    comptime const Cols: int;
    type Grid = T[this.Rows][this.Cols];
}

class Patch16 implements Tensor2D<float32> {
    comptime const Rows: int = 16;
    comptime const Cols: int = 16;
}

declare const grid: Patch16.Grid;
grid satisfies float32[16][16];
```

### implementing multiple interfaces can share compatible associated comptime constants

> One associated comptime constant can satisfy multiple interfaces when requirements agree.
> Implements two interfaces that require the same associated constant with compatible contracts.
> A single class declaration should satisfy both owners and project one coherent value.

```ds
interface ReadStore {
    comptime const SegmentBytes: number = 1024;
}

interface WriteStore {
    comptime const SegmentBytes: number = 1024;
}

class UnifiedStore implements ReadStore, WriteStore {
    comptime const SegmentBytes: number = 1024;
}

declare const bytes: UnifiedStore.SegmentBytes;
bytes satisfies 1024;
```

### implementing multiple interfaces rejects incompatible associated comptime defaults

> Incompatible associated comptime defaults require an explicit compatible override.
> Implements multiple interfaces that provide conflicting associated constant defaults.
> The class must fail unless it supplies an explicit compatible reconciliation.

```ds
interface ReadStore {
    comptime const SegmentBytes: number = 1024;
}

interface WriteStore {
    comptime const SegmentBytes: number = 2048;
}

class UnifiedStore implements ReadStore, WriteStore {}
```

- invalid static argument: incompatible associated comptime defaults across inherited contracts