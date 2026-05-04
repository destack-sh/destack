# Relations

Memory relation aliases expose ownership and placement as type algebra.

## local and shared

### local values can refer to shared values

> Local storage can hold handles to shared values.

```ds
class Registry {}

const registry: shared Registry = new Registry();
const local = { registry };

local.registry satisfies shared Registry;
```

### shared values cannot refer to local values

> Shared storage cannot point directly into a Worker-local heap.

```ds
class LocalBox {}

struct SharedBox {
    value: WithSpace<LocalBox, "local">;
}

let value: shared SharedBox;
```

- contains: shared

## algebra

### Shared rebases values into shared space

`Shared<T>` is the named form of `WithSpace<T, "shared">`.

```ds
struct Cell {
    value: int32;
}

let cell: Shared<Cell>;
cell satisfies shared Cell;
```

### SpaceOf extracts explicit placement

`SpaceOf<T>` returns the placement carried by a qualified type.

```ds
struct Cell {
    value: int32;
}

declare const space: SpaceOf<shared Cell>;
space satisfies "shared";
```

### OwnershipOf extracts explicit ownership

`OwnershipOf<T>` returns the ownership carried by a qualified type.

```ds
struct Cell {
    value: int32;
}

declare const ownership: OwnershipOf<^Cell>;
ownership satisfies "owned";
```

### WithSpace preserves ownership

Changing placement does not change ownership.

```ds
struct Cell {
    value: int32;
}

declare const ownership: OwnershipOf<WithSpace<^Cell, "shared">>;
ownership satisfies "owned";
```

### WithOwnership preserves placement

Changing ownership does not change placement.

```ds
struct Cell {
    value: int32;
}

declare const space: SpaceOf<WithOwnership<shared Cell, "owned">>;
space satisfies "shared";
```
