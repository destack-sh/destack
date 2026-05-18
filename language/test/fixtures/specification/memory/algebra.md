# Algebra

Memory algebra is the type-level model driving ownership, access, lifetime, and placement.

## constructors

### Managed can name value types

`Managed<T>` is valid for value-shaped types too.

```ds
struct Point {
    x: int32;
}

Managed<Point> satisfies Managed<Point>;
```

### forms compose without erasing payloads

Ownership forms preserve the payload form.

```ds
struct Cell {
    value: int32;
}

Owned<Borrowed<Cell, "static">> satisfies Owned<Borrowed<Cell, "static">>;
Borrowed<Owned<Cell>, "static"> satisfies Borrowed<Owned<Cell>, "static">;
BaseOf<Owned<Borrowed<Cell, "static">>> satisfies Cell;
PayloadOf<Owned<Borrowed<Cell, "static">>> satisfies Borrowed<Cell, "static">;
```

### local and shared place values

`local T` and `shared T` are surface forms for concrete placement.

```ds
struct Cell {
    value: int32;
}

local Cell satisfies Placed<Cell, "local">;
shared Cell satisfies Placed<Cell, "shared">;
```

### owned placement forms commute

Ownership and placement can be written in either order.

```ds
struct Cell {
    value: int32;
}

local ^Cell satisfies ^local Cell;
local ^Cell satisfies WithSpace<^Cell, "local">;
shared ^Cell satisfies ^shared Cell;
shared ^Cell satisfies WithSpace<^Cell, "shared">;
```

### Local and Ambient rebase placement

`Local<T>` and `Ambient<T>` are named helpers for placement.

```ds
struct Cell {
    value: int32;
}

Local<Cell> satisfies Placed<Cell, "local">;
Ambient<shared Cell> satisfies Placed<Cell, "ambient">;
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
PlaceOf<local Cell> satisfies "local";
PlaceOf<shared Cell> satisfies "shared";
```

### PlaceOf distributes over unions

Union placement is the union of each arm's placement.

```ds
struct Cell {
    value: int32;
}

PlaceOf<Cell | local Cell | shared Cell> satisfies "ambient" | "local" | "shared";
SpaceOf<Cell | local Cell | shared Cell> satisfies "local" | "shared";
PlaceIn<Cell | local Cell | shared Cell, "local"> satisfies "local" | "shared";
```

### OwnershipOf extracts outer ownership

`OwnershipOf<T>` returns the outer ownership carried by a qualified type.

```ds
struct Cell {
    value: int32;
}

OwnershipOf<^Cell> satisfies "owned";
OwnershipOf<Owned<Borrowed<Cell, "static">>> satisfies "owned";
OwnershipOf<Borrowed<Owned<Cell>, "static">> satisfies "borrowed";
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

Borrowed and readonly forms carry access.

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

### WithSpace rebases placement

Changing placement removes any existing placement and preserves the rest of the form.

```ds
struct Cell {
    value: int32;
}

OwnershipOf<WithSpace<^Cell, "shared">> satisfies "owned";
AccessOf<WithSpace<^readonly Cell, "shared">> satisfies "readonly";
PayloadOf<WithSpace<^Cell, "shared">> satisfies ^Cell;
WithSpace<shared ^Cell, "local"> satisfies Placed<^Cell, "local">;
```

### WithOwnership adds ownership

Changing ownership wraps the existing form.

```ds
struct Cell {
    value: int32;
}

SpaceOf<WithOwnership<shared Cell, "owned">> satisfies "shared";
AccessOf<WithOwnership<readonly Cell, "owned">> satisfies "readonly";
WithOwnership<readonly Cell, "owned"> satisfies Owned<readonly Cell>;
```

### WithLifetime preserves payload and access

Changing a borrow lifetime leaves the payload and access alone.

```ds
function check<A: Lifetime, B: Lifetime>(value: ExclusiveBorrowed<shared int32, A>): void {
    WithLifetime<typeof value, B> satisfies ExclusiveBorrowed<shared int32, B>;
    LifetimeOf<Owned<Borrowed<int32, A>>> satisfies A;
}
```

### WithAccess projects borrow access

Changing access leaves the payload and placement alone.

```ds
function check<L: Lifetime>(value: Borrowed<shared int32, L>): void {
    WithAccess<Cell, "readonly"> satisfies readonly Cell;
    WithAccess<^Cell, "readonly"> satisfies ^readonly Cell;
    WithAccess<typeof value, "exclusive"> satisfies ExclusiveBorrowed<shared int32, L>;
}
```

### WithBase preserves composed forms

Changing the base type leaves the composed form alone.

```ds
struct Cell {
    value: int32;
}

struct Payload {
    value: int32;
}

WithBase<shared ^readonly Cell, Payload> satisfies shared ^readonly Payload;
```
