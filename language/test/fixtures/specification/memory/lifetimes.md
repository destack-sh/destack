# Lifetimes

## escaping

### returning a local borrow is rejected

Borrowed access cannot outlive the storage it came from.

```ds
struct Point {
    x: int32;
    y: int32;
}

function escapedPoint(): &Point {
    let point = ^Point { x: 1, y: 2 };
    return &point;
}
```

- contains: cannot return reference to local

### returning ownership is allowed

Return owned storage when the value must escape.

```ds
struct Point {
    x: int32;
    y: int32;
}

function returnedPoint(): ^Point {
    let point = ^Point { x: 1, y: 2 };
    return point;
}
```

## generics

### static lifetime names static storage

Static storage is named with the literal `"static"`.

```ds
declare const value: Borrowed<int32, "static">;

value satisfies Borrowed<int32, "static">;
```

### managed field borrows infer real lifetimes

Borrowing through managed storage infers a lifetime for that specific source place.

```ds
class User {
    name: string = "";
}

function nameOf(user: User): &readonly string {
    return &readonly user.name;
}

nameOf(new User()) satisfies &readonly string;
```

### managed branches join through inference

Branches can return managed-rooted borrows without naming the lifetime.

```ds
class User {
    name: string = "";
}

function pick(flag: boolean, a: User, b: User): &readonly string {
    return flag ? &readonly a.name : &readonly b.name;
}

pick(true, new User(), new User()) satisfies &readonly string;
```

### managed arrays keep element borrows precise

Borrowing an array element keeps the array alive without losing the element path.

```ds
function second<T>(items: Array<T>): &readonly T {
    return &readonly items[1];
}

let items: Array<int32> = [1, 2, 3];
let item = second(items);

item satisfies &readonly int32;
```

### managed roots cannot be named as lifetimes

Managed-rooted borrows still have specific inferred lifetimes.

```ds
class User {
    name: string = "";
}

function nameView(user: User): ReadonlyBorrowed<string, "managed"> {
    return &readonly user.name;
}
```

- contains: lifetime

### boxed field borrows cannot outlive the box

A borrow through a box depends on the boxed owner.

```ds
class User {
    name: string = "";
}

function leakedName(user: Box<User>): &readonly string {
    return &readonly user.name;
}
```

- contains: cannot return reference to local

### output borrow can name the input lifetime

Explicit lifetime relationships use static parameters.

```ds
function borrowInput<L: Lifetime>(point: Borrowed<Point, L>): Borrowed<Point, L> {
    return point;
}
```

### item borrow inherits the slice lifetime

Borrowing through an input keeps the input lifetime.

```ds
function first<T, L: Lifetime>(items: Borrowed<[T], L>): Borrowed<T, L> {
    return &items[0];
}
```

### stored borrow carries a lifetime parameter

Types that store borrowed access carry the lifetime they depend on.

```ds
struct View<T, L: Lifetime> {
    items: Borrowed<[T], L>;
}
```

### unrelated input lifetime is rejected

Returned borrowed access must come from the declared lifetime.

```ds
function pick<A: Lifetime, B: Lifetime>(
    a: Borrowed<int32, A>,
    b: Borrowed<int32, B>,
): Borrowed<int32, A> {
    b
}
```

- contains: lifetime

## suspension

### borrow across await is rejected

Borrowed access cannot live across an async suspension point.

```ds
declare function ready(): Promise<void>;

async function read(value: int32): Promise<int32> {
    let borrow = &value;
    await ready();
    let after = borrow;
    after satisfies &int32;
    return value;
}
```

- contains: borrow

### borrow after await is allowed

Borrow again after resuming.

```ds
declare function ready(): Promise<void>;

async function read(value: int32): Promise<int32> {
    await ready();
    let borrow = &value;
    borrow satisfies &int32;
    return value;
}
```

### borrow across yield is rejected

Borrowed access cannot live across a generator suspension point.

```ds
function* read(value: int32): Generator<int32, void, unknown> {
    let borrow = &value;
    yield 1;
    let after = borrow;
    after satisfies &int32;
}
```

- contains: borrow

### borrow after yield is allowed

Borrow again after the generator resumes.

```ds
function* read(value: int32): Generator<int32, void, unknown> {
    yield 1;
    let borrow = &value;
    borrow satisfies &int32;
}
```
