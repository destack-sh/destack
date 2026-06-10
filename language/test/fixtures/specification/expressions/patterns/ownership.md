# Ownership Patterns

Patterns can state the access or ownership they bind with.

## borrow bindings

### borrow field as readonly

Borrow prefixes bind the selected place.

```ds
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

let Point { x: &readonly x } = point;
x satisfies &readonly int32;
```

### borrow field access is explicit

Each binding names the access it takes.

```ds
struct Cell {
    value: int32;
}

declare const cell: Cell;

let Cell { value: &readonly read } = cell;
let Cell { value: &write } = cell;
let Cell { value: &exclusive replace } = cell;

read satisfies &readonly int32;
write satisfies &int32;
replace satisfies &exclusive int32;
```

## move bindings

### move field

Move prefixes bind the selected place by ownership.

```ds
struct Pair {
    left: ^string;
    right: ^string;
}

declare const pair: Pair;

let Pair { left: ^left } = pair;
left satisfies ^string;
```

## dereference patterns

### match through readonly reference

`*` dereferences the scrutinee, then the inner pattern destructures the place.

```ds
struct Point {
    x: int32;
    y: int32;
}

declare const point: &readonly Point;

match (point) {
    *Point { x: &readonly x, y: &readonly y } => {
        x satisfies &readonly int32;
        y satisfies &readonly int32;
    }
}
```

### dereference before borrow

Dereference and reborrow compose in one pattern.

```ds
declare const value: &readonly int32;

match (value) {
    *&readonly inner => {
        inner satisfies &readonly int32;
    }
}
```

### borrow reference itself

Without `*`, the pattern borrows the reference value.

```ds
declare const value: &readonly int32;

match (value) {
    &readonly inner => {
        inner satisfies &readonly &readonly int32;
    }
}
```
