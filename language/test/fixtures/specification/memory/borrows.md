# Borrows

## sources

### managed fields can be borrowed

Borrowing through managed storage produces borrowed access.

```ds
class User {
    name: string = "";
}

let user: User = new User();
let name = &user.name;

name satisfies &string;
```

### owned fields can be borrowed

Borrowing an owned field does not move the owner.

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

Readonly borrowed access protects indexed elements.

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

Mutable borrowed access may overlap readonly borrowed access.

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

Mutable borrowed access may overlap another mutable borrow.

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

Disjoint fields do not overlap.

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

A borrow ends at its last use.

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

## receivers

### destinations choose managed or borrowed elements

A typed destination can select the overload that returns the requested form.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension of Bucket<T> {
    at(this: Bucket<T>, index: uint): T {
        return this.items[index];
    }

    at(this: &readonly Bucket<T>, index: uint): &readonly T {
        return &readonly this.items[index];
    }
}

let bucket = new Bucket<int32>();

let item = bucket.at(0);
let value: int32 = bucket.at(0);
let borrowed: &readonly int32 = bucket.at(0);

item satisfies int32;
value satisfies int32;
borrowed satisfies &readonly int32;
```

### element writes can overlap readonly borrows

Writing an existing element does not invalidate an element borrow.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension of Bucket<T> {
    at(this: &readonly Bucket<T>, index: uint): &readonly T {
        return &readonly this.items[index];
    }

    set(this: &Bucket<T>, index: uint, value: T): void {
        this.items[index] = value;
    }
}

let bucket = new Bucket<int32>();
let item = bucket.at(0);

bucket.set(0, 1);
item satisfies &readonly int32;
```

### resizing excludes element borrows

Resizing a collection cannot overlap a borrowed element.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension of Bucket<T> {
    at(this: &readonly Bucket<T>, index: uint): &readonly T {
        return &readonly this.items[index];
    }

    push(this: &exclusive Bucket<T>, value: T): void {
        this.items.push(value);
    }
}

let bucket = new Bucket<int32>();
let item = bucket.at(0);

bucket.push(1);
item satisfies &readonly int32;
```

- contains: cannot borrow as exclusive

### resizing is allowed after last use

The collection can be mutated after the element borrow ends.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension of Bucket<T> {
    at(this: &readonly Bucket<T>, index: uint): &readonly T {
        return &readonly this.items[index];
    }

    push(this: &exclusive Bucket<T>, value: T): void {
        this.items.push(value);
    }
}

let bucket = new Bucket<int32>();
let item = bucket.at(0);

item satisfies &readonly int32;
bucket.push(1);
```

## overloads

### ownership forms select overloads

Overload selection includes ownership.

```ds
struct Packet {
    id: int32;
}

declare function send(packet: Packet): "managed";
declare function send(packet: ^Packet): "owned";

let managed = Packet { id: 1 };
let owned = ^Packet { id: 2 };

send(managed) satisfies "managed";
send(owned) satisfies "owned";
```

### access forms select overloads

Overload selection includes access.

```ds
struct Packet {
    id: int32;
}

declare function inspect(packet: &readonly Packet): "read";
declare function inspect(packet: &Packet): "write";
declare function inspect(packet: &exclusive Packet): "exclusive";

let packet = Packet { id: 1 };

inspect(&readonly packet) satisfies "read";
inspect(&packet) satisfies "write";
inspect(&exclusive packet) satisfies "exclusive";
```

## signatures

### borrowed parameters use surface syntax

Borrowed parameters can use `&T`.

```ds
struct Point {
    x: int32;
}

function read(point: &Point): int32 {
    return point.x;
}
```

### exclusive parameters use surface syntax

Exclusive parameters can use `&exclusive T`.

```ds
struct Point {
    x: int32;
}

function write(point: &exclusive Point): void {
    point.x = 2;
}
```

### borrowed generics preserve type arguments

Borrowed generic types preserve their arguments.

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

Borrowed fixed arrays preserve their length.

```ds
function readLane<comptime N: uint>(value: &[uint8; N]): uint8 {
    return value[0];
}
```
