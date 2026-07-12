---
title: Memory
description: Ownership, borrowing, lifetimes, allocation, capabilities, and synchronization.
order: 23
---

# Memory

TypeScript, like many managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed on some (local) heap, and all value types are copied by default.
That is convenient and often what we want, but sometimes we need to take direct control of memory, whether for better performance, or just to express certain invariants in the code.

Destack supports explicit, optional type modifiers for controlling memory _placement_ and _ownership_:
- **Placement** - where the value is located: ambient by default, explicitly `local` to one Worker, `shared` across Workers (or `static` for constant and `frame` for activation frames).
- **Ownership** - who "owns" the value: managed (`T`), owned (`^T`), borrowed (`&T`, `&readonly T`, `&exclusive T`), or raw (`*T`).

The two axes of ownership and placement compose and commute freely, e.g. `shared ^T` and `^shared T` both mean an owned handle to a value in shared space, and `shared &T` is a borrow of a shared value.
Once structural shapes and constraints have been [reified, instantiated, or erased](./types.md#representation), the plain old `T` still behaves as the type's default representation, exactly like we're used to from TypeScript: value types are values, object types are managed references, both can be aliased and mutated freely (locally).

Everything in this chapter follows from four rules:

1. Every value has exactly one owner - for managed values, the owner is the runtime.
2. A borrow must remain valid: a place cannot move or drop while an overlapping borrow is live, and a borrow cannot outlive the source it came from.
3. Exclusive borrows are truly exclusive: no overlapping borrow of the same place may be live.
4. Shared memory must never point into local memory.

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
let view: &readonly User = &readonly user;    // OK: readonly may overlap the mutable borrow
let claim: &exclusive User = &exclusive user; // ERROR: `borrow` and `view` are still live
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

Borrow checking is just rule 2 applied per "access path", so disjoint fields can be borrowed independently when the compiler can prove they do not overlap.

```ds
struct Point {
    x: int32;
    y: int32;
}

class User {
    name: string = "";
}

let user: User = new User();

// `&user.name` borrows through the managed `User` handle
let name = &user.name;

// `name` is borrowed access into managed storage
name satisfies &string;

let point = ^Point { x: 1, y: 2 };

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
Writing through a non-exclusive borrow is therefore allowed only when that place is **overwrite-stable**: the old value requires no destruction (no `Drop`), and the new bytes mean what the old bytes meant (one fixed layout, no variant tag), both of which scalar and managed-reference fields trivially satisfy.
Everything else requires `&exclusive`: overwriting a variant reinterprets the payload under a live interior borrow, overwriting an owning value frees memory a borrow may still target, and so on.

```ds
struct Frame {
    pixels: ^Buffer; // owned interior storage
}

let frame: ^Frame = Frame { pixels: Buffer.open() };

let pixels = &frame.pixels; // interior borrow into the owned buffer
let alias = &frame;         // non-exclusive borrows may overlap

*alias = Frame { pixels: Buffer.open() }; // ERROR: overwrite drops the old buffer under `pixels`
```

It should be noted that as with the rest of borrowing and ownership, _regular_ managed land needs to know about exactly zero of this because managed handles alias freely and mutably.
Writes through one managed handle can at most result in _stale_ reads through another (a "borrow" into an array taken before it grew still reads the old buffer), which is just ordinary TypeScript aliasing, and not really a memory safety problem in itself.

### Rooting

Borrows into managed storage stay valid via something we call "automatic rooting": the compiler remembers every managed handle a borrow needs via a hidden frame slot that keeps it alive until the borrow's last use (and the collector scans that slot like any other handle local).
This is essentially what we always want anyway, and it lets the GC not worry about interior borrows _at all_: no write can invalidate a borrow rooted in managed storage since overwriting the path the borrow came through leaves the old object alive.

```ds
class Profile {
    name: string;
}

class User {
    profile: Profile;
}

let name = &user.profile.name;          // roots the `user.profile` handle
user.profile = new Profile("other");    // OK: the old profile stays pinned
print(*name);                           // still reads the borrowed profile
```

The hidden slot is just a compiler temporary, and borrows of temporaries follow the same extension rules as Rust: a borrow that initializes a binding extends its temporary to the binding's lifetime, and any other temporary lives to the end of the enclosing statement.

### Definite Assignment

In general, all binding must be definitely assigned to _something_ before they can be used in any way (read, moved, borrowed, returned, or implicitly dropped):

```ds
let value: Buffer;

if (enabled) {
    value = Buffer.open();
}

process(value); // ERROR: `value` is not assigned on every path
```

When "maybe" state is explicitly desired, we can represent it with a nullish union (or any other union as needed)_

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

Lifetimes tie a borrow to its source, and in Destack they are "just" comptime parameters: `<comptime L: Lifetime>` parameters on `Borrowed<T, L>`, available to all the regular TypeScript-style type algebra and inference (including flow typing and narrowing).
Like `Access`, `Space`, and `Place`, `Lifetime` is a kind of static _value_ rather than a type, which is why parameters over these forms always carry the `comptime` modifier.
In practice they are spelled out in exactly one place - `declare` signatures - and inferred everywhere else, including from function bodies.

```ds
function read(user: &User): &string {
    return &user.name;
}

function first<T>(items: &[T]): &T {
    return &items[0];
}
```

Elided borrowed forms complete the owning declaration with hidden generic lifetime parameters in _all_ declarations, not just in function signatures.
Unlike structural constraints, this does not select a hidden concrete representation.

```ds
// elided form
struct WorldView {
    engine: &Engine;
    assets: &AssetStore;
}

// explicit form
struct WorldView<comptime L1: Lifetime, comptime L2: Lifetime> {
    engine: Borrowed<Engine, L1>;
    assets: Borrowed<AssetStore, L2>;
}
```

Because lifetimes are part of type inference, and type inference also analyzes method bodies, lifetime inference also derives from function bodies.
Each elided borrow in the signature gets its own hidden lifetime parameter first, and the body then solves the return lifetime:

```ds
// elided form
function first(a: &Node, b: &Node): &Node {
    return a;
}

// explicit form
function first<comptime L1: Lifetime, comptime L2: Lifetime>(
    a: Borrowed<Node, L1>,
    b: Borrowed<Node, L2>,
): Borrowed<Node, L1> {
    return a;
}

// elided form
function choose(a: &Node, b: &Node, flag: boolean): &Node {
    return flag ? a : b;
}

// explicit form
function choose<comptime L1: Lifetime, comptime L2: Lifetime>(
    a: Borrowed<Node, L1>,
    b: Borrowed<Node, L2>,
    flag: boolean,
): Borrowed<Node, L1 | L2> {
    return flag ? a : b;
}
```

Because `declare` functions do not have bodies, it follows that declaration-only APIs must spell out lifetime relationships explicitly.

```ds
// rejected
declare function only(value: &Node): &Node;

// accepted
declare function only<comptime L: Lifetime>(value: Borrowed<Node, L>): Borrowed<Node, L>;

// rejected
declare function choose(
    a: &Node,
    b: &Node,
): &Node;

// accepted
declare function choose<comptime L1: Lifetime, comptime L2: Lifetime>(
    a: Borrowed<Node, L1>,
    b: Borrowed<Node, L2>,
): Borrowed<Node, L1 | L2>;
```

Taken together, the mechanisms of elision and inference for lifetimes let us implement the vast majority of low level ownership patterns _without_ having to specify lifetimes to the compiler explicitly (yay).
It should also be noted that one consequence of this body-derived inference is that a body change can change an inferred public signature, but that is just the tradeoff here.

## Suspension

As discussed in [Continuations](./expressions.md#continuations), values and objects in "managed land" work exactly like in regular TypeScript and can be referenced and mutated freely wherever.
When _borrowing_ across suspension points (like `yield` or `await`), the compiler must still guarantee that the core safety rules are upheld.
And because multiple continuations may be active concurrently (even if not in parallel), borrows that must be owned by the current frame to cross suspension points - in other words, borrows coming from managed values must _not_ cross suspension points.

It follows that the same body is fine or rejected purely based on where the borrowed value lives:

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
    await tick(); // ERROR: `name` borrows managed storage across suspension
    return name.clone();
}
```

One load-bearing guarantee underneath all of this: suspension points are always lexically explicit (`await` and `yield`).
There is no hidden suspension - no implicit awaits, no preemption points, no suspending allocators - so "does this borrow cross suspension" can always be answered by looking at the code (and declaration-only APIs need no extra annotations, since asyncness is visible in signatures and `declare` signatures already spell out lifetimes).

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
// when `image` is collected, the GC runs drop glue for `pixels`
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

In some situations it is useful to handle allocation errors directly, and for that Destack supports more direct fallible allocation accessors both via higher level `try*` methods in standard library types (like `Array.tryReserve`) and the low level intrinsics (`MaybeUninit<T>`, `Unique<T>`, etc.) that they are built on.
Regular managed object allocation (`new T()`) can also be made fallible via the `new?` operator, which behaves similar in spirit to the regular [`?`](./expressions.md#maybe-must-and-coalesce) and `await?` operators, propagating an `AllocationError` to the containing context:

```ds
try {
    let buffer: Block[] = Array.tryWithCapacity(count)?; // fallible library allocation
    let page = new? Page(buffer);                        // fallible managed allocation
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

```ds
struct Request<T> {
    header: Header;
    body: T;
}

let localRequest: Request<Body>;          // ambient, default -> Request is local heap
let sharedRequest: shared Request<Body>;  // explicit, shared -> Request is shared heap
```

In general, type placement is contextual: all types are "ambient" by default, i.e., they come with no inherent placement, and types are only placed wherever their parent is placed until someone either specifies placement explicitly (e.g., `local T`, `shared T`) or we reach the module top (whcih is "local").
Specific nominal declarations may also choose an intrinsic placement for themselves, which then forces them into a specific space:

```ds
local newtype interface Awaitable<T> { await(): T }
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

It's important to note that _by itself_ shared placement, like any space placement, is **not** a synchronization primitive and does **not** imply atomic access, locking, `Sync`, or anything like that.
That is by design; it's up to userland libraries to require [capabilities](#capabilities) like `Send` and `Sync` for APIs that transfer or publish values for correctness, but `shared` itself is really only about placement.

### Static Space

Because Destack inherits the JS/TS Worker model for isolation, module-scoped constants are owned by each _Worker_ and are not actually process-global as they would be in most other languages.
For genuinely _shared_ process-global state, the binding _itself_ can be declared as `shared`.

| Form | Binding place | Value place | Meaning |
|------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

Note that marking the binding itself as `shared` also types the value as `shared` (as it is illegal to point from shared storage into local storage anyway, this is convenient).
Shared bindings are always `const`, and `const` means exactly what it means in TypeScript: the binding is not reassignable, while the value behind it mutates freely through its own API.

One constraint follows directly from the Worker model: a `shared` binding is reachable from _every_ Worker by construction, so it must meet the same bar as any value crossing Workers - the value type must be `Sync` (see [Capabilities](#capabilities)).
In particular, a bare owned global like `shared const world: ^World` is rejected: every Worker can reach the binding, so its exclusivity cannot be checked.
Global mutable state instead puts a lock in the static and lets the guard be the runtime exclusivity oracle:

```ds
shared const world: Mutex<World> = new Mutex(new World());

const view = world.lock(); // guard dereferences to &exclusive World
```

Because module code runs on every Worker, shared bindings are initialized exactly once by the runtime, before any other Worker can observe them.

## Conversions

Reference conversions follow directly from [the four memory rules](#memory), and borrowing from a live place works whenever the requested loan rules hold:

| Source | Borrow | Notes |
| --- | --- | --- |
| local managed `T` | `&T` / `&readonly T` / `&exclusive T` | exclusive needs no overlapping loan; none may cross [suspension](#suspension) |
| owned `^T` | `&T` / `&readonly T` / `&exclusive T` | owned sources may also cross suspension |
| shared owned `shared ^T` | `shared &T` / `shared &readonly T` / `shared &exclusive T` | owned storage stays unique even in shared space, since the handle itself still has one holder |
| shared managed `T` | `&T` / `&readonly T` | mutation routes through `Sync` APIs and atomic field access (see [Synchronization](#synchronization)) |
| shared managed `T` | `&exclusive T` | never: independent Workers prevent exclusive execution |

When a local managed value appears where a borrow is required, the compiler inserts a borrow coercion whose lifetime is inferred from that use:

```ds
declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;
declare function replace(value: &exclusive User): void;

inspect(user); // implicit `User` to `&readonly User`
modify(user);  // implicit `User` to `&User`
replace(user); // implicit `User` to `&exclusive User`
```

This rule applies to normalized memory forms rather than particular source spellings: a source that reduces to local managed storage may coerce to a compatible borrowed view wherever those forms are nested.
The coercion changes the runtime carrier from a managed handle to a borrowed address and retains the managed root for the inferred loan.
It is inserted only when an expected type requires a borrowed form, so `const same = user` still infers the ordinary managed `User` type.
Readonly managed views only coerce to readonly borrows, and no managed-derived borrow may cross a suspension point.

Borrows can weaken freely, but cannot be upgraded (obviously):

| From | To | Notes |
| --- | --- | --- |
| `&exclusive T` | `&T` / `&readonly T` | temporary reborrow that suspends the exclusive loan |
| `&T` | `&readonly T` | readonly reborrow |
| managed reference `T` | `^T` | never: managed ownership cannot become unique ownership |
| `&T` | `T` / `^T` | never: borrowed access does not own the value |

Raw pointers convert freely in, and only unsafely out:

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

Destack's "memory algebra" is a fancy way of saying that the axes of ownership, access, lifetime, and placement are just types that we can do TypeScript-style algebra and inference with.
We can inspect a type's memory form and build a derived form because all the qualified surface forms like `readonly T`, `^T`, `&T`, `*T`, `local T`, and `shared T` correspond to builtin intrinsic types and rewrite helpers:

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
newtype Placed<T, P: Place> = intrinsic;
```

Specifically, all surface sigils and keywords are just compact syntax for those intrinsic forms that commute the way they read:

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
type OwnedSharedUser = ^shared User;           // Owned<Placed<User, "shared">>
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
```

`Space` represents one builtin concrete space while `Place` means either a concrete `Space` or `"ambient"`, and ambient placement follows the containing context until a final layout is required:
```ds
PlaceOf<User> satisfies "ambient";
SpaceOf<^User> satisfies never;
PlaceIn<^User, "shared"> satisfies "shared";

PlaceOf<local User> satisfies "local";
SpaceOf<local User> satisfies "local";
PlaceIn<local User, "shared"> satisfies "local";

PlaceOf<shared User> satisfies "shared";
SpaceOf<shared User> satisfies "shared";
PlaceIn<shared User, "local"> satisfies "shared";

PlaceOf<User | local User | shared User> satisfies "ambient" | "local" | "shared";
SpaceOf<User | local User | shared User> satisfies "local" | "shared";
PlaceIn<User | local User | shared User, "local"> satisfies "local" | "shared";
PlaceIn<User | local User | shared User, "shared"> satisfies "local" | "shared";
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
WithOwnership<shared User, "owned"> satisfies Owned<shared User>;
WithPlace<shared User, "ambient"> satisfies Placed<User, "ambient">;
WithAccess<User, "readonly"> satisfies readonly User;
WithAccess<^User, "readonly"> satisfies ^readonly User;
WithAccess<&User, "exclusive"> satisfies &exclusive User;
```

Because memory qualification is just ordinary type-level computation, userland code can introspect and rewrite ownership and placement using the same type system we already use for all other types.
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
PlaceOf<typeof localBuffer> satisfies "ambient";
PlaceOf<typeof sharedBuffer> satisfies "shared";
```

## Polymorphism

Since ownership, access, lifetime, and placement are all reified as memory form types, contracts and implementors get to be polymorphic and (somewhat) conditional over their ownership, space, and access, even on the receiver type.
That lets types expose one natural operation when only the projected form changes, and separate operations when the semantics actually differ.
The caller chooses the level of control by writing the expression / providing the type they mean:

```ds
process(user);            // managed/default value
process(&readonly user);  // readonly borrowed access
process(&user);           // mutable borrowed access
process(&exclusive user); // exclusive borrowed access
process(^user);           // owned value
```

Dispatch resolution uses the actual form during overload resolution, so container interfaces like `Iterable<T>` can support ordinary TypeScript iteration and borrowed iteration without adding Rust-style method family explosion:

```ds
declare type Point = { x: number; y: number };
declare const points: Array<Point>;

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

// moved iteration
for (const point of ^points) {
    point satisfies Point;
}
```

## Capabilities

Inspired by many other systems-y languages, Destack encodes synchronization and memory primitives as trait-like interfaces like `Copy`, `Clone`, `Send`, and `Sync`.

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |
| `Send` | Value can cross a Worker boundary. |
| `Sync` | References to shared values can be used concurrently through the type's own API. |

Like Rust's auto traits, `Send` and `Sync` are derived structurally by default (a type is `Send` when all of its fields are) and can be opted out of explicitly.
Implementing `Send` or `Sync` _manually_ - against the structural evidence, as interior-mutability primitives must - is an `@unsafe` claim like any other unchecked memory assertion.
For borrows and handles, the rules follow per memory form:

| Form | Crosses Workers when |
| --- | --- |
| `T` (local managed) | never, local handles stay on their Worker |
| `shared T` | `T: Sync` for aliased access |
| `^T` | `T: Send` |
| `&readonly T` | `T: Sync`, and the source is owned or static |
| `&exclusive T` | `T: Send`, and the source is owned or static |
| `&T` | never, aliased mutability |
| `shared` static binding | requires `T: Sync` outright, the binding reaches every Worker by construction |

The `&T` row is the important one: the aliased-mutable middle of the borrow ladder is only safe because a Worker is a single-threaded execution domain, and so it must never cross a Worker boundary.
Also, note again `shared T` means `T` lives in [shared space](#shared-space); it does _not_ make `T` automatically `Sync` by itself.
Userland APIs such as channels, Worker pools, atomics, locks, and actors can require `Send` or `Sync` when they need those stronger guarantees, very similar to Rust or even Swift.

## Synchronization

The standard library provides the usual memory and synchronization primitives on top of this unified memory system.
The full details are documented in the library, but the basics should be familiar to anyone with a systems-level background.
This is also where the `@unsafe` actually lives: a `Mutex<T>` mutates through a non-exclusive receiver via `UnsafeCell` and atomics, and its guard manufactures the `&exclusive T` that the runtime lock (not the borrow checker) guarantees - userland just composes the safe surface.

| Primitive | Contract |
|-----------|----------|
| `Box<T>` | unique heap ownership for `Owned<T>`, with deterministic drop when `T: Drop` |
| `Rc<T>` | local shared ownership of `Owned<T>`, non-atomic refcount, not transferable across Workers |
| `Arc<T>` | shared ownership of `Owned<T>`, atomic refcount, transferable when `T` satisfies the required `Send` / `Sync` bounds |
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
