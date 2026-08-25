---
title: Expressions
description: Values, control flow, dispatch, errors, metaprogramming, and modules.
order: 22
---

# Expressions

## Values

Destack supports "expressions as values" where (almost) all statements are expressions that produce values, and the last expression (no trailing `;`) becomes the value of the overall expression.

```ds
const result = if (condition) {
    computeA()
} else if (condition) {
    computeB()
} else {
    computeB()
};

function add(a: int, b: int): int {
    a + b // implicit return
}
```

If-let expressions enable nice sugar for matching a value with a refutable pattern in a conditional.
Bindings from the pattern are available in the positive branch:

```ds
const result = if (let value! = maybe) {
    value
} else {
    0
};

if (let (x, y) = point) {
    print(x + y);
}
```

Binding conditions also work in `while` loops, where each successful match scopes its bindings
through the remaining condition and the loop body:

```ds
while (let value! = queue.tryPop() && value.isReady()) {
    process(value);
}
```

We adopt the proposed `do { ... }` syntax to denote a block into an expression when braces would otherwise be ambiguous with an object expression or statement block.
The final expression without a trailing semicolon becomes the block value.

```ds
const user = do {
    const record = loadUser(id)?;
    User.fromRecord(record)
};
```

## Closures

Closures in Destack work essentially like TypeScript's closures, capturing the surrounding lexical environment and preserving lexical `this` around a generic `Function<Parameters, Return>`.
"Arrow function types" are syntax sugar for that form, so `(message: string) => Result<void, IOError>` is the same type as `Function<(string,), Result<void, IOError>>`.

```ds
let count = 0;

const next = () => {
    count += 1;
    return count;
};
next satisfies () => number;
next satisfies Function<(), number>;

const read = () => count;
```

The type we get by default for function types like `(Parameters) => Return` is a `Function<Parameters, Return, const M: Multiplicity>`, which is a fat pointer that can (optionally) capture an environment. The "multiplicity" parameter decides how often it may be invoked: `"repeatable"` is the default, and `"once"` makes it an affine value consumed by its first invocation.
When a true thin pointer is required, use `FunctionPointer`.
Thus, `Function`s also behave more like `Dynamic` by default, and explicit generics are required to force monomorphisation:

```ds
function apply<F: (int32) => int32>(callback: F, value: int32): int32 {
    return callback(value);
}
```

As with all of Destack, the `Function`s behind closures behave like one would expect in TypeScript by default, with additional control available on demand via the regular memory modifiers like `&Function<(string,), void>`.
By default, captures preserve variable identity: the captured variable's storage is automatically managed by the compiler, and every closure that captures that variable observes the same storage.
The capture policy can be configured via the `@capture` decorator:

| Policy | Meaning |
| --- | --- |
| `"manage"` | preserve variable identity through compiler-managed storage |
| `"borrow"` | capture borrowed access to the original binding |
| `"copy"` | snapshot the current value |
| `"move"` | move the binding into the closure |

Explicit capture forms choose how the callable value itself will be stored in the closure environment:

```ds
let count = 0;

@capture("borrow")
let borrowed: &Function<(), int32> = () => count;

let value = 0;

@capture("move")
let owned: ^Function<(), int32> = () => value;

let state = 0;

let managed: Function<(), int32> = () => state;
```

The short form for `@capture` sets the default for every captured binding, and the object form overrides selected bindings, including `this`:

```ds
class Client {
    prefix: string;

    make(socket: Socket, logger: Logger): (message: string) => Result<void, IOError> {
        @capture({
            default: "copy",
            socket: "move",
            logger: "borrow",
            this: "borrow",
        })
        return (message) => {
            logger.info("sending");
            return socket.write(`${this.prefix}: ${message}`);
        };
    }
}
```

Of course, closures with custom capture behavior must still follow general ownership rules - for example, if one closure moves a binding, later uses or captures of that binding are rejected.

```ds
@capture("move")
let close: Function<(), Result<void, IOError>, "once"> = () => socket.close();
```

## Continuations

Async functions and generators are closures that can pause and be resumed later via stackful `Continuation`s: the runtime parks the live frame and hands back an ordinary owned value whose one-shot `resume(value)` continues the frame, whose `complete(value)` enters generator completion, and whose `Drop` destroys retained execution without re-entering it.
Generators store continuations directly, while promises and tasks expose completion under their respective observation and ownership rules:

| Form | Meaning |
|------|---------|
| `Continuation<TResume, TYield, TReturn>` | one-shot owned continuation, `Drop` destroys it |
| `Promise<T>` | repeatable Worker-local completion for a copyable value |
| `Task<T>` | consuming Worker-local completion with cancellation and scope ownership |
| `Generator<Y, R, N>` | Worker-local suspended generator |
| `AsyncGenerator<Y, R, N>` | Worker-local suspended async generator |
| produced `T` | value eventually produced by async code |

As in TypeScript, `await` and `yield` are (stackful) suspension points: the entire stack up to that point is parked, and some other task gets to run.
In pure managed land, suspension works as before, and managed values can be stored in parked frames because it's all - well - managed.

```ds
type User = { name: string; };

async function read(user: User): Promise<string> {
    const name = user.name;
    await tick();
    return name; // valid as before
}
```

## Tasks

Stackful continuations and (relatively) cheap Workers make Destack's concurrency ergonomic and _structured_ by default, while also keeping TypeScript's familiar `Promise` behavior: calling an async function starts it eagerly and returns a worker-local `Promise<T>` that can be stored, combined, and awaited as usual.
Pending handles live in ordinary places such as a `Promise`'s reactions, the scheduler queue, or a `Task`, while task cancellation remains an explicit cooperative operation.
Work that outlives its frame can be spawned into an explicit `TaskScope` following the structured concurrency model of Trio and Kotlin.
Asynchronous disposal cancels pending children and waits for their cleanup:

```ds
async function crawl(seeds: [Url]): Promise<Report> {
    await using scope = TaskScope.open();

    const tasks = seeds.map((seed) =>
        scope.spawn(async (): Task<Page> => await fetch(seed)),
    );
    const pages = [];
    for (const task of tasks) {
        pages.push(await task);
    }

    return Report.from(pages);
} // asynchronous disposal cancels pending children and waits for cleanup
```

Cancelling a task resumes its parked continuation into the cancellation path at its suspension point, cleanup (`using` / `finally` / `Drop`) runs deterministically, and cancellation stops at the task boundary while the Worker carries on.
Awaiting a cancelled task enters the awaiting coroutine's cancellation path.
By the time a frame exits normally, everything it started must be awaited, returned, or handed to a scope - the `no-floating-promises` rule, `deny` by default in `.ds` (strict TypeScript codebases already lint this, Destack just means it) - and when a frame unwinds instead, its still-pending children are cancelled:

```ds
async function refresh(cache: Cache): Promise<void> {
    fetchAndStore(cache); // ERROR: floating promise, await it or hand it to a scope
}
```

## Patterns

TypeScript has pattern based destructuring for arguments and assignment-like expressions, and Destack extends that idea into `match`, `if (let ...)`, `let ... else`, and `catch match` with a full suite of patterns for every type family:

| Family | Example | Meaning |
|--------|---------|---------|
| Wildcard | `_` | match and ignore the value |
| Binding | `value` | bind the matched value |
| Literal | `"ok"`, `0`, `true` | match one literal value |
| Range | `0..10`, `..=255` | match an integer, `bigint`, or `char` interval |
| Tuple | `(x, y)` | destructure a tuple value |
| Sequence | `[head, ...tail]` | destructure finite ordered elements |
| Object | `{ kind: "ok", value }` | destructure a structural object |
| Nominal object | `Point { x, y }`, `User { name }` | match a nominal object-shaped value and destructure stored fields |
| Nominal tuple | `UserId(value)`, `Config({ debug })` | match a newtype and project its backing value |
| Enum | `State.Ready` | match a nominal enum variant without payload |
| Union | `0 | 1 | 2` | accept any listed pattern |
| Rest | `...tail` | bind the remaining sequence view or object fields |
| Default | `name = "guest"` | bind a fallback when the selected value is `undefined` |
| Must | `value!` | bind the non-nullish value |
| Borrow binding | `&readonly value`, `&value`, `&exclusive value` | bind the selected place through a borrow |
| Move binding | `^value` | bind the selected place by ownership |
| Dereference | `*Point { x, y }` | dereference the selected place before matching |
| Guard | `pattern if (condition)` | require an extra boolean condition |

For exhaustive matching with those patterns, Destack supports the `match` expression:

```ds
match (result /* Result<T, E> */) {
    Ok { value } => process(value)
    Err { error } if (isRetryable(error)) => retry()
    Err { error } => fail(error)
}
```

Guarded arms narrow their own bodies, but they do not contribute to exhaustiveness by themselves (because the guard can reject a value that the pattern matched).
Like with other conditional expressions, the resulting type of a match expression is the union of its arms' types.

```ds
declare const point: Point;
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at ${x}, ${y}` // irrefutable if point: Point
}
```

Some patterns are irrefutable, meaning they always match, and then no fallback branches are needed at all.
Refutable patterns require some fallback such that all branches are always covered: a `match` fallback arm, an `else` branch for `if (let ...)`, or an `else` continuation for `let ... else`.

```ds
declare const point: Point;
match (point) {
    Point { x: 0, y } => "vertical"
    Point { x, y: 0 } => "horizontal"
    _ => "neither" // required fallback
}

declare const result: Result<int32, string>;
let Ok { value }! = result else {
    return Result.err("missing value");
};
```

Unlike with construction (`{ ... }` for objects, `T { ... }` for structs, `new T(...)` for classes), the pattern destructuring unifies structs and classes into a single nominal object pattern (`T { ... }`).
Admittedly, this is a little suboptimal since it's not perfectly symmetrical, but we couldn't think of a more reasonable syntax that's not ambiguous or "magic" in some worse way.

```ds
class User {
    name: string = "";

    get displayName(): string {
        return this.name;
    }
}

declare const user: User;

match (user) {
    User { name } => name
}
```

The patterns match only real fields, not getters or setters:

```ds
match (user) {
    User { displayName } => displayName // ERROR: getter
}
```

Computed object pattern keys must close to static terms when matching closed object-shaped values.
Dynamic computed keys are still valid against indexed sources, because those are resolved through the ordinary `Index` / `IndexSet` surface rather than through declared fields.

## Guards

Guards are boolean expressions that can refine types, like `"name" in value`, `instanceof`, and `value is T` checks:
 - `"name" in value` for object-shaped values.
 - `instanceof` for classes.
 - `value is T` for primitive tags, union cases, exact runtime types.

Unlike in TypeScript, the runtime `typeof` check cannot be used as a guard, since its just too vague to narrow anything useful with.
(The `typeof` type operator is a separate feature and still queries the type of a value in type space, as in `PlaceOf<typeof buffer>`.)
Instead, Destack supports an additional precise `value is T` to check whether the current runtime representation of `value` carries the case, type identity, or registered relation for `T`:

```ds
struct User {
    name: string;
}

function label(value: User | string): string {
    if (value is User) {
        return value.name;
    } else {
        return value;
    }
}
```

Like other guards, `value is T` returns `boolean` and narrows the branch:
 - When `true`: narrows to the part of its current type that can be `T`.
 - When `false`: narrows away the covered part (when that can be represented).
For union values, the guard test sees "through" the union payload:

```ds
const value: string | int32 = 1;

if (value is string) {
    value satisfies string;
} else {
    value satisfies int32;
}
```

## Loops

For convenience and clarity, Destack supports `loop` as the explicit infinite loop form, and like other expressions, loops can produce a value through `break`.

```ds
const line = loop {
    const input = readInput();
    if (input == "quit") {
        break "done";
    }
    process(input);
};

let status = outer: loop {
    break outer: "done";
};
status satisfies "done";
```

## Using

Resource management with `using` and `await using` follows the [TC39 explicit resource management proposal](https://github.com/tc39/proposal-explicit-resource-management).
`Dispose` and `AsyncDispose` define the cleanup protocols as nominal interfaces:

- `using` accepts `Dispose | null | undefined`.
- `await using` accepts `AsyncDispose | Dispose | null | undefined`, and falls back to synchronous disposal when the resource only implements `Dispose`.
- `null` and `undefined` are ignored, following the spec.

Resources are cleaned up at lexical scope exit in LIFO order, and `await using` runs async cleanup when required.
Cleanup - that is, the dispose function - runs when the scope exits for any reason: fallthrough, `return`, `break`, `continue`, or `?`.
The same using form also works in loop form, where it applies for every iteration.

```ds
{
    using input = openFile(inputPath),
          output = openFile(outputPath);

    copy(input, output);
} // output is disposed, then input is disposed

async function runQuery(sql: string): Result<Row[], DatabaseError> {
    await using connection = await pool.connect();

    return await connection.query(sql);
} // connection is disposed and awaited

for (using file of files) {
    process(file);
} // file is disposed after each iteration
```

## Operators

Destack extends TypeScript operators with typed overloads and some additional precision.
Logical operators (`&&`, `||`, `??`), optional chaining, assignment, and strict identity (`===`, `!==`) are not (directly) overloadable, as usual, and same for increment (`++`) and decrement (`--`).
Compound assignment operators like `+=` are desugared into their component operations (`+` and `=`), and are thus indirectly overloadable.

| Operator | Example | Interface |
|----------|---------|----------|
| `+` | `a + b` | `Add<T>` |
| `-` | `a - b` | `Subtract<T>` |
| `*` | `a * b` | `Multiply<T>` |
| `/` | `a / b` | `Divide<T>` |
| `%` | `a % b` | `Remainder<T>` |
| `**` | `a ** b` | `Power<T>` |
| `+` | `+a` | `Plus` |
| `-` | `-a` | `Negate` |
| `&` | `a & b` | `And<T>` |
| `\|` | `a \| b` | `Or<T>` |
| `^` | `a ^ b` | `Xor<T>` |
| `~` | `~a` | `Not` |
| `<<` | `a << b` | `ShiftLeft<T>` |
| `>>` | `a >> b` | `ShiftRight<T>` |
| `>>>` | `a >>> b` | `ShiftRightUnsigned<T>` |
| `==`, `!=` | `a == b` | `PartialEqual<T>` |
| `===`, `!==` | `a === b` | `StrictEqual<T>` |
| `<`, `<=`, `>`, `>=` | `a < b` | `Compare<T>` or `PartialCompare<T>` |
| `[]` | `a[i]` | `Index<I>` |
| `[] =` | `a[i] = v` | `IndexSet<I, V>` |
| `*` | `*a` | `Dereference<"readonly">` |
| `* =` | `*a = v` | `Dereference<"mutable"> & T extends OverwriteStable` or `Dereference<"exclusive">` |

Equality `==` / `!=` follows Rust's split between "partial" and "total" equality:
- Equality operators `==` and `!=` dispatch through `PartialEqual<T>.equal`
- `Equal<T>` is a stronger _marker_ interface based on `PartialEqual<T>`

`Equal` and `PartialEqual` are different because of floating point numbers, since e.g. `NaN` does not equal itself.
Comparison operators `<` / `>` follow the same pattern and support `PartialCompare<T>` where ordering may be undefined (e.g., floats), and `Compare<T>` when ordering is total (e.g., integers).
Strict identity `===` still keeps its TypeScript meaning: by value for primitives (including `string` and `char` contents, and `NaN === NaN` is still `false`), and by reference identity for managed objects.

Dereference operators are a little different from the main "value-shaped" operators, because `Dereference<A>` transparently _dereferences_ (projects) access forms on use.
The `*` operator projects the place behind the borrow returned from `Dereference.dereference()`, so the dereference expression has the pointee type while writes back via the returned indirection.

```ds
struct Box<T> {
    ptr: ^T;
}

extension<T, const A: Access = "readonly"> of Box<T> implements Dereference<A> {
    type Output = T;

    dereference(): WithAccess<&T, A> {
        &this.ptr
    }
}
```

For example, `*box` flows via `Dereference<"readonly">`, while assignment through `*box` requires `Dereference<"mutable">` when the value is `OverwriteStable` and `Dereference<"exclusive">` otherwise.
Importantly, member lookup and method calls may auto-dereference transparently through `Dereference` without any special syntax - this is what enables ergonomic access to smart pointer like wrappers and guards.

## Arithmetic

Destack's sized numeric types are checked for correctness _in all modes_, no ifs or buts, no debug-only checking and no silent release-mode wraparound:

- **Overflow traps.** Arithmetic on sized integer types (`+`, `-`, `*`, `**`, `<<`, negation) traps when the result does not fit the type.
- **Division traps.** Integer division and remainder by zero trap (`/`, `%`), as does overflowing division (`int32.MIN / -1`).
- **Floats follow IEEE-754.** Float arithmetic never traps; division by zero, `NaN`, and infinities behave exactly as in TypeScript.

When modular or clamping semantics are desired, we can say so explicitly with the standard library helpers:

```ds
declare const a: uint8;
declare const b: uint8;

a.wrappingAdd(b) satisfies uint8;            // modular arithmetic
a.saturatingAdd(b) satisfies uint8;          // clamps at the bounds
a.checkedAdd(b) satisfies uint8 | undefined; // detects overflow as a value
Wrapping(a) + Wrapping(b);                   // wrapping by type
```

Conversions follow the same explicitness rule as the rest of Destack: `as` between numeric types is allowed only when every value of the source type is representable in the destination (lossless), and lossy conversions must pick their behavior explicitly:

```ds
declare const wide: int32;

const a: int64 = wide as int64;   // OK: lossless widening
const b: uint8 = wide as uint8;   // ERROR: lossy conversion
const c = wide.truncate<uint8>(); // explicit: keep the low bits
const d = wide.saturate<uint8>(); // explicit: clamp into range
const e = wide.tryInto<uint8>();  // explicit: uint8 | undefined
```

Similarly, mixed-type arithmetic (different widths or int/float mixes) requires explicit conversion to a common type first; the only implicit numeric flow is literal typing as described in [Primitives](./types.md#primitives).

## Ranges

Range expressions like `a..b` produce range values for slicing, indexing, iteration, and any APIs that want to think in terms of bounds.
Like Rust and Python, the default range is half-open, and all range forms implement `RangeBounds<T>`, whose `startBound()` and `endBound()` methods return `Bound<T>`:

| Expression | Type | Meaning |
|------------|------|---------|
| `start..end` | `Range<T>` | Include `start`, exclude `end` |
| `start..=end` | `RangeInclusive<T>` | Include both bounds |
| `start..` | `RangeFrom<T>` | Include `start`, no end bound |
| `..end` | `RangeTo<T>` | No start bound, exclude `end` |
| `..=end` | `RangeToInclusive<T>` | No start bound, include `end` |
| `..` | `RangeFull` | No start or end bound |

Ranges work in patterns and subscripts exactly like one would expect from other languages:

```ds
let values: Slice<int32> = [1, 2, 3, 4, 5];

(&values)[1..4] satisfies &Slice<int32>;
(&readonly values)[1..4] satisfies &readonly Slice<int32>;
(&exclusive values)[1..4] satisfies &exclusive Slice<int32>;
```

## Properties

Object literals and declared fields and methods have fixed identifier, string, or integer names.
Use `value[key]` for runtime selection and computed fields in object patterns for destructuring indexed values.

## Dispatch

"Dispatch" is how calls, member accesses, and overloadable operators select the specific field or method to use.
Destack's overload resolution rule is based on TypeScript: build the candidate set, keep candidates compatible with the arguments as written, then pick the first match _in declaration order_:

### Overloads

Unlike in TypeScript, there may be multiple overloaded _implementations_ for the same name and scope:

```ds
function parse(input: string): int32 {
    return parseInt(input);
}

function parse(input: int32): int32 {
    // legal, actually different implementation!
    return input;
}
```

Members work in the same way (after receiver lookup): inherent members first, then visible extension members in declaration order.
Because Destack has strict types, property access and method calls may use different access paths; that is, a field and method can share a source name: `value.name` resolves the field/accessor projection, while `value.name()` resolves the method-call projection.
(Fields and accessors share the property projection and therefore cannot share a name.)

### Operators

For overloadable operators, the operator decides the interface to check, and the left operand is the receiver (matching how it is written in the interface implementation).
Binary operators do not fall back to the right operand, so if both operand orders are desired - `a + b` and `b + a`, both receiver implementations must exist.

```ds
newtype interface Add<T = this> {
    type Output;

    add(other: T): this.Output;
}

extension of Vector2 implements Add<Vector2> {
    type Output = Vector2;

    add(other: Vector2): this.Output {
        Vector2({ x: this.x + other.x, y: this.y + other.y })
    }
}

extension of Vector2 implements Add<float32> {
    type Output = Vector2;

    add(other: float32): this.Output {
        Vector2({ x: this.x + other, y: this.y + other })
    }
}

const moved = position + offset; // Add<Vector2>
const padded = position + 1.0;   // Add<float32>

moved satisfies Vector2;
padded satisfies Vector2;

1.0 + position; // requires Add<Vector2> on float32
```

### Interfaces

As discussed in Types, structural interfaces keep normal TypeScript shape checking and mostly work exactly as expected: bare structural interface annotations are constraints in checking positions and erase to the dynamic `Dynamic<T>` in storage positions.
Because structural interfaces are satisfied by shape, writing `implements` on one is only an explicit declaration-site check.

```ds
interface PointLike {
    x: int32;
    y: int32;
}

struct Point implements PointLike {
    y: int32;
    x: int32;
}

function lengthSquared(point: PointLike): int32 {
    point.x * point.x + point.y * point.y
}
```

One point of difference to TypeScript is mutability through interfaces, because we need to actually compile the code into something with a fixed shape.
Interface fields are still read-write by default, but concrete fields satisfy mutable structural fields only when their types match exactly (after normal alias and newtype normalization).

```ds
interface PointLike {
    readonly x: int32 | float32;
}

struct Point {
    x: int32;
}

const point: PointLike = Point { x: 1 };
point.x satisfies int32 | float32;
```

Readonly fields only need, well, reads, so they can widen through ordinary implicit casts and nested structural views.
However, if `PointLike.x` were mutable, this conversion of `Point` to `PointLike` would be rejected because writing through `PointLike` would no longer be correct (it would have to implicitly widen, but it doesn't and cannot know that!).

### Index Signatures

Index signatures like `{ [index: string]: string }` (as in `Record<K, V>`) still work like structural constraints, and can be satisfied by closed object shapes _or_ by types implementing `Index` for readonly access and `IndexSet` for writes.

```ds
interface Bag<T> {
    readonly [key: string]: T;
}

// closed object view
const counts: Bag<int32> = { apples: 3, oranges: 2 };

function read<T>(bag: Bag<T>, key: string): T | undefined {
    bag[key]
}

read(counts, "apples") satisfies int32 | undefined;

// growing keyed storage uses a represented collection
const dynamicCounts = new Map<string, int32>();
dynamicCounts.set("apples", 3);
dynamicCounts.has("apples") satisfies boolean;
dynamicCounts satisfies Index<string>;
dynamicCounts satisfies IndexSet<string, int32>;
```

### Unions

Member dispatch on union receivers is resolved per variant, and every variant must expose that member.
If all variants resolve to the same implementation, the call is static; otherwise the result type is the union of the selected return types and the compiler emits dynamic checks to dispatch on the right member at runtime.

```ds
struct TcpStream {
    write(chunk: [uint8]): Result<usize, IOError> {}
}

struct MemoryBuffer {
    write(chunk: [uint8]): Result<usize, never> {}
}

function writeAll(sink: TcpStream | MemoryBuffer, chunk: [uint8]) {
    const written = sink.write(chunk);
    written satisfies Result<usize, IOError> | Result<usize, never>;
}
```

### Coherence

Unlike Rust, Destack has no _general orphan rule_: any module may extend any nominal type and implement any nominal interface for it.
And because Destack always compiles whole programs from source, coherence can be enforced where it is actually needed instead of at every possible declaration site, which is quite nice:

| Claim | Scope | Conflict |
| --- | --- | --- |
| Extension members | lexical: same file, or explicitly imported | ambiguity error at the use site |
| `implements` declarations | global: unique per (type, interface instantiation) pair | compile error pointing at both declarations |
| Operator and capability dispatch | global: works wherever the interface is nameable | none possible, implementations are unique |

For `implements`, uniqueness covers blankets too (no specialization, and bare-bounded blankets only from the interface's package, see [Blankets](./types.md#blankets)) - and this is how we keep generic instantiation coherent: a `Set<Vector2>` is always built and queried under the same `Hash` implementation, no matter which modules the value comes through.

When two packages do collide, the build fails and the fix is source-level (drop a dependency, or vendor and patch).
Libraries _should_ therefore only implement pairs they own one side of (a default-`warn` diagnostic nudges accordingly); foreign-on-foreign implementations belong in applications, where nothing downstream can collide.

## Errors

The banishing of exceptions is Destack's most immediately noticeable divergence from TypeScript: Destack uses **Result-first error handling** exclusively, and throwing exceptions is not allowed in any native Destack code.
Recoverable errors use `Result<T, E>`, integrate with `try` / `catch`, and can be propagated with `?`, `??`, and force-unwrapped postfix `!`.
(JavaScript exceptions remain valid _syntax_ because we need to integrate with JS targets directly, but in regular userland, exceptions are forbidden.)

The same rule also extends to async code: a Destack `Promise<T>` never rejects, because rejection is just an asynchronous exception.
Async failure travels as `Task<Result<T, E>>` or `Promise<Result<T, E>>`, [panics](#panics) unwind the Worker, and rejection-capable host promises are adopted into result carriers (or panic) at the host binding boundary.

### Error

Like in Rust, types that want to be handled as general errors explicitly implement the nominal `Error` interface:

```ds
newtype interface Error {
    display(): MaybeOwned<string>;

    source(): Dynamic<Error> | undefined {
        undefined
    }
}
```

### Result

Destack provides `Result<T, E>` as the primary error handling mechanism:

```ds
export struct Ok<T> {
    kind: "Ok" = "Ok";
    value: T;
}

export struct Err<E> {
    kind: "Err" = "Err";
    error: E;
}

export newtype Result<T, E> = Ok<T> | Err<E>;
```

We typically construct results through `Result.ok(value)` and `Result.err(error)`.
The variants are ordinary nominal data, so pattern matching works directly:

```ds
declare function parseInteger(raw: string): Result<int32, ParseError>;

function parsePort(raw: string): Result<uint16, ParseError> {
    const value = parseInteger(raw);
    if (value < 0 || value > 65535) {
        return Result.err(ParseError(`port out of range: ${raw}`));
    } else {
        return Result.ok(value as uint16);
    }
}

match (parsePort(input)) {
    Ok { value } => connect(value)
    Err { error } => report(error)
}
```

`Result` is great for synchronous error handling, and the exact same idea extends to asynchronous errors by composition: an async operation that can fail returns `Task<Result<T, E>>` or `Promise<Result<T, E>>`, and the standard library provides the result combinators and `?` propagation on those compositions directly.

```ds
declare function fetchUser(id: UserId): Task<Result<User, NetworkError>>;

async function loadProfile(id: UserId): Task<Result<Profile, NetworkError | DecodeError>> {
    const user = await? fetchUser(id); // `await? expr` is sugar for `(await expr)?`
    const profile = decodeProfile(user)?;
    return Result.ok(profile);
}
```

For `await`, we also have some `await? expr` sugar for `(await expr)?`, and `await! expr` is sugar for `(await expr)!`.

### Maybe, Must and Coalesce

`?`, postfix `!`, and `??` all unwrap the same `Try`-based absence-or-failure shape.
Opening removes outer `null` / `undefined`, opens one `Try` carrier, and removes `null` / `undefined` from the carrier's success value.
It's much simpler than it sounds:

```ds
declare const x: Result<T | null | undefined, E | null | undefined> | null | undefined;

// x?
// -> success: T
// -> failure (propagated): E | null | undefined
```

Nullish values on the failure side remain in the failure side.
The three operators differ on the failure case:

```ds
x?      // success T, failure leaves the expression
x!      // success T, failure traps
x ?? y  // success T, failure evaluates y
```

The try operator `?` keeps the success value and lets absence or failure leave the current expression in whichever way the container requires.
Inside a `try` block with `catch`, propagation transfers the failure value to the catch instead.

```ds
function readConfig(path: string): Result<Config, IOError | ParseError> {
    const text = readFile(path)?;
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

The try-coalesce operator `??` accepts the same shape locally "within" the expression with a direct fallback instead of letting it bubble up to the container as with `?`.
The result of `Result<T, E> ?? F` is the non-nullish opened success type joined with the fallback type `T | F`:

```ds
declare const defaultConfig: Config;

declare function loadConfig(): Result<Config, IOError> | null;
const a = loadConfig() ?? defaultConfig;
a satisfies Config;

declare function loadMaybeConfig(): Result<Config | null | undefined, IOError | null> | undefined;
const b = loadMaybeConfig() ?? defaultConfig;
b satisfies Config;
```

All unwrap operators unwrap exactly _one_ layer of `Try`, so nested `Try` values inside the success type also stay wrapped at the inner layer:

```ds
declare function loadNested(): Result<Result<Config, ParseError>, IOError>;

const c = loadNested() ?? defaultConfig;
c satisfies Result<Config, ParseError> | Config;
```

Postfix `!` is the "must" forced unwrap form: it opens the same outer nullish and single `Try` layer, but [traps](#panics) instead of propagating or falling back when the value is absent or failed:

```ds
const config = loadConfig()!;
config satisfies Config;
```

Regarding precedence, whitespace decides between the try operator and a ternary, which should mostly follow how we would naturally type (and format) these expressions anyway: an attached question mark is try-propagation, a detached one is a ternary condition.
This keeps the branches unambiguous in both directions: `flag ? -x : x` is a conditional, and `x? - 1` subtracts from the opened success value (so a compact ternary requires its spaces in `.ds`).

### Try

The "try" operators `?`, `??` and `!` are all based on the builtin `Try` operator interface, much like in Rust.
The carrier has an output type and a residual type, can branch into either case, and can then rebuild itself from an output value:

```ds
struct Continue<C> {
    kind: "continue" = "continue";
    value: C;
}

struct Break<B> {
    kind: "break" = "break";
    residual: B;
}

newtype ControlFlow<B, C> = Break<B> | Continue<C>;

newtype interface FromResidual<R> {
    static fromResidual(residual: R): this;
}

newtype interface Try extends FromResidual<this.Residual> {
    type Output;
    type Residual;

    static fromOutput(output: this.Output): this;
    branch(): ControlFlow<this.Residual, this.Output>;
}
```

For `Result<T, E>`, `Output` is `T` and `Residual` is `E`: `Ok { value }` branches to `Continue<T>` and `Err { error }` branches to `Break<E>`.

`FromResidual` is the half of the protocol that rebuilds a carrier, and `Try` extends it as a separate base interface so that `?` can propagate across carriers whose residual types differ.
Propagating a residual `R` out of a function requires the function's return type to satisfy `FromResidual<R>`, so a `Result<T, IOError>` residual travels into a `Result<U, ConfigError>` return exactly when that return type implements `FromResidual<IOError>`.

### Try-Catch-Finally

The well known `try`/`catch` forms still work with explicit `Try` propagation, which is quite cool:

```ds
declare function readConfig(path: string): Result<Config, IOError>;

try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e) { // e: IOError
    log("failed to read config:", e)
}
```

The example above uses `Result`, but any type implementing `Try` can be used:
- `try` provides an error propagation context (but does not unwrap `Result` values by itself)
- Use `?` inside the block to propagate `Try` failures into the catch
- Use `??` inside the block when the failure should be handled locally with a fallback

For convenience, Destack also supports a nicer `catch match` form that can branch on `Try` failures directly for some pretty pleasant syntactic sugar:

```ds
try {
    let config = readConfig()?; // -> Result<RawConfig, MissingError>
    config satisfies RawConfig;

    let config = parseConfig(config)?; // -> Result<Config, FormatError>
    config satisfies Config;

    // ... do stuff with config ...
} catch match (failure) { // failure: MissingError | FormatError
    MissingError { path } => Report.wrap(failure, `missing config: ${path}`)
    FormatError { line } => Report.wrap(failure, `bad format on line ${line}`)
}
```

Finally arms run as usual after the `try` / `catch` body, including when `?` leaves the block early.
Oh, and the whole try-catch-finally form is an expression like any other, so we can still do `let result = try { ... }` and scope the operation and result directly that way as well.

### Panics

Unrecoverable failures are hard **panics**: panics occur when a must unwrap (`!`) fails, when an explicit `panic("...")` runs, when a runtime check traps (arithmetic overflow, out-of-bounds indexing, lone-surrogate indexing), or when `unreachable` code is reached.
There is no userland `catch` for panics, that's what makes them panics: a panic means the program is outside its specified envelope.

Mechanically, a panic unwinds the _current_ `Worker`:
1. The panic starts unwinding from the trapping point with a message payload.
2. Cleanup runs on the way out - `using` / `await using` disposal, `finally` arms, and `Drop` glue for owned values - in the usual LIFO order.
3. The Worker terminates; a parent or supervisor observes the termination and gets the `Panic` struct - message, source location, and stack trace when available - through the regular `Worker` API and decides what to do (restart, propagate, report).
4. A panic _during_ that cleanup aborts: there is no unwinding the unwinding.

Conveniently, the Worker thus also becomes the fault boundary, mirroring both the web's worker model and (roughly) Erlang-style supervision: a panic never silently corrupts sibling Workers, and the test harness and the simulator can observe panics as ordinary (deterministic) Worker terminations without any language-level catch.
Nice.
For cases where we do want to unambiguously kill the whole program, Destack supports a stronger `abort` for genuinely unrecoverable states like detected memory corruption:

| Form | Cleanup | Boundary |
| --- | --- | --- |
| `panic` | unwinds with `using` / `finally` / `Drop` cleanup | terminates the Worker |
| `abort` | none, stops immediately | terminates the whole process |

## Trees (TSX)

TypeScript XML (`.tsx`) is a great way of writing UI-shaped code and has even seen some successful adoption for other tree-shaped data structures as well.
It's not perfect, but it is very useful in many situations, and Destack (`.ds`) natively supports `.tsx`-like constructs with the same rules:

```ds
// Wall.ds
<Wall id={1}>
    <Block name="foo" color={Color.RED} />
    <Block name="bar" color={Color.BLUE} />
</Wall>;

// Prompt.ds
<Prompt>
    <System>You are a helpful assistant.</System>
    <User>{userMessage}</User>
</Prompt>;

// Level.ds
<Level difficulty={3}>
    <Player position={spawn} />
    {enemies.map(e => <Enemy {...e} />)}
</Level>;
```

Tree literals build through a **tree builder**: a type implementing the `TreeBuilder` interface from `destack:tree`.
The builder resolves from the contextual type of the literal, so `const page: Panel = <div/>` builds through `Panel`'s `TreeBuilder` implementation.
A literal without a contextual builder reads the default builder from the `compiler.tree` option, spelled as `"<specifier>#<Export>"` like TypeScript's `jsxImportSource`.

```ds
extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel { /* ... */ }

    static fragment<Children: (...unknown[],)>(children: Children): Panel { /* ... */ }
}
```

Tags split by their written case, as in TSX:
 - **Lowercase tags** like `<div>` are keys of the builder's `Tags` row, never names in scope. The literal checks its attributes against the tag's declared row and builds through the builder's `element` static.
 - **`<>...</>` fragments** build through the builder's `fragment` static.
 - **Uppercase tags** like `<Header>` are **components**: ordinary names resolved through lexical scope. A callable component is invoked with its props row; a class component constructs through its constructor's props parameter; a struct component constructs through its literal field form, so `<Point x={1} y={2}/>` is `Point { x: 1, y: 2 }`. The produced value feeds the surrounding tree.

Components need no protocol: any callable or constructible type in scope serves.

```ds
struct Badge {
    label: string;
}

function Header(props: { title: string }): Panel { /* ... */ }

const page: Panel = <><Header title="hi"/><Badge label="new"/></>;
```

Attributes check per key against the declared row: written keys must exist in the row, values must be assignable to their properties, and every non-optional key must be provided.
Spread attributes `{...props}` contribute their enumerable members to the written row under TSX merge rules.
Component children synthesize the `children` prop and check against its declared type.

Children are **tuple-typed and move**: the children of a literal form a tuple in source order, with text children typed as string literals.
Spread children `{...pair}` splat statically sized tuple operands into the children tuple; dynamically sized operands are rejected.

## Decorators

Like TypeScript, Destack uses `@` for decorator-like constructs, but Destack supports both "annotations" and "decorators", and also many more constructs can be annotated / decorated.
The syntax for both data annotations and behavior decorators is unified, the target - the thing pointed to in `@<expr>` - decides:
 - **Annotations** are _values_ like `newtype`s. They add typed metadata to the target, but don't directly change the target's behavior.
 - **Decorators** are toolchain operations that contribute code or change the analyzed shape.

Annotations carry typed metadata. The toolchain interprets annotations with defined compiler behavior.

```ds
newtype deprecated = () | (string,);

@deprecated("use newAPI instead") // metadata annotation, doesn't do anything
function oldAPI() {
    // ...
}

@tracked
@derive(Clone, Debug)
struct User {
    id: UserId;
    name: string;
}
```

### Diagnostics

Like in other languages, (some of) Destack's diagnostics can be tuned with scoped decorators:
 - `@allow`: explicitly allow a specific diagnostic
 - `@warn`: warn about a specific diagnostic
 - `@deny`: error about a specific diagnostic
 - `@forbid`: forbid a specific diagnostic (cannot be overridden by `@allow`)
 - `@expect`: expect a specific diagnostic (suppress, error if not produced)

```ds
@allow("no-floating-promises", {
    if: import.meta.dev,
    otherwise: "deny",
    reason: "debug telemetry",
})
module {}
```

### Unsafe

`@unsafe` marks an operation that is unsafe to call. `@safe` marks an operation that contains unsafe implementation details behind a safe API.

```ds
@unsafe
declare function read<T>(pointer: *T): ^T;

@safe
function get<T>(items: Slice<T>, index: usize): T {
    if (index >= items.length) {
        panic("index out of bounds");
    }

    return items.unsafeGet(index);
}
```

### Derive

Similar to Rust, Destack `derive`s compiler-known interfaces for annotated declarations.

```ds
@derive(Clone, Debug)
struct User {
    id: UserId;
    name: string;
}
```

Destack supports capability derives for memory, comparison, formatting, hashing, and serialization:

| Derive | Library identity | Applies to | Explicit failure |
|--------|------------------|------------|------------------|
| `Copy` | `destack:memory.Copy` | nominal value types whose fields are all copyable | field or representation is not copyable |
| `SharedSafe` | `destack:memory.SharedSafe` | types whose structural closure stays shared-safe | field reaches worker-local state |
| `Clone` | `destack:memory.Clone` | nominal value types whose fields are cloneable | field is not cloneable |
| `Default` | `destack:memory.Default` | nominal value types whose fields have defaults | field has no default |
| `Debug` | `destack:ops.Debug` | nominal value types | field is not debug-formatable |
| `Display` | `destack:ops.Display` | nominal value types | field is not display-formatable |
| `PartialEqual` | `destack:ops.PartialEqual` | nominal value types | field is not partially comparable for equality |
| `Equal` | `destack:ops.Equal` | nominal value types | field does not have total equality |
| `PartialCompare` | `destack:ops.PartialCompare` | nominal value types | field is not partially orderable |
| `Compare` | `destack:ops.Compare` | nominal value types | field is not totally orderable |
| `Hash` | `destack:ops.Hash` | nominal value types | field is not hashable |
| `Serialize` | `destack:serde.Serialize` | nominal value types | field cannot be serialized by the selected serializer |
| `Deserialize` | `destack:serde.Deserialize` | nominal value types | field cannot be deserialized by the selected deserializer |

Also unlike Rust, Destack's `derive` supports automatic globally configured (and module/target/..-overridable) derives that are applied by default without explicit `derive` annotation whenever possible.
This is very convenient since most types do in fact want all the same basic well known `derive`s, but we can also trivially disable this globally, or override it per-item with an empty `@derive()`.

### Static If

Destack also supports a special intrinsic `@if` decorator that gates the inclusion of certain nodes based on a static term (roughly like `#[cfg(attr)]` in Rust).
The static term must be based on static data (like `import.meta`), and when the condition evaluates to false, the annotated thing is ignored and removed from checking and output.

```ds
interface FileSystem {
    open(path: string): Result<File, IOError>;

    @if(import.meta.platform != "windows")
    chmod(path: string, mode: uint16): Result<void, IOError>;

    @if(import.meta.platform == "windows")
    setAttributes(path: string, attrs: WindowsFileAttributes): Result<void, IOError>;
}
```

Static ifs may annotate any meaningfully "removable" source contribution:

| Context | Nodes | Static inputs |
| --- | --- | --- |
| Module level | imports, re-exports, top-level declarations, `module { ... }` decorators | profile, module metadata, literals |
| Declaration members | struct fields, class / interface / extension members, enum variants | profile, module metadata, literals, closed constants |
| Expression positions | statements, match cases, call / tree / generic arguments, tuple elements, object and type literal fields | profile, module metadata, literals, closed constants |

When a choice depends on generic parameters, static terms don't work (since they must be evaluated ahead of inference), and we can use type algebra and regular `const` gating instead for concrete specialisation.

## Module

Destack modules can contain (up to) one `module { ... }` declaration block, which carries decorators that apply to the whole source module.

```ds
@noHeap
module {}
```

Module metadata such as the role, labels, product, or active derives comes from the compiler / target / profile configuration and is read through `import.meta`.

## Globals

TypeScript supports ambient global typings, which were designed for typing the "magic" global objects provided by embedders, but it has no way to contribute _value_ globals in userland.
Destack supports "real" value `global { ... }` declarations to define such globals that can then be automatically included everywhere by (explicit) reference in the compiler / target configuration.
The active set of modules to consider for `global` declarations is configured via the `globals` field in the compiler / target configuration.

```ds
// browser-globals.ds
global { // just omit the `declare`!
    const window: Window = runtime.browser.window();
    const document: Document = runtime.browser.document();
}
```

A global block may also re-export named bindings from another module into the ambient globals.
Indeed, that is the same mechanism the well known Destack prelude uses to inject intrinsic language items:

```ds
global {
    export { Add, Subtract } from "destack:ops";
}
```

## Const Evaluation

Inspired by modern languages like Zig and Jai, Destack supports compile-time evaluation with `const` expressions: ordinary code evaluated by the compiler, with the results baked into the emitted artifact.

```ds
const LOOKUP_TABLE: uint8[] = const {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

Functions do not need to declare themselves as compile-time or runtime: the same function can run at compile time when all inputs are static, and at runtime when some input is only known at runtime:

```ds
function factorial(n: int): int {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

const BAKED = const factorial(10);      // compile time
BAKED satisfies int;

const COMPUTED = factorial(getUserInput()); // runtime (in this case, at module initialization time)
COMPUTED satisfies int;
```

When runtime execution would be meaningless or unsafe, a function can be declared `const function` to declare that a function has no runtime callable form, but otherwise uses normal function syntax.
(This is the reverse of a `constexpr`-style keyword: Destack functions run at compile time by usage, and `const function` marks the ones with no runtime form.)

```ds
const function fieldOffset<T>(name: string): usize {
    // inspect `T` at compile time
}

const offset = const fieldOffset<User>("name");
```

The evaluation scope for each const expression is isolated to its declaration site, and the only way to get a value "out" is to use const as an expression: no reaching into statics or globals is allowed.
(Locals _inside_ the expression may of course be mutated.)

```ds
const WIDTH = const {
    let width = 4;
    width *= 2;
    width
};

let counter = 0;
const {
    counter += 1; // ERROR: outer mutation
}
```

Const blocks can also appear as members on object-like types for static checks, where they run in the static environment of the declaration or instantiation that they appear in post-inference, and they can access the same [static terms](./types.md#static) as `@if`.

```ds
struct Buffer<const Size: uint> {
    const {
        assert(Size > 0 && Size <= 65536);
    }

    data: [uint8; Size];
}
```

Because const expressions are late-evaluated expressions, const conditions type check like ordinary conditions.
Both branches are analyzed, and the expression type is still the joined branch type.
The compiler may eliminate the untaken branch before final lowering when the condition is computed from static inputs:

```ds
function isPowerOfTwo(value: uint): boolean {
    if (value == 0) {
        return false;
    }

    let n = value;
    while (n > 1) {
        if (n % 2 != 0) {
            return false;
        }
        n /= 2;
    }

    return true;
}

function blockCost(width: uint): int32 {
    if (isPowerOfTwo(width)) {
        return 1;
    } else {
        return 2;
    }
}

const COST = const blockCost(8); // evaluated by the compiler

function blockMultiply<const Width: uint>(a: [int32; Width], b: [int32; Width]): int32 {
    const {
        assert(isPowerOfTwo(Width));
    }
}
```

A const parameter read as a value must appear in the signature, as `Width` does in the operand types above.
A value used only in the body belongs in an ordinary parameter: `blockCost` takes one, and `const blockCost(8)` evaluates the call at compile time.

Of course, const results must also be lowerable into the target artifact.
Plain data such as numbers, strings, arrays, tuples, objects, structs, and enums are all fine, but dynamic runtime resources like pointers and handles and such don't work because we can't meaningfully serialize them.
