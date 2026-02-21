# Associated Comptime Constants: Projections

Associated comptime projection tests live here.

## projections

### type-space indexes remain indexed access in associated aliases

> Type-space indexes inside associated aliases must keep indexed-access semantics.
> This case uses `keyof` and a type-space key parameter to verify `Row[K]` is not reclassified as a fixed-size array.
> Projection should preserve TypeScript-style index semantics even on owners that also declare associated comptime members.

```ds
class Columnar<Row> {
    comptime const Width: number = 8;
    type Cell<K: keyof Row> = Row[K];
}

declare const cell: Columnar<{ id: int32, name: string }>.Cell<"id">;
cell satisfies int32;
```

### as comptime forces fixed-size array interpretation

> `as comptime` forces fixed-size array interpretation in `.ds` type indexes.
> This case keeps the index expression in value-space and marks it explicitly as comptime.
> Projection should fold the static value and produce a fixed-size array type.

```ds
class Columnar<Row, comptime Lanes: number> {
    type Lane = Row[Lanes as comptime];
}

declare const lane: Columnar<uint8, 16>.Lane;
lane satisfies uint8[16];
```

### ambiguous type indexes require as comptime disambiguation

> Unqualified `T[N]` is ambiguous when `N` is not known to be type-space or value-space.
> This case leaves `N` unconstrained and uses the index in an associated alias.
> This should fail and require explicit disambiguation.

```ds
class Columnar<Row, N> {
    type Lane = Row[N];
}
```

- contains: ambiguous

### type level projections can use associated comptime constants

> Associated comptime constant projections are valid in type level expressions.
> Projects a constant-derived alias in pure type position on a concrete owner.
> The projected alias must fold using the specialized associated constant value.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
    type Segment = uint8[SegmentBytes];
}

declare const segment: SegmentPlan<string>.Segment;
segment satisfies uint8[4096];
```

### value level projections can be used when fully resolvable

> Value level associated comptime projections are valid when compile-time resolvable.
> Uses a value-space projection where the owner is fully specialized.
> The expression should remain valid because the projected constant is compile-time resolvable.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

const bytes = SegmentPlan<string>.SegmentBytes + 1;
bytes satisfies number;
```

### value level projections reject unresolved generic substitutions

> Value level associated comptime projections must be resolvable in the current context.
> Uses value-space projection from a generic owner with unresolved substitutions.
> This should fail because the projection is not compile-time resolvable.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

function unresolved<Row>(): number {
    SegmentPlan<Row>.SegmentBytes
}
```

- contains: resolvable

### associated comptime projections reject dynamic instance access

> Associated comptime constants are projected through type owners, not instance values.
> Attempts to access an associated constant through an instance value.
> The fixture ensures access remains owner-qualified and type-static.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

const plan = new SegmentPlan<string>();
const bytes = plan.SegmentBytes;
```

- contains: does not exist

### associated comptime projections reject assignment in value space

> Associated comptime constants are folded compile-time values and not mutable runtime slots.
> Attempts to assign through a projected associated comptime member in value space.
> The assignment target should be rejected because projection is not a runtime storage location.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

function mutate(): void {
    SegmentPlan<string>.SegmentBytes = 2048;
}
```

- contains: invalid assignment target

### associated comptime projections detect cycles

> Cycles in associated comptime projections are rejected.
> Creates a direct cycle between sibling associated constants on one owner.
> Static evaluation should terminate with a cycle diagnostic.

```ds
class CyclicPlan {
    comptime const A: number = this.B;
    comptime const B: number = this.A;
}
```

- contains: cycle

### associated comptime projections compose with associated types

> Associated type projections can consume associated comptime constant projections.
> Feeds an associated constant into an associated type alias on the same owner.
> Projection verifies type/value composition is resolved in owner scope.

```ds
class Batch<Row> {
    comptime const SegmentRows: number = 256;
    type Segment = Row[SegmentRows];
}

declare const segment: Batch<int32>.Segment;
segment satisfies int32[256];
```

### associated comptime projections can parameterize vector width aliases

> Associated comptime values can drive SIMD-style width aliases.
> Uses a computed associated constant as a static argument for a vector-width alias.
> The resulting projection must preserve the folded width literal.

```ds
newtype Vector<T, comptime N: int> = T;

class SimdConfig<T> {
    comptime const Width: int = T extends float32 ? 16 : 8;
    type Lane = Vector<T, Width>;
}

declare const lane: SimdConfig<float32>.Lane;
lane satisfies Vector<float32, 16>;
```

### associated comptime projections reject runtime-only operations in initializer flow

> Projections must remain static-evaluable and cannot depend on runtime-only effects.
> Routes an associated constant through a runtime-only call in its initializer flow.
> The declaration should fail static-evaluation checks.

```ds
class RuntimeOnly {
    static now(): number {
        1024
    }

    comptime const Stamp: number = RuntimeOnly.now();
}
```

- contains: static expression

### associated comptime constants cannot be declared in type aliases

> Type aliases are pure type-space declarations and cannot own associated comptime constants.
> Declares an associated-style comptime member inside a `type` alias body.
> This should reject this owner kind for associated values.

```ds
type BatchShape<T> = {
    comptime const SegmentRows: number = 256;
};
```

- contains: associated comptime
