# Borrows

Borrows take checked access to a live place without moving it.

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

### local managed values can be borrowed exclusively

Local (ambient) managed storage can satisfy exclusive access when no overlapping loan is live.

```ds
class Counter {
    value: int32 = 0;
}

let counter: Counter = new Counter();
let write = &exclusive counter;

write.value = 1;
```

### shared managed fields can be borrowed non-exclusively

Shared managed storage can be borrowed as long as the borrow does not require uniqueness.

```ds
class User {
    name: string = "";
}

let user: shared User = new User();
let name = &readonly user.name;

name satisfies shared &readonly string;
```

### shared managed values cannot be borrowed exclusively

Exclusive borrowed access cannot be proven from a shared managed handle (ever, unfortunately).

```ds
class Counter {
    value: int32 = 0;
}

let counter: shared Counter = new Counter();
let write = &exclusive counter;

write.value = 1;
```

- contains: shared managed

### shared managed arrays cannot grow directly

Growing an array requires exclusive access, which shared managed handles cannot provide.

```ds
let items: shared Array<int32> = Array.from([1, 2, 3]);

items.push(4);
```

- contains: shared managed

### shared managed maps cannot insert directly

Inserting into a map requires exclusive access, which shared managed handles cannot provide.

```ds
let scores: shared Map<string, int32> = Map.from([
    ["ada", 1],
]);

scores.insert("grace", 2);
```

- contains: shared managed

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

### exclusive borrows can split fields

Exclusive borrows of disjoint fields can overlap.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point = ^Point { x: 1, y: 2 };
let x = &exclusive point.x;
let y = &exclusive point.y;

*x = 3;
*y = 4;
```

### parent borrows overlap child borrows

Borrowing a whole value overlaps every field.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point = ^Point { x: 1, y: 2 };
let whole = &readonly point;
let x = &exclusive point.x;

whole.x satisfies int32;
*x = 3;
```

- contains: cannot borrow as exclusive

### variable indexes may overlap fixed indexes

A variable index may name the same element as a fixed index.

```ds
function write(values: &exclusive [int32; 4], index: uint): void {
    let dynamic = &exclusive values[index];
    let first = &exclusive values[0];

    *dynamic = 1;
    *first = 2;
}
```

- contains: cannot borrow as exclusive

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

### reborrows suspend exclusive borrows

Exclusive borrowed access can be reborrowed and then used again after the reborrow ends.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let exclusive = &exclusive point;
let field = &exclusive exclusive.x;

*field = 2;
exclusive.x = 3;
```

## receivers

### expected result type selects value or borrow

The expected result type can select an overload that returns the requested form.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension<T> of Bucket<T> {
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

### field writes can overlap readonly borrows

Writing through ordinary mutable access does not invalidate a field borrow.

```ds
class Cell<T> {
    value: T;
}

extension<T> of Cell<T> {
    get(this: &readonly Cell<T>): &readonly T {
        return &readonly this.value;
    }

    set(this: &Cell<T>, value: T): void {
        this.value = value;
    }
}

let cell = new Cell<int32>();
let value = cell.get();

cell.set(1);
value satisfies &readonly int32;
```

### custom collections can exclude unstable growth

Userland methods can require exclusive receiver access when they may invalidate element borrows.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension<T> of Bucket<T> {
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

### array growth excludes element borrows

Growing an array can move element storage.

```ds
let items: Array<int32> = [1, 2, 3];
let item = &readonly items[1];

items.push(4);
item satisfies &readonly int32;
```

- contains: cannot borrow as exclusive

### returned element borrows exclude array growth

Returning an element borrow keeps that element path live.

```ds
function second<T>(items: Array<T>): &readonly T {
    return &readonly items[1];
}

let items: Array<int32> = [1, 2, 3];
let item = second(items);

items.push(4);
item satisfies &readonly int32;
```

- contains: cannot borrow as exclusive

### array removal excludes element borrows

Removing from an array can invalidate element paths.

```ds
let items: Array<int32> = [1, 2, 3];
let item = &readonly items[1];

items.removeAt(0);
item satisfies &readonly int32;
```

- contains: cannot borrow as exclusive

### array shrink excludes element borrows

Shrinking capacity can move element storage.

```ds
let items: Array<int32> = [1, 2, 3];
let item = &readonly items[1];

items.shrinkToFit();
item satisfies &readonly int32;
```

- contains: cannot borrow as exclusive

### custom collections can mutate after last use

The collection can take exclusive receiver access after the element borrow ends.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension<T> of Bucket<T> {
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

### array growth is allowed after last use

Arrays can grow after the element borrow ends.

```ds
let items: Array<int32> = Array.from([1, 2, 3]);
let item = &readonly items[1];

item satisfies &readonly int32;
items.push(4);
```

### array element replacement excludes element borrows

Replacing an array element requires exclusive access to the array.

```ds
let items: Array<int32> = [1, 2, 3];
let item = &readonly items[1];

items[1] = 4;
item satisfies &readonly int32;
```

- contains: cannot borrow as exclusive

### map insert excludes value borrows

Inserting into a map can move entry storage.

```ds
let scores: Map<string, int32> = Map.from([
    ["ada", 1],
]);

let score: &int32 | undefined = scores.get("ada");

scores.insert("grace", 2);
score satisfies &int32 | undefined;
```

- contains: cannot borrow as exclusive

### map removal excludes value borrows

Removing from a map can invalidate an entry path.

```ds
let scores: Map<string, int32> = Map.from([
    ["ada", 1],
]);

let score: &int32 | undefined = scores.get("ada");

scores.remove("ada");
score satisfies &int32 | undefined;
```

- contains: cannot borrow as exclusive

### method arguments are evaluated before exclusive receivers

Call arguments are evaluated before a method takes exclusive receiver access.

```ds
class Bucket<T> {
    items: T[] = [];
}

extension<T> of Bucket<T> {
    length(this: &readonly Bucket<T>): uint {
        return this.items.length;
    }

    push(this: &exclusive Bucket<T>, value: T): void {
        this.items.push(value);
    }
}

let bucket = new Bucket<uint>();
bucket.push(bucket.length());
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
class Cell<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function read<T>(cell: &Cell<T>): T {
    return cell.value;
}
```

### borrowed fixed arrays keep their length

Borrowed fixed arrays preserve their length.

```ds
function readLane<comptime N: uint>(value: &[uint8; N]): uint8 {
    return value[0];
}
```
