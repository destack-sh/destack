# Associated Comptime Constants: Basics

Small associated comptime fixtures to introduce core behavior.

## basic

### class owner can project associated comptime in type and value space

> Class owners can declare associated comptime constants.
> Type and value projections should agree on the same specialized constant.

```ds
class SegmentPlan<Row> {
    comptime const Width: number = Row extends string ? 4 : 2;
    type Lane = Row[Width];
}

declare const lane: SegmentPlan<string>.Lane;
lane satisfies string[4];

const width = SegmentPlan<string>.Width;
width satisfies number;
```

### interface default associated comptime flows to implementors

> Interface defaults should be usable without an explicit override.
> Projecting from the implementor should use the default constant.

```ds
interface Batch<Row> {
    comptime const Size: number = 8;
    type Chunk = Row[this.Size];
}

struct Logs implements Batch<string> {}

declare const chunk: Logs.Chunk;
chunk satisfies string[8];
```

### associated comptime declarations can reference sibling members through this

> Inside associated comptime declarations, `this` refers to the owner with substitutions applied.
> Sibling associated comptime references through `this` should fold after owner substitution.

```ds
class Layout<Row> {
    comptime const Width: number = Row extends string ? 4 : 2;
    comptime const DoubleWidth: number = this.Width * 2;
    type Lane = Row[this.DoubleWidth];
}

declare const lane: Layout<string>.Lane;
lane satisfies string[8];
```

### extension implementor can provide required associated comptime

> Extensions can satisfy associated comptime requirements.
> Owner projections should include extension-provided constants.

```ds
interface Window<Row> {
    comptime const Rows: number;
    type Slice = Row[this.Rows];
}

struct Data<Row> {}

extension<Row> of Data<Row> implements Window<Row> {
    comptime const Rows: number = 16;
}

declare const slice: Data<uint8>.Slice;
slice satisfies uint8[16];
```

### unresolved generic value projection is rejected

> Value-space projections must be compile-time resolvable.
> Unresolved generic owners cannot be projected as values.

```ds
class Plan<Row> {
    comptime const Width: number = Row extends string ? 4 : 2;
}

function unresolved<Row>() {
    const width = Plan<Row>.Width;
    width
}
```

- contains: resolvable

### associated comptime projections can drive fixed array aliases

> Associated comptime projections can be used as fixed array sizes with `as comptime`.
> The projection should fold from a specialized owner before array sizing.

```ds
class SegmentPlan<Row> {
    comptime const Width: number = Row extends string ? 4 : 2;
}

type Lane = uint8[SegmentPlan<string>.Width as comptime];

declare const lane: Lane;
lane satisfies uint8[4];
```

### associated comptime projections stay out of instance value space

> Associated comptime constants are projected from owners, not runtime instances.
> Instance member access should not expose associated comptime constants.

```ds
class SegmentPlan<Row> {
    comptime const Width: number = Row extends string ? 4 : 2;
}

const plan = new SegmentPlan<string>();
const width = plan.Width;
```

- property 'Width' does not exist on type SegmentPlan<string>