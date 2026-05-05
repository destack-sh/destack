# Borrows

## shared references

### shared reference expression yields reference type

> Reference expressions yield `&readonly T` types.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let shared: &readonly Point = &readonly point;
```

### shared reference is not assignable to owned type

> References are not assignable to owned values.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let value: Point = &readonly point;
```

- contains: not assignable

### shared reference is not assignable to mutable reference

> Shared references cannot be assigned to mutable references.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let shared: &readonly Point = &readonly point;
let mutableRef: &Point = shared;
```

- contains: not assignable

### shared reference rejects member assignment

> Shared references cannot be used to mutate through members.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let shared = &readonly point;
shared.x = 2;
```

- contains: immutable reference

### shared reference rejects nested member assignment

> Shared references are readonly through nested fields.

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

let shared = &readonly user;
shared.profile.name = "Grace";
```

- contains: immutable reference

### shared reference rejects index assignment

> Shared references cannot be used to mutate through index assignment.

```ds
let values: int32[] = [1, 2, 3];
let shared: &readonly int32[] = &readonly values;
shared[0] = 4;
```

- contains: immutable reference

## mutable references

### mutable reference expression yields mutable reference type

> Mutable reference expressions yield `&T` types.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let mutableRef: &Point = &point;
```

### mutable reference allows member assignment

> Mutable references allow member mutation.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let mutableRef: &Point = &point;
mutableRef.x = 2;
mutableRef.x satisfies int32;
```

### mutable reference is not assignable to shared reference

> Mutable references are invariant and do not coerce to shared references.

```ds
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let mutableRef: &Point = &point;
let shared: &readonly Point = mutableRef;
```

- contains: not assignable

## borrow conflicts

### mutable borrow conflicts with shared borrow

> Mutable borrows cannot overlap with shared borrows.

```ds native=true
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &readonly container.data;
    let mutableRef = &container.data;
    sharedRef.value;
    mutableRef.value;
}
```

- contains: cannot borrow as mutable

### mutable borrow conflicts with mutable borrow

> Mutable borrows cannot overlap with other mutable borrows.

```ds native=true
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let firstRef = &container.data;
    let secondRef = &container.data;
    firstRef.value;
    secondRef.value;
}
```

- contains: cannot borrow as mutable

### shared borrows do not conflict

> Shared borrows of the same field can overlap.

```ds native=true
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let firstRef = &readonly container.data;
    let secondRef = &readonly container.data;
    firstRef.value;
    secondRef.value;
}
```

### mutable borrow after shared use is allowed

> Borrows expire after their last use.

```ds native=true
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &readonly container.data;
    sharedRef.value;
    let mutableRef = &container.data;
    mutableRef.value;
}
```

## annotations

### reference annotations are accepted

> Reference annotations can appear in signatures.

```ds
struct Point {
    x: int32;
}

function read(value: &Point): int32 {
    return value.x;
}
```

### owned annotations are accepted

> Owned annotations can appear in signatures.

```ds
struct Point {
    x: int32;
}

function copy(value: ^Point): ^Point {
    return value;
}
```

### generic reference annotations are accepted

> Reference annotations accept generic types.

```ds
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function read<T>(value: &Box<T>): T {
    return value.value;
}
```

### reference annotations can use fixed arrays

> Reference annotations support fixed arrays parameterized by generic values.

```ds
function readLane<comptime N: number>(value: &[uint8; N]): uint8 {
    return value[0];
}
```

### reference annotations can use associated types

> Reference annotations can use associated types in type position.

```ds
interface BufferLike<T> {
    type Item = T;
}

class TextBuffer implements BufferLike<string> {}

function read(value: &TextBuffer.Item): void {
}
```

### reference annotations can use associated constants

> Associated constants can size fixed arrays in reference annotations.

```ds
class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}

function read(value: &[uint8; Segment<string>.Width]): uint8 {
    return value[0];
}
```
