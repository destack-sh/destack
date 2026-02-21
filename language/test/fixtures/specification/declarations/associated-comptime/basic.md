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

### extension implementor can provide required associated comptime

> Extensions can satisfy associated comptime requirements.
> Owner projections should include extension-provided constants.

```ds
interface Window<Row> {
    comptime const Rows: number;
    type Slice = Row[this.Rows];
}

struct Data<Row> {}

extension<Row> for Data<Row> implements Window<Row> {
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
