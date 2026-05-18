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

### managed is not a lifetime

Managed storage participates in lifetime inference, but it does not introduce a lifetime literal.

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

### owned borrow across await is allowed

Owned locals stay alive in a suspended async frame.

```ds
declare function ready(): Promise<void>;

async function read(value: int32): Promise<int32> {
    let borrow = &readonly value;
    await ready();
    borrow satisfies &readonly int32;
    return *borrow;
}
```

### local managed borrow across await is rejected

Local managed owners can be reached again after suspension, so their interior borrows are current-turn only.

```ds
class User {
    name: string = "";
}

declare function ready(): Promise<void>;

async function read(user: User): Promise<string> {
    let name = &readonly user.name;
    await ready();
    name satisfies &readonly string;
    return name.clone();
}
```

- contains: suspension

### local managed borrow across yield is rejected

Generator suspension has the same current-turn boundary as async suspension.

```ds
class User {
    name: string = "";
}

function* read(user: User): Generator<string, void, unknown> {
    let name = &readonly user.name;
    yield "ready";
    name satisfies &readonly string;
    return name.clone();
}
```

- contains: suspension

### shared managed borrow across await is rejected

Shared managed storage can be borrowed non-exclusively, but managed-rooted interior borrows still cannot survive suspension.

```ds
class User {
    name: string = "";
}

declare function ready(): Promise<void>;

async function read(user: shared User): Promise<string> {
    let name = &readonly user.name;
    await ready();
    name satisfies shared &readonly string;
    return name.clone();
}
```

- contains: suspension

### mutable parameter borrow across await is source-checked

Borrowed parameters may cross suspension when the caller proves the source is suspension-stable.

```ds
declare function ready(): Promise<void>;

async function read(value: &int32): Promise<int32> {
    await ready();
    value satisfies &int32;
    return *value;
}
```

### readonly parameter borrow across await is source-checked

Readonly borrowed parameters follow the same source proof rule.

```ds
declare function ready(): Promise<void>;

async function read(value: &readonly int32): Promise<int32> {
    await ready();
    value satisfies &readonly int32;
    return *value;
}
```

### async borrow cannot escape its owner

Returned borrowed access still needs an owner that outlives the promise result.

```ds
class User {
    name: string = "";
}

async function read(): Promise<&readonly string> {
    let user: ^User = new User();
    return &readonly user.name;
}
```

- contains: lifetime

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

### shared owned borrow across await is allowed

Shared placement does not remove the uniqueness proof of owned storage.

```ds
struct Packet {
    id: int32;
}

declare function ready(): Promise<void>;

async function read(packet: shared ^Packet): Promise<int32> {
    let id = &readonly packet.id;
    await ready();
    id satisfies shared &readonly int32;
    return *id;
}
```

### exclusive borrow across await is rejected

Exclusive borrowed access must not cross suspension.

```ds
declare function ready(): Promise<void>;

async function write(value: &exclusive int32): Promise<void> {
    await ready();
    *value = 1;
}
```

- contains: exclusive

### local exclusive borrow across await is rejected

Exclusive borrowed access from a local value must not cross suspension.

```ds
declare function ready(): Promise<void>;

async function write(value: int32): Promise<void> {
    let exclusive = &exclusive value;
    await ready();
    *exclusive = 1;
}
```

- contains: exclusive

### exclusive borrow after await is allowed

Exclusive access can begin after resuming.

```ds
declare function ready(): Promise<void>;

async function write(value: int32): Promise<void> {
    await ready();
    let exclusive = &exclusive value;
    *exclusive = 1;
}
```

### owned borrow across yield is allowed

Owned locals stay alive in a suspended generator frame.

```ds
function* read(value: int32): Generator<int32, void, unknown> {
    let borrow = &readonly value;
    yield 1;
    borrow satisfies &readonly int32;
}
```

### borrow after yield is allowed

Borrowed access can begin after resuming.

```ds
function* read(value: int32): Generator<int32, void, unknown> {
    yield 1;
    let borrow = &value;
    borrow satisfies &int32;
}
```

### exclusive borrow across yield is rejected

Exclusive borrowed access must not cross generator suspension.

```ds
function* write(value: &exclusive int32): Generator<void, void, unknown> {
    yield;
    *value = 1;
}
```

- contains: exclusive

### local exclusive borrow across yield is rejected

Exclusive borrowed access from a local value must not cross generator suspension.

```ds
function* write(value: int32): Generator<void, void, unknown> {
    let exclusive = &exclusive value;
    yield;
    *exclusive = 1;
}
```

- contains: exclusive
