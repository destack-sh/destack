# Algebra

Memory algebra is the type-level model driving ownership, access, lifetime, and placement.

## constructors

### Managed can name value types

`Managed<T>` is valid for value-shaped types too.

```ds
struct Point {
    x: int32;
}

Managed<Point> satisfies Form<Point, "managed", "ambient">;
```

### shared rewrites space

`shared T` is `WithSpace<T, "shared">`.

```ds
struct Cell {
    value: int32;
}

shared Cell satisfies WithSpace<Cell, "shared">;
```

### owned shared forms commute

Ownership and placement can be written in either order.

```ds
struct Cell {
    value: int32;
}

shared (^Cell) satisfies ^(shared Cell);
shared (^Cell) satisfies WithSpace<^Cell, "shared">;
```

### Local and Ambient rewrite placement

`Local<T>` and `Ambient<T>` are just named helpers for `WithPlace`.

```ds
struct Cell {
    value: int32;
}

Local<Cell> satisfies WithSpace<Cell, "local">;
Ambient<shared Cell> satisfies WithPlace<shared Cell, "ambient">;
```

## accessors

### SpaceOf extracts concrete placement

`SpaceOf<T>` returns concrete placement.

```ds
struct Cell {
    value: int32;
}

SpaceOf<shared Cell> satisfies "shared";
SpaceOf<Cell> satisfies never;
```

### PlaceOf preserves ambient placement

`PlaceOf<T>` can return `"ambient"`.

```ds
struct Cell {
    value: int32;
}

PlaceOf<Cell> satisfies "ambient";
PlaceOf<shared Cell> satisfies "shared";
```

### PlaceOf distributes over unions

Union placement is the union of each arm's placement.

```ds
struct Cell {
    value: int32;
}

PlaceOf<Cell | shared Cell> satisfies "ambient" | "shared";
SpaceOf<Cell | shared Cell> satisfies "shared";
PlaceIn<Cell | shared Cell, "local"> satisfies "local" | "shared";
```

### OwnershipOf extracts explicit ownership

`OwnershipOf<T>` returns the ownership carried by a qualified type.

```ds
struct Cell {
    value: int32;
}

OwnershipOf<^Cell> satisfies "owned";
OwnershipOr<Cell, "managed"> satisfies "managed";
```

### LifetimeOf extracts borrowed lifetime

Borrowed forms carry their lifetime.

```ds
function check<L: Lifetime>(value: Borrowed<int32, L>): void {
    LifetimeOf<typeof value> satisfies L;
}
```

### AccessOf extracts access

All forms carry their access mode.

```ds
struct Cell {
    value: int32;
}

function check<L: Lifetime>(
    readonlyValue: ReadonlyBorrowed<int32, L>,
    value: Borrowed<int32, L>,
    exclusiveValue: ExclusiveBorrowed<int32, L>,
): void {
    AccessOf<Cell> satisfies "mutable";
    AccessOf<readonly Cell> satisfies "readonly";
    AccessOf<^readonly Cell> satisfies "readonly";
    AccessOf<^Cell> satisfies "mutable";
    AccessOf<typeof readonlyValue> satisfies "readonly";
    AccessOf<typeof value> satisfies "mutable";
    AccessOf<typeof exclusiveValue> satisfies "exclusive";
}
```

## rewrites

### WithSpace preserves ownership and access

Changing placement does not change ownership or access.

```ds
struct Cell {
    value: int32;
}

OwnershipOf<WithSpace<^Cell, "shared">> satisfies "owned";
AccessOf<WithSpace<^readonly Cell, "shared">> satisfies "readonly";
```

### WithOwnership preserves placement and access

Changing ownership does not change placement or access.

```ds
struct Cell {
    value: int32;
}

SpaceOf<WithOwnership<shared Cell, "owned">> satisfies "shared";
AccessOf<WithOwnership<readonly Cell, "owned">> satisfies "readonly";
WithOwnership<readonly Cell, "owned"> satisfies ^readonly Cell;
```

### WithLifetime preserves base, place and access

Changing a borrow lifetime leaves the other axes alone.

```ds
function check<A: Lifetime, B: Lifetime>(value: ExclusiveBorrowed<shared int32, A>): void {
    WithLifetime<typeof value, B> satisfies ExclusiveBorrowed<shared int32, B>;
}
```

### WithAccess preserves base and place

Changing access leaves the other axes alone.

```ds
function check<L: Lifetime>(value: Borrowed<shared int32, L>): void {
    WithAccess<Cell, "readonly"> satisfies readonly Cell;
    WithAccess<^Cell, "readonly"> satisfies ^readonly Cell;
    WithAccess<typeof value, "exclusive"> satisfies ExclusiveBorrowed<shared int32, L>;
}
```

### WithBase preserves axes

Changing the base type leaves the form axes alone.

```ds
struct Cell {
    value: int32;
}

struct Payload {
    value: int32;
}

WithBase<shared (^readonly Cell), Payload> satisfies shared (^readonly Payload);
```
