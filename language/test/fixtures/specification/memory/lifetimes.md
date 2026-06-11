# Lifetimes

Lifetimes tie a borrow to its source, and are inferred from bodies wherever possible.

## escaping

### local borrow cannot escape a function

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

### owned return can escape a function

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

## named lifetimes

### static lifetime names static storage

Static storage is named with the literal `"static"`.

```ds
declare const value: Borrowed<int32, "static">;

value satisfies Borrowed<int32, "static">;
```

## inference

### unused arguments do not constrain returned borrows

Only the returned source has to outlive the result.

```ds
struct Node {
    id: int32;
}

function first(a: &Node, b: &Node): &Node {
    return a;
}

function pass(a: &Node): &Node {
    let local = ^Node { id: 1 };
    return first(a, &local);
}
```

### local returned source cannot escape

A function cannot return a borrow rooted in its own frame.

```ds
struct Node {
    id: int32;
}

function first(a: &Node, b: &Node): &Node {
    return a;
}

function escaped(b: &Node): &Node {
    let local = ^Node { id: 1 };
    return first(&local, b);
}
```

- borrow does not live long enough

### conditional borrow result joins lifetimes

A conditional borrow has the joined lifetime of both branches.

```ds
struct Node {
    id: int32;
}

function choose(a: &Node, b: &Node, flag: bool): &Node {
    return flag ? a : b;
}

function pass(a: &Node, b: &Node, flag: bool): &Node {
    return choose(a, b, flag);
}
```

### conditional borrow cannot return local true branch

A returned conditional borrow cannot include a branch rooted in the current function.

```ds
struct Node {
    id: int32;
}

function choose(a: &Node, b: &Node, flag: bool): &Node {
    return flag ? a : b;
}

function escaped(b: &Node, flag: bool): &Node {
    let local = ^Node { id: 1 };
    return choose(&local, b, flag);
}
```

- borrow does not live long enough

### conditional borrow cannot return local false branch

A returned conditional borrow cannot include a branch rooted in the current function.

```ds
struct Node {
    id: int32;
}

function choose(a: &Node, b: &Node, flag: bool): &Node {
    return flag ? a : b;
}

function escaped(a: &Node, flag: bool): &Node {
    let local = ^Node { id: 1 };
    return choose(a, &local, flag);
}
```

- borrow does not live long enough

### declarations reject elided returned borrow lifetimes

Declaration-only signatures must spell returned borrow lifetimes explicitly.

```ds
struct Node {
    id: int32;
}

declare function only(value: &Node): &Node;
```

- declaration-only borrowed return needs an explicit lifetime relationship

### multi-input declarations need a lifetime relationship

A declaration with several borrowed inputs must also state which input roots the result.

```ds
struct Node {
    id: int32;
}

declare function choose(a: &Node, b: &Node): &Node;
```

- declaration-only borrowed return needs an explicit lifetime relationship

### struct can return a borrowed field from input

A struct with a borrowed field can be returned when the field comes from an input.

```ds
struct Engine {
    frame: uint64;
}

struct EngineBorrow {
    engine: &Engine;
}

function borrowEngine(engine: &Engine): EngineBorrow {
    return EngineBorrow { engine };
}
```

### struct cannot return a borrowed field from local

A returned struct cannot contain a borrow rooted in its own function.

```ds
struct Engine {
    frame: uint64;
}

struct EngineBorrow {
    engine: &Engine;
}

function escaped(): EngineBorrow {
    let engine = ^Engine { frame: 1 };
    return EngineBorrow { engine: &engine };
}
```

- borrow does not live long enough

### struct can store several input borrows

A returned struct is valid when every borrowed field comes from an input.

```ds
struct Engine {
    frame: uint64;
}

struct AssetStore {
    count: uint32;
}

struct WorldBorrow {
    engine: &Engine;
    assets: &AssetStore;
}

function borrowWorld(engine: &Engine, assets: &AssetStore): WorldBorrow {
    return WorldBorrow { engine, assets };
}
```

### returned struct cannot contain a local borrow

Every borrowed field in a returned struct must outlive the struct.

```ds
struct Engine {
    frame: uint64;
}

struct AssetStore {
    count: uint32;
}

struct WorldBorrow {
    engine: &Engine;
    assets: &AssetStore;
}

function escaped(engine: &Engine): WorldBorrow {
    let assets = ^AssetStore { count: 1 };
    return WorldBorrow { engine, assets: &assets };
}
```

- borrow does not live long enough

### explicit field lifetime uses a named parameter

Use `Borrowed<T, L>` when a stored field must use a specific lifetime parameter.

```ds
struct Node {
    id: int32;
}

struct NodeBorrow<comptime L: Lifetime> {
    node: Borrowed<Node, L>;
}

function borrowNode<comptime L: Lifetime>(node: Borrowed<Node, L>): NodeBorrow<L> {
    return NodeBorrow { node };
}
```

### explicit field lifetime rejects shorter borrows

A struct with lifetime `L` cannot store a borrow rooted in the current function.

```ds
struct Node {
    id: int32;
}

struct NodeBorrow<comptime L: Lifetime> {
    node: Borrowed<Node, L>;
}

function escaped<comptime L: Lifetime>(): NodeBorrow<L> {
    let node = ^Node { id: 1 };
    return NodeBorrow { node: &node };
}
```

- borrow does not live long enough

### elided stored fields induce distinct lifetimes

Each elided stored borrow gets its own inferred lifetime parameter.

```ds
struct Node {
    id: int32;
}

struct Pair {
    left: &Node;
    right: &Node;
}

function pair(left: &Node, right: &Node): Pair {
    return Pair { left, right };
}
```

### explicit stored fields can name different lifetimes

Different stored borrow lifetimes are written with `Borrowed<T, L>`.

```ds
struct Node {
    id: int32;
}

struct Pair<comptime A: Lifetime, comptime B: Lifetime> {
    left: Borrowed<Node, A>;
    right: Borrowed<Node, B>;
}

function pair<comptime A: Lifetime, comptime B: Lifetime>(
    left: Borrowed<Node, A>,
    right: Borrowed<Node, B>,
): Pair<A, B> {
    return Pair { left, right };
}
```

### managed field borrow follows the owner

The returned borrow is tied to the managed object.

```ds
class User {
    name: string = "";
}

function nameOf(user: User): &readonly string {
    return &readonly user.name;
}

nameOf(new User()) satisfies &readonly string;
```

### dynamic managed conditionals join lifetimes

Either managed object can be the returned owner.

```ds
class User {
    name: string = "";
}

function pick(flag: boolean, a: User, b: User): &readonly string {
    return flag ? &readonly a.name : &readonly b.name;
}

pick(true, new User(), new User()) satisfies &readonly string;
```

### array element borrow follows the array

An element borrow is tied to the array it came from.

```ds
function second<T>(items: Array<T>): &readonly T {
    return &readonly items[1];
}

let items: Array<int32> = [1, 2, 3];
let item = second(items);

item satisfies &readonly int32;
```

### boxed field borrow follows the box

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

## explicit relationships

### output borrow can name an input lifetime

Explicit lifetime relationships use static parameters.

```ds
function borrowInput<comptime L: Lifetime>(point: Borrowed<Point, L>): Borrowed<Point, L> {
    return point;
}
```

### item borrow follows the slice lifetime

Borrowing through an input keeps the input lifetime.

```ds
function first<T, comptime L: Lifetime>(items: Borrowed<[T], L>): Borrowed<T, L> {
    return &items[0];
}
```

### stored borrow can carry a lifetime parameter

Types that store borrowed access carry the lifetime they depend on.

```ds
struct View<T, comptime L: Lifetime> {
    items: Borrowed<[T], L>;
}
```

### output borrow must match its declared lifetime

Returned borrowed access must come from the declared lifetime.

```ds
function pick<comptime A: Lifetime, comptime B: Lifetime>(
    a: Borrowed<int32, A>,
    b: Borrowed<int32, B>,
): Borrowed<int32, A> {
    b
}
```

- contains: lifetime

## suspension

### owned borrow can cross await

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

### local managed borrow cannot cross await

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

### local managed borrow cannot cross yield

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

### shared managed borrow cannot cross await

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

### borrowed parameter can cross await

Borrowed parameters may cross suspension when the caller proves the source is suspension-stable.

```ds
declare function ready(): Promise<void>;

async function read(value: &int32): Promise<int32> {
    await ready();
    value satisfies &int32;
    return *value;
}
```

### readonly parameter borrow can cross await

Readonly borrowed parameters follow the same source proof rule.

```ds
declare function ready(): Promise<void>;

async function read(value: &readonly int32): Promise<int32> {
    await ready();
    value satisfies &readonly int32;
    return *value;
}
```

### async return cannot contain a local borrow

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

### borrow can begin after await

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

### shared owned borrow can cross await

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

### exclusive parameter borrow can cross await

Exclusive borrowed parameters may cross suspension when the caller proves the source is suspension-stable.

```ds
declare function ready(): Promise<void>;

async function write(value: &exclusive int32): Promise<void> {
    await ready();
    *value = 1;
}
```

### local exclusive borrow can cross await

Owned locals stay uniquely owned by the suspended async frame.

```ds
declare function ready(): Promise<void>;

async function write(value: int32): Promise<void> {
    let exclusive = &exclusive value;
    await ready();
    *exclusive = 1;
}
```

### exclusive borrow can begin after await

Exclusive access can begin after resuming.

```ds
declare function ready(): Promise<void>;

async function write(value: int32): Promise<void> {
    await ready();
    let exclusive = &exclusive value;
    *exclusive = 1;
}
```

### owned borrow can cross yield

Owned locals stay alive in a suspended generator frame.

```ds
function* read(value: int32): Generator<int32, void, unknown> {
    let borrow = &readonly value;
    yield 1;
    borrow satisfies &readonly int32;
}
```

### borrow can begin after yield

Borrowed access can begin after resuming.

```ds
function* read(value: int32): Generator<int32, void, unknown> {
    yield 1;
    let borrow = &value;
    borrow satisfies &int32;
}
```

### exclusive parameter borrow can cross yield

Exclusive borrowed parameters use the same source proof rule in generators.

```ds
function* write(value: &exclusive int32): Generator<void, void, unknown> {
    yield;
    *value = 1;
}
```

### local exclusive borrow can cross yield

Owned locals stay uniquely owned by the suspended generator frame.

```ds
function* write(value: int32): Generator<void, void, unknown> {
    let exclusive = &exclusive value;
    yield;
    *exclusive = 1;
}
```
