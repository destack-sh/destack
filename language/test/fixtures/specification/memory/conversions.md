# Conversions

Reference conversions follow the memory rules: borrows can weaken, ownership never appears out of thin air, and raw pointers convert in safely but out only unsafely.

## weakening

### mutable borrows weaken to readonly

A readonly reborrow gives up mutation.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let borrow = &point;
let view: &readonly Point = borrow;

view.x satisfies int32;
```

### exclusive borrows weaken to mutable

A reborrow suspends the exclusive loan until the reborrow ends.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let exclusive = &exclusive point;
let borrow: &Point = exclusive;

borrow.x = 2;
```

## ownership

### managed values do not become owned

Ownership is provenance, and a traced handle cannot prove uniqueness.

```ds
class User {}

let user: User = new User();
let owned: ^User = user;
```

- contains: not assignable

### borrows do not become owned

Borrowed access does not own the value.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let borrow = &point;
let owned: ^Point = borrow;
```

- contains: not assignable

### clone bridges managed to owned

A copy is the explicit way to obtain ownership from managed data.

```ds
@derive(Clone)
struct Point {
    x: int32;
}

declare const point: &readonly Point;

let owned: ^Point = point.clone();
```

## raw

### references convert to raw pointers safely

Creating a pointer value is safe; only using it is not.

```ds
class User {}

let user = new User();
let pointer: *User = &user;

pointer satisfies *User;
```

### raw pointers do not reborrow safely

Recovering checked access from a raw pointer is an unsafe claim.

```ds
class User {}

let user = new User();
let pointer: *User = &user;
let borrow: &User = pointer;
```

- contains: unsafe
