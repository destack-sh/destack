# Borrows

## sources

### managed fields can be borrowed

Managed values can produce borrowed access.

```ds
class User {
    name: string = "";
}

let user: User = new User();
let name = &user.name;

name satisfies &string;
```

### owned fields can be borrowed

Borrowing an owned value does not move it.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &readonly point.x;

x satisfies &readonly int32;
point.x satisfies int32;
```

## readonly

### readonly borrows may overlap

Readonly borrows of the same place can overlap.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let first = &readonly point.x;
let second = &readonly point.x;

first satisfies &readonly int32;
second satisfies &readonly int32;
```

### readonly borrows are deep

Readonly borrowed access cannot mutate nested fields.

```ds
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

let user = User {
    profile: Profile { name: "Ada" },
};

let borrow = &readonly user;
borrow.profile.name = "Grace";
```

- contains: readonly

### readonly borrows protect indexed elements

Readonly borrowed access cannot mutate through indexes.

```ds
let values: int32[] = [1, 2, 3];
let borrow = &readonly values;

borrow[0] = 4;
```

- contains: readonly

## mutable

### mutable borrows can mutate

Mutable borrowed access can mutate through the borrow.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &point.x;

*x = 2;
```

### mutable borrows may overlap readonly borrows

Mutable borrowed access may alias.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let read = &readonly point.x;
let write = &point.x;

read satisfies &readonly int32;
*write = 2;
```

### mutable borrows may overlap mutable borrows

Mutable borrowed access may alias another mutable borrow.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let first = &point.x;
let second = &point.x;

*first = 2;
*second = 3;
```

## exclusive

### exclusive borrows can mutate

Exclusive borrowed access can mutate through the borrow.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &exclusive point.x;

*x = 2;
```

### exclusive borrows exclude readonly borrows

Exclusive borrowed access cannot overlap another borrow of the same place.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let read = &readonly point.x;
let write = &exclusive point.x;

read satisfies &readonly int32;
*write = 2;
```

- contains: cannot borrow as exclusive

### exclusive borrows exclude mutable borrows

Exclusive borrowed access cannot overlap ordinary mutable borrowed access.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let first = &exclusive point.x;
let second = &point.x;

*first = 2;
*second = 3;
```

- contains: cannot borrow as exclusive

## paths

### disjoint fields can be borrowed separately

Borrow checking is based on access paths.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point = ^Point { x: 1, y: 2 };
let x = &readonly point.x;
let y = &point.y;

x satisfies &readonly int32;
*y = 3;
```

### borrow ends after last use

Last use ends the borrow.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let read = &readonly point.x;

read satisfies &readonly int32;

let write = &exclusive point.x;
*write = 2;
```

## signatures

### borrowed parameters use surface syntax

Common borrowed parameters use `&T`.

```ds
struct Point {
    x: int32;
}

function read(point: &Point): int32 {
    return point.x;
}
```

### exclusive parameters use surface syntax

Exclusive borrowed parameters use `&exclusive T`.

```ds
struct Point {
    x: int32;
}

function write(point: &exclusive Point): void {
    point.x = 2;
}
```

### borrowed generics preserve type arguments

Borrowed access composes with generic types.

```ds
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function read<T>(box: &Box<T>): T {
    return box.value;
}
```

### borrowed fixed arrays keep their length

Borrowed access composes with fixed arrays.

```ds
function readLane<comptime N: number>(value: &[uint8; N]): uint8 {
    return value[0];
}
```
