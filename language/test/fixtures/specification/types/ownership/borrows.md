# Borrows

## shared references

### shared reference expression yields reference type

> Reference expressions yield `&T` types.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let shared: &Point = &point;
```

### shared reference is not assignable to owned type

> References are not assignable to owned values.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let value: Point = &point;
```

- contains: not assignable

### shared reference is not assignable to mutable reference

> Shared references cannot be assigned to mutable references.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let shared: &Point = &point;
let mutableRef: &mut Point = shared;
```

- contains: not assignable

### shared reference rejects member assignment

> Shared references cannot be used to mutate through members.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let shared = &point;
shared.x = 2;
```

- contains: immutable reference

### shared reference rejects index assignment

> Shared references cannot be used to mutate through index assignment.

```ds
let values: int32[] = [1, 2, 3];
let shared: &int32[] = &values;
shared[0] = 4;
```

- contains: immutable reference

## mutable references

### mutable reference expression yields mutable reference type

> Mutable reference expressions yield `&mut T` types.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let mutableRef: &mut Point = &mut point;
```

### mutable reference allows member assignment

> Mutable references allow member mutation.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let mutableRef: &mut Point = &mut point;
mutableRef.x = 2;
mutableRef.x satisfies int32;
```

### mutable reference is not assignable to shared reference

> Mutable references are invariant and do not coerce to shared references.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let mutableRef: &mut Point = &mut point;
let shared: &Point = mutableRef;
```

- contains: not assignable

## borrow conflicts

### mutable borrow conflicts with shared borrow

> Mutable borrows cannot overlap with shared borrows.

```ds native=true
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &container.data;
    let mutableRef = &mut container.data;
    sharedRef.value;
    mutableRef.value;
}
```

- contains: cannot borrow as mutable

### mutable borrow conflicts with mutable borrow

> Mutable borrows cannot overlap with other mutable borrows.

```ds native=true
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let firstRef = &mut container.data;
    let secondRef = &mut container.data;
    firstRef.value;
    secondRef.value;
}
```

- contains: cannot borrow as mutable

### shared borrows do not conflict

> Shared borrows of the same field can overlap.

```ds native=true
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let firstRef = &container.data;
    let secondRef = &container.data;
    firstRef.value;
    secondRef.value;
}
```

### mutable borrow after shared use is allowed

> Borrows expire after their last use.

```ds native=true
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &container.data;
    sharedRef.value;
    let mutableRef = &mut container.data;
    mutableRef.value;
}
```

## borrow modes

### hint mode emits warnings for conflicts

> Hint mode reports borrow conflicts as warnings.

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "borrowMode": "hint" } }
```

```ds
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &container.data;
    let mutableRef = &mut container.data;
    sharedRef.value;
    mutableRef.value;
}
```

- warning: contains: cannot borrow as mutable

### strict mode reports conflicts as errors

> Strict mode reports borrow conflicts as errors.

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "borrowMode": "strict" } }
```

```ds
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &container.data;
    let mutableRef = &mut container.data;
    sharedRef.value;
    mutableRef.value;
}
```

- contains: cannot borrow as mutable
