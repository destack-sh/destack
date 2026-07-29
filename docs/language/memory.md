---
title: Memory
description: Ownership, borrowing, lifetimes, allocation, capabilities, and synchronization.
order: 23
---

# Memory

TypeScript, like many managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed on some (local) heap, and all value types are copied by default.
That is convenient and often what we want - and it remains the default in TypeScript-oriented Destack (!) -  but sometimes we want to take additional control of memory, whether for better performance, or just to express certain invariants in the code.
Destack has a broader, more explicit memory model with four dimensions of *opt-in* control:

- **Ownership**: managed `T`, (explicitly) owned `^T`, borrowed `&T`, or raw `*T`.
- **Access**: readonly, mutable, or exclusive.
- **Placement**: relative inside stored type definitions, otherwise `local` by default or explicitly `shared` across Workers.
- **Lifetime**: provenance of a borrow, such as `"static"` or `"frame"`.

Ownership and placement compose freely, e.g. `shared ^T` and `^shared T` both mean an owned value in shared space, and `shared &T` is a borrow of a shared value.
Plain old `T` still behaves as the type's default representation, exactly like we're used to from TypeScript: value types are direct values, while reference types are managed references that can be aliased and mutated freely in local space.

## Ownership

Ownership determines who keeps a value alive, who is allowed to mutate it, and when and how it is eventually freed.
The notion of ownership has many names and forms, but today it is most commonly associated with Rust's explicit ownership system, and that is also the system most similar to Destack.
Really, "ownership" just means that each _value_ has an owner and certain rules apply to how we can pass and store values to certain places, depending on which level of mutability and ownership they need.

The usual explanation of "ownership" sounds more complex than it is, especially to developers used to "managed" languages, and _especially_ because Rust tradition (deliberately) unifies "liveness", "exclusivity" and "mutability" while only liveness is required for memory safety.
Unlike Rust, Destack supports _both_ multiple mutable borrows (`&T`) and exclusive mutable borrows (`&exclusive T`):

| Form | Meaning | Mutable? | Exclusive? |
|------|---------|----------|------------|
| `T` | default value form: direct for value types, managed for reference types | yes | depends on the default form |
| `^T` | owned value form | yes | yes (single owner) |
| `&T` | borrowed access | yes | no |
| `&readonly T` | readonly borrowed access | no | no |
| `&exclusive T` | exclusive borrowed access | yes | yes |
| `*T` | raw pointer | yes (unchecked) | no (unchecked) |

The owner of a value is responsible for keeping it alive, and also for disposing of the owned value when the parent's own lifetime ends.
The parent (owner) could be in static storage for global constants, it could be another owned or even managed value, or it could be a call frame in a method (in which case we get a stack allocation).

```ds
let user: User = new User();   // managed: the runtime owns it
let owned: ^User = new User(); // owned: this binding owns it, dropped after last use

let borrow: &User = &user;                    // mutable borrow of the managed value
let view: &readonly User = &readonly user;    // readonly may overlap the mutable borrow
let claim: &exclusive User = &exclusive user; // ERROR: the earlier loans are still live
let pointer: *User = &user;                   // OK: raw pointers are inert and unchecked
```

Each ownership form also has a corresponding normalized representation in our little ["type algebra"](#algebra), which means we get to do regular TypeScript-style type space logic, conditionals and remapping (including for lifetimes!).

## Borrowing

To ensure memory safety even in unmanaged land, Destack follows the Rust idea of using ownership rules to ensure that _borrowing_ a reference `&T` remains valid - that is, `T` must remain alive (must not be deallocated) while _any_ `&T` is active.
Much more so than in Rust, Destack infers lifetimes for borrowed values even with complex control flow, so most of the time all lifetimes are inferred correctly and we don't need to think too much.

Unlike in Rust, in Destack, mutability is decoupled from borrowing: we can have multiple mutable borrows `&T` and readonly borrows `&readonly T` of the same `T` _at the same time_, as long as there is no concurrent `&exclusive T` borrow (which mirrors Rust's `&mut T`).
Importantly, this is still memory safe because all operations that may invalidate a borrow require `&exclusive T` access.

| Form | Access |
|------|--------|
| `&readonly T` | may overlap, cannot mutate through the borrow |
| `&T` | may overlap, can mutate through the borrow |
| `&exclusive T` | cannot overlap another loan of the same place in execution, can mutate through the borrow |

For local managed storage, exclusivity applies to execution through loans rather than ownership or reachability: ordinary managed aliases may still exist, but only one continuation executes between suspension points.
An exclusive borrow therefore grants exclusive borrowed access for the operation without implying unique ownership, sole reachability, or a `noalias` optimization guarantee.

```ds
struct Point {
    x: int32;
    y: int32;
}

class User {
    name: string = "";
}

let user: User = new User();

// `&user.name` borrows through the managed `User` reference
let name = &user.name;

// `name` is borrowed access into managed storage
name satisfies &string;

let point: ^Point = Point { x: 1, y: 2 };

// `&readonly point.x` borrows from owned storage without moving `point`
let readX = &readonly point.x;

// `&point.x` *can* overlap with readonly borrowed access
let writeX = &point.x;
*writeX = 3;

// `&point.y` mutably borrows a disjoint field
let writeY = &point.y;
*writeY = 3;

// the readonly borrow is still valid here
readX satisfies &readonly int32;

// `&exclusive point.x` is allowed after the overlapping borrows are no longer live
let exclusiveX = &exclusive point.x;
*exclusiveX = 4;
```

### Stability

Allowing multiple live mutable borrows is memory safe only because every operation that may _invalidate_ another live borrow requires exclusivity (via `&exclusive`).
Writing through a non-exclusive borrow is therefore allowed only when the place to be written to is **overwrite-stable**: the old value requires no destruction, the representation has one fixed layout, and every value observable during a concurrent write remains valid.

```ds
struct Frame {
    pixels: ^Buffer; // owned interior storage
}

let frame: ^Frame = Frame { pixels: Buffer.open() };

let pixels = &frame.pixels; // interior borrow into the owned buffer
let alias = &frame;         // non-exclusive borrows may overlap

*alias = Frame { pixels: Buffer.open() }; // ERROR: overwrite drops the old buffer under `pixels`
```

It should be noted that as with the rest of borrowing and ownership, _regular_ managed land needs to know about exactly zero of this because managed references alias freely and mutably.
Writes through one managed reference can at most result in _stale_ reads through another (a "borrow" into an array taken before it grew still reads the old buffer), which is just ordinary TypeScript aliasing, and not a memory safety problem in itself.

### Rooting

Borrows into managed storage stay valid via "rooting": the compiler retains and pins every managed object needed by the borrowed path until the borrow's last use.
The collector scans the hidden roots like ordinary reference locals, while pinning keeps interior addresses stable if the collector moves other objects.

```ds
class Profile {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

class User {
    profile: Profile;
}

let name = &user.profile.name;         // roots and pins the old profile
user.profile = new Profile("other");  // OK: the old profile remains valid
print(*name);                          // still reads the borrowed profile
```

The hidden slot is just a compiler temporary, and borrows of temporaries follow the same extension rules as Rust: a borrow that initializes a binding extends its temporary to the binding's lifetime, and any other temporary lives to the end of the enclosing statement.

### Definite Assignment

In general, all bindings must be definitely assigned to _something_ before they can be read / used in any way (read, moved, borrowed, returned, or implicitly dropped):

```ds
let value: Buffer;

if (enabled) {
    value = Buffer.open();
}

process(value); // ERROR: `value` is not assigned on every path
```

When "maybe" state is explicitly desired, we can represent it with a nullish union (or any other union or explicit type as needed):

```ds
let value: Buffer | undefined;

if (enabled) {
    value = Buffer.open();
}

if (value != undefined) {
    process(value);
}
```

## Lifetimes

Lifetimes are how we tie a borrow to its source, and for the most part, they are inferred and we don't need to think about them too much.
In source, lifetimes are modeled as ordinary comptime parameters (`<comptime L: Lifetime>` on `Borrowed<T, L>`), with the familiar compact tick form `'a` for the common cases:

| Source | Lifetime root |
|--------|---------------|
| static storage | `"static"` |
| owned frame storage | `"frame"` |
| local managed storage | the current synchronous region, which ends at the next suspension point |

Like `Access`, `Space`, and `Place`, `Lifetime` is a kind of static _value_, which is why memory form parameters need the `comptime` modifier.
Fortunately, the tick spelling for lifetimes (e.g. `'a`) is much nicer and almost always sufficient, as `<'a>` declares the comptime lifetime parameter `'a`, `&'a T` names a borrow's lifetime, and `View<'a>` binds it as an ordinary generic argument.

```ds
function read(user: &User): &string {
    return &user.name;
}

function first<T>(items: &[T]): &T {
    return &items[0];
}

function pick<'c>(a: &'c Node, b: &'c Node): &'c Node {
    return a;
}
```

Lifetime elision - whether you need to write a lifetime parameter explicitly or not - follows the same rules as Rust:
- **Type declarations**: structs, classes, enums, and type aliases need their lifetime parameters explicitly.
- **Function signatures**: including written function types elide freely: elided borrows complete the signature with hidden generic lifetime parameters.
(We tried inducing lifetimes implicitly, but that turned out ergonomically weird and a nightmare to implement correctly.)

```ds
// stored borrows write their lifetimes
struct WorldView<'e, 'a> {
    engine: &'e Engine;
    assets: &'a AssetStore;
}

// signatures elide, completing hidden lifetime parameters
function inspect(engine: &Engine): &string {
    return &engine.name;
}
```

Like in Rust, a signature always means exactly what it says, and nothing is inferred into the lifetimes from the function body.
(We tried that too, and it caused all sorts of inference complexities and weird action at a distance behavior.)
The elision rules are straightforward and a little more general than in Rust:
 - With a borrowed receiver, the result takes the receiver's lifetime.
 - With other borrowed inputs, the result takes the union of the input lifetimes.
 - With no borrowed inputs at all, the result is `"static"`.

```ds
// elided form
function choose(a: &Node, b: &Node, flag: boolean): &Node {
    return flag ? a : b;
}

// explicit form
function choose<'a, 'b>(
    a: &'a Node,
    b: &'b Node,
    flag: boolean,
): Borrowed<Node, 'a | 'b> {
    return flag ? a : b;
}
```

## Suspension

As discussed in [Continuations](./expressions.md#continuations), values and objects in "managed land" work exactly like in regular TypeScript and can be referenced and mutated freely wherever.
When _borrowing_ across suspension points (like `yield` or `await`), we must still prove that the core safety rules are upheld, i.e. that the borrow source remains alive.
And because multiple continuations may be active concurrently (even if not in parallel), a borrow acquired from local managed storage is rooted in the current synchronous region, and that region ends at the next suspension point.

```ds
// owned by the frame: the borrow may cross suspension
async function readOwned(user: ^User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}

// borrowed parameter: also fine, the caller proves the source is owned or static
async function readBorrowed(user: &User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}

// managed: rejected, another continuation could touch `user` while we are parked
async function readManaged(user: User): Promise<string> {
    const name = &readonly user.name;
    await tick(); // ERROR: the region rooting `name` ends at this suspension
    return name.clone();
}
```

It should be noted that this works precisely because suspension points are always lexically explicit (`await` and `yield`, including in function signatures).
So, in this case, having "colored functions" is a nice ergonomic tradeoff.

## Drop

Whenever the lifetime of a value ends and it is deallocated, Destack supports running a `Drop` finalizer, similar to Rust's `Drop`.
This happens when the compiler inserts a drop for an owned local after its last use, when an owned field is being destroyed, and when the runtime _eventually_ reclaims an unreachable managed allocation.
Drop sites are statically known: maybe-present state must be represented as an explicit type like `T | undefined`, and a place conditionally moved on one branch is an error at the join, so there are no runtime drop flags as such.

```ds
function run(): void {
    let buffer: ^Buffer = Buffer.open();
    process(&buffer);
    // `buffer` is dropped immediately after its last use
}
```

The same `Drop` also works for managed references, where it is run before they are freed:

```ds
class Image {
    pixels: Unique<[uint8]>;
}

let image: Image = new Image();
// when `image` is collected, the GC runs `Drop` for `pixels`
```

Unlike Rust and C++ RAII, Destack models `Drop` and `Dispose` / `AsyncDispose` separately: `Drop` follows lifetime, while [`using`](./expressions.md#using) is lexical.
Because of this split, `Drop` _can_ run eagerly after last use, which is great for memory pressure, but also means Destack's `Drop` is not the right fit for lexical RAII-style cleanup.
In general, `Drop` should manage memory and memory-shaped cleanup, while `using` should manage richer resource finalization:

| Protocol | Purpose | Timing |
|----------|---------|--------|
| `Drop` | ownership finalization for memory and owned fields | deterministic for owned values, GC-timed for managed values |
| `Dispose` | explicit synchronous resource cleanup | lexical `using` scope exit |
| `AsyncDispose` | explicit asynchronous resource cleanup | lexical `await using` scope exit |

External resources such as files, locks, sockets, transactions, and temporary runtime registrations should use `Dispose` or `AsyncDispose`, not `Drop`.
This keeps RAII-style cleanup explicit and predictable even though ordinary TS++ values are "managed" by default:

```ds
using file = File.open(path)?;
await using connection = await pool.connect();
```

Low-level code can of course still control finalization explicitly:
`drop(value)` ends ownership immediately, `forget(value)` intentionally suppresses automatic drop, `ManuallyDrop<T>` stores a value outside automatic drop handling, and `Box<T>.leak()` turns one owned allocation into a static borrow.

## Allocation

The primary way to construct new values is `new`, which initializes a `T` and produces the ownership form required by the _destination_ type - `new` is really just an initializer that calls the type's constructor.
The required destination type decides whether that is managed storage, owned storage, inline frame storage, shared storage, or some lower-level allocation form.

```ds
let a: User = new User();   // managed
let b: ^User = new User();  // owned
```

In some situations it is useful to handle allocation errors directly, and for that Destack supports fallible allocation accessors both via higher level `try*` methods in standard library types (like `Array.tryReserve`) and the low level intrinsics (`MaybeUninit<T>`, `Unique<T>`, etc.) that they are built on.

```ds
try {
    let buffer: Block[] = Array.tryWithCapacity(count)?; // fallible library allocation
    let page = new Page(buffer);
    page.fill()?;

    return page;
} catch (e: AllocationError) {
    return null;
}
```

Code can also opt out of managed allocation locally via [restrictions](./expressions.md#restrictions):

```ds
@noManaged
function processFrame(input: &[Sample]): ^Frame {
    return buildFrame(input);
}

@noHeap
function interruptHandler(input: &[Sample]): Frame {
    return buildFrameOnStack(input);
}
```

## Space

The space of a value determines where it is actually located in memory, and since Destack follows web and TypeScript conventions, the `Worker`-_local_ heap is the default main memory space.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space, and user and library code can almost always just pretend spaces don't even exist.
The actual placement of a type is determined relative to the parent until we find an explicit qualifier or reach the module (which is local by default):

```ds
struct Request<T> {
    header: Header;
    body: T;
}

let localRequest: Request<Body>;                 // relative Body resolves to local
let sharedRequest: shared Request<Body>;         // relative Body resolves to shared
let mixedRequest: local Request<shared Body>;    // explicit Body remains shared
```

A value's "place" is where its bytes are actually stored, _not_ the binding that points to it (copying a managed reference preserves the place of its referent).
Values never move between spaces, so a `shared` value cannot be passed where a `local` value is expected, and vice versa.
Local Workers cannot see into other Workers, so a value also cannot move from one Worker's `local` space into another's.

```ds
function replace(user: User): void {
    consumeExclusive(user); // OK: `User` means `local User` here
}

declare const sharedUser: shared User;
replace(sharedUser); // ERROR: the parameter is local
```

Instance members instead interpret relative types in the receiver's space, and the destination of `new` selects the constructor receiver's space contextually as well:

```ds
class Box {
    user: User;

    constructor(user: User) {
        this.user = user;
    }
}

const localBox: local Box = new Box(localUser);   // Box.user is a local User here
const sharedBox: shared Box = new Box(sharedUser); // and a shared User here
```

To work with values from more than one space, we can use the ordinary comptime `Space` parameter:

```ds
function inspectIn<comptime S: Space>(
    value: Placed<&readonly User, S>,
): void;

inspectIn(localUser);  // S = "local"
inspectIn(sharedUser); // S = "shared"
```

An unconstrained type parameter may include explicit placement, so its body must be valid for every placement admitted by its bounds.
An operation that only some placements grant therefore moves into the signature, where each caller supplies it from concrete storage:

```ds
function reset<T extends Resettable>(item: T): void {
    item.reset(); // ERROR: reset() needs &exclusive this, unprovable for every T
}

function reset<T extends Resettable>(item: &exclusive T): void {
    item.reset(); // OK: the caller acquires where placement is concrete
}

reset(&exclusive users[0]); // local managed at the call site grants exclusivity
```

Finally, a declaration may also choose an intrinsic placement for itself, which forces every use into that space:

```ds
local class Promise<T> {}
shared class Channel<T> {}
local newtype TaskId = uint64;

PlaceOf<Promise<void>> satisfies "local";
PlaceOf<Channel<string>> satisfies "shared";
PlaceOf<TaskId> satisfies "local";
```

### Shared Space

In TypeScript tradition, each "Worker" (in the JS spec generally referred to as "Agent") has its own isolated local heap that cannot be touched by other workers.
For sharing memory across workers, TypeScript has the `SharedArrayBuffer` concept and plain old message passing with `postMessage`.
That works, but is architecturally limited and makes it complex to implement more sophisticated parallelism patterns.

Destack supports an explicit, separate, fully featured _shared_ memory space that is visible to all `Worker`s in the same `Runtime` via regular references and objects.
Basically, `shared T` is the typed, generalized version of the `SharedArrayBuffer` idea with the full type system and object graphs at our disposal with the simple rule that local may point into shared, but shared must not point into local memory.
Shared placement permits concurrent access but does not itself provide synchronization; code uses atomics, locks, or higher-level protocols where ordering is required.

### Shared Module Bindings

Because Destack inherits the JS/TS Worker model for isolation, module-scoped constants are owned by each _Worker_ and are not actually process-global as they would be in most other languages.
For genuinely _shared_ process-global state, the binding _itself_ can be declared as `shared`.

| Form | Binding place | Value place | Meaning |
|------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared reference |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit for both binding and value |

Note that marking the binding itself as `shared` also types the value as `shared` (as it is illegal to point from shared storage into local storage anyway, this is convenient).
Shared bindings are always `const`, and `const` means exactly what it means in TypeScript: the binding is not reassignable, while the value behind it mutates freely through its own API.
Like every value stored in shared space, a shared binding's value must satisfy `SharedSafe`.

```ds
shared const world: Mutex<World> = new Mutex(new World());

const view = world.lock(); // guard dereferences to &exclusive World
```

Because module code runs on every Worker, shared bindings are initialized exactly once by the runtime, before any other Worker can observe them.

## Conversions

Reference conversions follow directly from [the five memory rules](#memory), and borrowing from a live place works whenever the requested loan rules hold:

| Source | Borrow | Notes |
| --- | --- | --- |
| local managed `T` | `&T` / `&readonly T` / `&exclusive T` | exclusive needs no overlapping loan; none may cross [suspension](#suspension) |
| owned `^T` | `&T` / `&readonly T` / `&exclusive T` | owned sources may also cross suspension |
| shared owned `shared ^T` | `shared &T` / `shared &readonly T` / `shared &exclusive T` | owned storage stays unique even in shared space because it still has one owner |
| shared managed `shared T` | `shared &T` / `shared &readonly T` | non-exclusive writes remain limited to overwrite-stable places; synchronization remains separate |
| shared managed `shared T` | `shared &exclusive T` | never: independent Workers prevent exclusive execution |

When a managed or owned value appears where a borrow is required, the compiler inserts a borrow coercion whose lifetime and placement are inferred from the source and use:

```ds
declare function inspect<comptime S: Space>(
    value: Placed<&readonly User, S>,
): void;
declare function modify<comptime S: Space>(
    value: Placed<&User, S>,
): void;
declare function replace<comptime S: Space>(
    value: Placed<&exclusive User, S>,
): void;

declare const localUser: local User;
declare const sharedUser: shared User;

inspect(localUser);  // implicit `local User` to `local &readonly User`
inspect(sharedUser); // implicit `shared User` to `shared &readonly User`
modify(localUser);   // implicit `local User` to `local &User`
modify(sharedUser);  // implicit `shared User` to `shared &User`
replace(localUser);  // implicit `local User` to `local &exclusive User`
replace(sharedUser); // ERROR: shared managed storage cannot grant exclusive access
```

The explicit `S` is what lets these declarations accept both local and shared borrows.
A declaration written only as `inspect(value: &readonly User)` accepts a local borrow, like any other bare free-function parameter.
Borrows can "weaken" (downgrade into a more restrictive form) but cannot be upgraded:

| From | To | Notes |
| --- | --- | --- |
| `&exclusive T` | `&T` / `&readonly T` | temporary reborrow that suspends the exclusive loan |
| `&T` | `&readonly T` | readonly reborrow |
| managed reference `T` | `^T` | never: managed ownership cannot become unique ownership |
| `&T` | `T` / `^T` | never: borrowed access does not own the value |

Safe code may convert a borrow into a raw pointer, while converting back requires an unsafe context:

```ds
let user = new User();

let borrow: &User = &user;   // default: a checked borrow
let pointer: *User = &user;  // typed as raw: an inert, unchecked pointer value
let again: &User = pointer;  // ERROR: pointers only reborrow inside @unsafe
```

## Unsafe

Safe Destack code can create and carry raw pointers, because there is nothing directly unsafe about just looking at pointers.
Raw pointers are inert: they do not keep storage alive, do not participate in borrow checking, and do not prove exclusivity.

Converting a borrow to a raw pointer is still safe because it does not touch the pointed-to memory:

```ds
let user = new User();

let borrow: &User = &user;
let pointer: *User = borrow; // OK: this only creates a raw pointer value
```

Unsafe begins when code relies on a memory invariant the compiler cannot prove:

| Operation | Example | Safe? | Why |
|-----------|---------|-------|-----|
| create or carry raw pointer values | `let pointer: *User = &user`, `pointer == other` | yes | does not touch memory |
| reinterpret raw pointer values | `pointer as *uint8`, `0x1000 as *uint8` | yes | makes no validity claim |
| wrapping address arithmetic | `wrappingOffset(pointer, 4)` | yes | makes no allocation claim |
| allocation-relative pointer math | `offset(pointer, 4)`, `offsetFrom(pointer, origin)` | no | claims same live allocation |
| access memory through a pointer | `asReference(pointer)`, `read(pointer)`, `write(pointer, value)` | no | bypasses borrow checking |
| build typed views from raw storage | `Slice.fromRaw(pointer, length)` | no | claims a valid region of `T` |
| raw bytes and layout tricks | `copyBytes(dst, src, n)`, `readVolatile(pointer)`, `transmute<T, U>(value)` | no | touches or reinterprets unchecked memory |

The compiler rejects unsafe operations, like raw pointer dereferencing, outside explicit [`@unsafe` / `@safe`](./expressions.md#taint) contexts.

## Algebra

Destack's "memory algebra" is a fancy way of saying that ownership, access, lifetime, and placement are just types that we can use with TypeScript-style algebra and inference.
We can inspect a type's memory form and build a derived form because qualified types like `readonly T`, `^T`, `&T`, `*T`, `local T`, and `shared T` correspond to builtin intrinsic types and rewrite helpers:

```ds
/// Automatically managed T, owned by the runtime.
newtype Managed<T> = intrinsic;
/// Owned T (`^T`).
newtype Owned<T> = intrinsic;
/// Borrowed T (`&T`).
newtype Borrowed<T, comptime L: Lifetime, comptime A: Access = "mutable"> = intrinsic;
/// Raw T (`*T`).
newtype Raw<T> = intrinsic;
/// Placed T (`local T` or `shared T`).
newtype Placed<T, comptime P: Place> = intrinsic;
```

Specifically, all memory sigils and keywords are compact syntax for intrinsic forms that normalize through the same algebra:

```ds
declare class User { /* ... */ };
declare struct Point { /* ... */ };

type NormalUser = User;                        // unqualified, normal default representation
type NormalPoint = Point;                      // unqualified value type, default-owned/direct
type ReadonlyUser = readonly User;             // WithAccess<User, "readonly">
type OwnedUser = ^User;                        // Owned<User>
type OwnedPoint = ^Point;                      // Owned<Point>, reduced to Point
type ReadonlyBorrowedUser = &readonly User;    // Borrowed<User, L, "readonly">
type BorrowedUser = &User;                     // Borrowed<User, L, "mutable">
type ExclusiveBorrowedUser = &exclusive User;  // Borrowed<User, L, "exclusive">
type RawUser = *User;                          // Raw<User>
type LocalUser = local User;                   // Placed<User, "local">
type SharedUser = shared User;                 // Placed<User, "shared">
type LocalOwnedUser = local ^User;             // Placed<Owned<User>, "local">
type SharedOwnedUser = shared ^User;           // Placed<Owned<User>, "shared">
type OwnedSharedUser = ^shared User;           // Placed<Owned<User>, "shared">
```

It follows that because forms compose, owning a borrow is different from borrowing an owner:

```ds
Owned<Borrowed<User, L>> // owns a borrow value
Borrowed<Owned<User>, L> // borrows an owned value
```

Destack also provides builtin accessors to inspect composed forms, e.g., `PayloadOf<T>` removes one outer form, while `BaseOf<T>` removes all transparent memory forms:

```ds
BaseOf<shared ^User> satisfies User;
PayloadOf<Owned<Borrowed<User, L>>> satisfies Borrowed<User, L>;
OwnershipOf<^User> satisfies "owned";
OwnershipOf<Point> satisfies "owned";
OwnershipOf<User> satisfies "managed";
OwnershipOf<Owned<Borrowed<User, L>>> satisfies "owned";
OwnershipOf<Borrowed<Owned<User>, L>> satisfies "borrowed";
OwnershipOr<User, "managed"> satisfies "managed";
AccessOf<User> satisfies "mutable";
AccessOf<readonly User> satisfies "readonly";
AccessOf<^readonly User> satisfies "readonly";
AccessOf<&exclusive User> satisfies "exclusive";
LifetimeOf<Managed<User>> satisfies never;
LifetimeOr<Managed<User>, "static"> satisfies "static";
```

Only borrowed forms carry a lifetime in their type.
The lifetime of a borrow acquired from managed or owned storage comes from the concrete value place, so querying a managed type alone yields `never` rather than inventing a frame provenance.

`Space` represents one concrete storage space while `Place` means either a concrete `Space` or `"relative"`.
Relative placement is resolved by a containing receiver and is local at a free declaration:

```ds
PlaceOf<User> satisfies "relative";
SpaceOf<^User> satisfies never;
PlaceIn<^User, "shared"> satisfies "shared";

PlaceOf<local User> satisfies "local";
SpaceOf<local User> satisfies "local";
PlaceIn<local User, "shared"> satisfies "local";

PlaceOf<shared User> satisfies "shared";
SpaceOf<shared User> satisfies "shared";
PlaceIn<shared User, "local"> satisfies "shared";

PlaceOf<User | local User> satisfies "relative";
PlaceOf<local (User | Team)> satisfies "local";
SpaceOf<local (User | Team)> satisfies "local";
```

Predicates with `Is*` are convenience wrappers around those same accessors:

```ds
IsOwned<^User> satisfies true;
IsBorrowed<&User> satisfies true;
IsShared<shared User> satisfies true;
IsShared<^User> satisfies false;
IsSharedIn<^User, "shared"> satisfies true;
```

Rewriting helpers preserve the rest of the type instead of rebuilding from a stripped base type:

```ds
WithSpace<^User, "shared"> satisfies Placed<^User, "shared">;
WithSpace<shared ^User, "local"> satisfies shared ^User;
WithOwnership<shared User, "owned"> satisfies Owned<shared User>;
WithPlace<shared User, "relative"> satisfies Placed<User, "relative">;
WithAccess<User, "readonly"> satisfies readonly User;
WithAccess<^User, "readonly"> satisfies ^readonly User;
WithAccess<&User, "exclusive"> satisfies &exclusive User;
```

Inside a type declaration, `this` in type or static position also carries the current instantiated form of that type to query against with the `*Of` and `Is*` family.

```ds
struct Buffer<T> {
    lock: PlaceOf<this> == "shared" ? Mutex : ();
    value: T;
}

declare const localBuffer: Buffer<string>;
declare const sharedBuffer: shared Buffer<string>;

sharedBuffer.lock satisfies Mutex;
localBuffer.lock satisfies ();
PlaceOf<typeof localBuffer> satisfies "local";
PlaceOf<typeof sharedBuffer> satisfies "shared";
```

## Polymorphism

Since ownership, access, lifetime, and placement are all exposed as ordinary comptime values, contracts and implementors can be polymorphic and conditional over ownership, space, and access.
The caller chooses the level of control through the expression and its type; owned arguments move in by position without a prefix operator:

```ds
declare const user: User;
declare const owned: ^User;

process(user);            // managed/default value
process(&readonly user);  // readonly borrowed access
process(&user);           // mutable borrowed access
process(&exclusive user); // exclusive borrowed access
process(owned);           // owned value, moved into the call
```

Dispatch resolution uses the actual form during overload resolution, so container interfaces like `Iterable<T>` can support ordinary TypeScript iteration and borrowed iteration without adding Rust-style method family explosion:

```ds
declare type Point = { x: number; y: number };
declare const points: Array<Point>;
declare const ownedPoints: ^Array<Point>;

// ordinary "managed" iteration
for (const point of points) {
    point satisfies Point;
}

// readonly borrowed iteration
for (const point of &readonly points) {
    point satisfies &readonly Point;
}

// mutable borrowed iteration
for (const point of &points) {
    point satisfies &Point;
}

// exclusive borrowed iteration
for (const point of &exclusive points) {
    point satisfies &exclusive Point;
}

// consuming iteration: an owned collection moves into its iterator by position
for (const point of ownedPoints) {
    point satisfies Point;
}
```

## Capabilities

Destack encodes memory capabilities as trait-like interfaces, usable as ordinary bounds:

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |
| `SharedSafe` | Values of the type may be stored in [shared space](#shared-space). |
| `OverwriteStable` | Place can be [overwritten](#stability) through a non-exclusive mutable access. |
| `DynamicSafe` | Type can be erased behind a [`dynamic`](./types.md#representation) carrier. |
| `Concrete` | Type has one complete storage representation. |
| `Zeroable` | Type is valid when all bytes are zero. |
| `Unpin` | Value may move out of pinned storage. |
| `Default` | Type has a conventional `default()` value. |

The compiler derives representation markers such as `Copy`, `SharedSafe`, `OverwriteStable`, and `DynamicSafe` structurally.
Member-bearing capabilities such as `Clone` and `Default` use ordinary extension implementations.

## Synchronization

The standard library provides the usual memory and synchronization primitives on top of this unified memory system.
The full details are documented in the library, but the basics should be familiar to anyone with a systems-level background.
This is also where the `@unsafe` actually lives: a `Mutex<T>` mutates through a non-exclusive receiver via `UnsafeCell` and atomics, and its guard manufactures the `&exclusive T` that the runtime lock (not the borrow checker) guarantees.

| Primitive | Behavior |
|-----------|----------|
| `Box<T>` | unique heap ownership for `Owned<T>`, with deterministic drop when `T: Drop` |
| `Rc<T>` | local shared ownership of `Owned<T>`, non-atomic refcount, not transferable across Workers |
| `Arc<T>` | shared ownership of `Owned<T>`, atomic refcount, transferable as `shared Arc<T>` of `SharedSafe` `T` |
| `Cell<T>` | local interior mutation by value, for small `Copy`-like state |
| `RefCell<T>` | local runtime borrow checking for cases static borrowing cannot express cleanly |
| `Atomic<T>` | lock-free scalar storage with explicit ordering and scope |
| `AsyncMutex<T>` | local mutual exclusion that suspends the current async task, not the Worker |
| `Mutex<T>` | shared mutual exclusion backed by atomics and runtime wait/wake support |

Because ownership and placement are part of our type system, and we can query and gate based on contextual type information using regular TypeScript algebra, we have a lot of flexibility and gain some nice ergonomics.
For example, we can provide ergonomic context-aware aliases that conform to the way they are used:

```ds
type Ref<T> =
    PlaceIn<T, "local"> extends "shared" ? Arc<T> : Rc<T>;

type Lock<T> =
    PlaceIn<T, "local"> extends "shared" ? Mutex<T> : AsyncMutex<T>;
```
