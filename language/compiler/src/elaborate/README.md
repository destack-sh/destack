# Elaborate

Elaborate turns DIR into a "canonical" form by transforming and reifying ("elaborating") implicit or higher level concepts from the base DIR into "canonical DIR".

The exact responsibilities of Elaborate are unfortunately a bit fuzzy because we need to support both high-level targets like JS/TS *and* low-level AOT targets.
We try to keep most reifications / transforms the same across targets to reduce the combinatorial explosion, but in some cases it's inevitable, for example to retain some nullish coalescing behavior without complicating the JS/TS codegen backend.

## Pipeline

Elaborate is part of the front-end that runs per profile on checked DIR, before Execute patches comptime results in.
For the most part, we can think of Elaborate as "transformation", which "simplify" the DIR, and "reification", which makes some implicit logic explicit.

## Transform

Transform rewrites (and "simplifies") structure without changing meaning.
Evaluation order stays the same and the output remains target-independent DIR.

### Split multi declarators into single bindings

Multi declarator lets are split into one statement per binding.

```ds
// source
function init(): int32 {
    let a = 1, b = 2, c = 3;
    return a + b + c;
}
```

```ds
// after transform
function init(): int32 {
    let a = 1;
    let b = 2;
    let c = 3;
    return a + b + c;
}
```

### Unwrap single expression blocks in source if else

Single expression blocks inside source if else are unwrapped to enable ternary lowering.

```ds
// source
function choose(x: boolean, y: int32, z: int32): int32 {
    if (x) { y } else { z }
}
```

```ds
// after transform
function choose(x: boolean, y: int32, z: int32): int32 {
    if (x) y else z
}
```

### Lower if let bindings to match expressions

If let is normalized into a match expression before match lowering.
Else-if chains are lowered by nesting the else branch into another match.
Match lowering then emits explicit checks and bindings.

```ds
// source
function unwrap(value: Option<int32>): int32 {
    if let Some(x) = value { x } else { 0 }
}
```

```ds
// after transform
function unwrap(value: Option<int32>): int32 {
    match (value) {
        Some(x) => x
        _ => 0
    }
}
```

Nested if let chains produce nested matches.
Those nested matches become nested if expressions after match lowering.

### Lower match expressions into decision trees

Match expressions become decision trees with explicit blocks.

```ds
// source
function classify(x: Option<int32>): int32 {
    match (x) {
        Some(v) if v > 0 => v
        Some(v) => -v
        None => 0
    }
}
```

```ds
// after transform
function classify(x: Option<int32>): int32 {
    if (x is Some) {
        let v = x.value;
        if (v > 0) { v } else { -v }
    } else {
        0
    }
}
```

### Convert simple if else expressions to ternary form

Simple if else expressions are converted to ternary form.

```ds
// source
function abs(x: int32): int32 {
    if (x < 0) -x else x
}
```

```ds
// after transform
function abs(x: int32): int32 {
    x < 0 ? -x : x
}
```

### Insert explicit returns

Implicit returns in function bodies become explicit return expressions.

```ds
// source
function addOne(x: int32): int32 {
    x + 1
}
```

```ds
// after transform
function addOne(x: int32): int32 {
    return x + 1;
}
```

### Normalize value expressions into statement form

Control flow expressions used as values are rewritten into explicit assignments or returns.
This makes value flow explicit for Lower and codegen without changing ordering.

#### Normalize if expressions used as values

If expressions in value position are rewritten to assignments or branch returns.

```ds
// source
function total(flag: boolean, a: int32, b: int32): int32 {
    let x = if (flag) { a } else { b }
    return x + 1;
}
```

```ds
// after transform
function total(flag: boolean, a: int32, b: int32): int32 {
    let x;
    if (flag) { x = a } else { x = b }
    return x + 1;
}
```

```ds
// source
function choose(flag: boolean, a: int32, b: int32): int32 {
    return if (flag) { a } else { b }
}
```

```ds
// after transform
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { 
        return a;
    } else { 
        return b;
    }
}
```

#### Normalize block expressions used as values

Block expressions are lifted into statement blocks that assign to a temp.

```ds
// source
function scoped(flag: boolean, a: int32): int32 {
    let value = do { let x = a + 1; if (flag) { x } else { x + 1 } };
    return value * 2;
}
```

```ds
// after transform
function scoped(flag: boolean, a: int32): int32 {
    let value;
    { let x = a + 1; if (flag) { value = x } else { value = x + 1 } }
    return value * 2;
}
```

#### Normalize sequence expressions into statements

Sequence expressions become statements that preserve evaluation order.

```ds
// source
function ordered(a: int32, b: int32): int32 {
    let value = (log(a), log(b), b + 1);
    return value;
}
```

```ds
// after transform
function ordered(a: int32, b: int32): int32 {
    let value;
    log(a);
    log(b);
    value = b + 1;
    return value;
}
```

#### Normalize labeled block break values

Labeled blocks stay labeled, but break-with-value becomes assignment plus a plain break.

```ds
// source
function pick(flag: boolean): int32 {
    let value = outer: {
        if (flag) { 
            break outer 1;
        }
        2
    };
    return value;
}
```

```ds
// after transform
function pick(flag: boolean): int32 {
    let value;
    outer: {
        if (flag) { value = 1; break outer; }
        value = 2;
    }
    return value;
}
```

### Drop parenthesized expressions

Parenthesized expressions are removed because precedence is already encoded in the tree.

```ds
// source
function add(a: int32, b: int32): int32 {
    return (a + b);
}
```

```ds
// after transform
function add(a: int32, b: int32): int32 {
    return a + b;
}
```

## Reify

Reify makes certain abstractions explicit ("realized").
It is profile-aware and uses Analyze resolutions and profile libraries.
The output remains DIR and feeds Execute and Lower.

### Insert ownership conversions

Ownership conversions are represented explicitly in DIR as ownership expressions.
Reify normalizes explicit ownership operators and inserts implicit borrows at reference boundaries.
Implicit ownership conversions only create borrows and never transfer ownership.

Implicit conversions:
- `T` → `&T` or `&readonly T` when a reference is required and the value is addressable
- `^T` → `&T` or `&readonly T` when a reference is required
- `&T` → `&readonly T` to reborrow as shared

Ownership insertion happens before implicit cast insertion.

### Insert explicit and implicit casts

Reify resolves `Expression::As` nodes, with source casts marked as `CastSource::Explicit` and inserted casts marked as `CastSource::Implicit`.

Implicit casts are inserted at "type boundaries", i.e. in places where values move.
Null and undefined literals are reified the same way when a nullable or union target type is expected.

| Site | Example | After reify |
| --- | --- | --- |
| binding initializer | `let x: float64 = y;` | `let x = y as float64;` |
| assignment | `x = y;` | `x = y as T;` |
| return | `return y;` | `return y as T;` |
| call argument | `f(y);` | `f(y as T);` |
| ternary | `cond ? a : b` | `cond ? (a as T) : (b as T)` |
| match arm | `case => expr` | `case => (expr as T)` |

Certain conversions are guaranteed to be lossless and can be performed implicitly, and we reify them into real casts.

| Conversion | Implicit | Notes |
| --- | --- | --- |
| intN → intM | yes | when M > N and signedness matches |
| float32 → float64 | yes | widening |
| int → float | yes | only when the full int range fits the float mantissa |
| literal → float | yes | only when the literal is exactly representable |
| T → T \| U | yes | union upcast |
| T → T \| null \| undefined | yes | nullable upcast |
| subtype → base | yes | instance upcast when assignable |
| class/struct → interface | yes | instance upcast to interface layout |
| T → object | yes | non-primitive to object |
| T → any | yes | widen to any |
| T → unknown | yes | widen to unknown |
| interface → interface | yes | "rebuild" interface reference |

```ds
// source
function intoFloat(x: int32): float64 {
    let y: float64 = x;
    return y;
}
```

```ds
// after reify
function intoFloat(x: int32): float64 {
    let y = x as float64;
    return y;
}
```

```ds
// source
function take(node: Node | null): Node | null {
    let value: Node | null = null;
    return value;
}
```

```ds
// after reify
function take(node: Node | null): Node | null {
    let value = null as Node | null;
    return value;
}
```

These conversions require an explicit `as T` in source.
Analyze enforces the requirement and Reify only classifies explicit casts.

| Conversion | Explicit | Notes |
| --- | --- | --- |
| intM → intN | yes | narrowing |
| float64 → float32 | yes | narrowing |
| int ↔ float | yes | when not provably lossless |
| int signedness change | yes | checked |
| pointer ↔ int | yes | checked |
| pointer ↔ pointer | yes | checked |
| union downcast | yes | checked |
| nullable downcast | yes | checked |
| instance downcast | yes | checked |
| enum ↔ int | yes | checked |
| enum ↔ string | yes | checked |
| object → T | yes | checked |
| any → T | yes | unchecked |
| unknown → T | yes | checked |

Enum casts follow the enum backing type (int or string).
Downcasts from `any` are unchecked, and downcasts from `unknown` are checked.

```ds
// source
function trunc(x: float64): int32 {
    return x as int32;
}
```

```ds
// after reify
function trunc(x: float64): int32 {
    return x as int32;
}
```

### Reify resolutions

Reify uses Analyze resolutions stored in TypeTable to make symbol dispatch explicit.
Resolutions apply to all symbol lookups: operators, method calls, member access, and index acces:
- `Resolution::Builtin` - keep as builtin (primitives, no symbol needed)
- `Resolution::Static` - single target known at compile time
- `Resolution::Dynamic` - union type dispatch, transform to runtime `is` type checks with static branches

After reify, only `Builtin` and `Static` resolutions remain.
Lower and codegen never see `Dynamic` resolutions.

#### Operators

Operators with resolutions are transformed to method calls.

**Static resolution:**

```ds
// source
struct Vec2 { 
    x: int32; 
    y: int32;
}

extension of Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 {
        return Vec2 { x: this.x + other.x, y: this.y + other.y };
    }
}

function sum(a: Vec2, b: Vec2): Vec2 {
    return a + b;
}
```

```ds
// after reify
function sum(a: Vec2, b: Vec2): Vec2 {
    return a.add(b);  // Resolution::Static { target: Vec2.add }
}
```

**Dynamic resolution:**

When the receiver is a union type with different implementations, we insert `is` type checks:

```ds
// source
function add(a: Vec2 | Vec3, b: Vec2 | Vec3): Vec2 | Vec3 {
    return a + b;  // Resolution::Dynamic { candidates: [Vec2.add, Vec3.add] }
}
```

```ds
// after reify
function add(a: Vec2 | Vec3, b: Vec2 | Vec3): Vec2 | Vec3 {
    if (a is Vec2) {
        return a.add(b);  // Resolution::Static { target: Vec2.add }
    } else {
        return a.add(b);  // Resolution::Static { target: Vec3.add }
    }
}
```

**Special operator mappings:**

- `NotEqual` → `!a.equal(b)` (wrap in unary not)
- `<`, `<=`, `>`, `>=` → `a.compare(b)` and compare result to `Ordering` variants
- Unary operators: `Negate` → `a.negate()`, `Plus` → `a.plus()`

#### Method calls

Method calls on union types with different target symbols are split into type checks.

**Static resolution:**

```ds
// source
struct Cat { 
    name: string;
}

extension of Cat {
    speak(): string { 
        return "meow";
    }
}

function greet(c: Cat): string {
    return c.speak();  // Resolution::Static { target: Cat.speak }
}
```

```ds
// after reify (unchanged, already static)
function greet(c: Cat): string {
    return c.speak();  // Resolution::Static { target: Cat.speak }
}
```

**Dynamic resolution:**

```ds
// source
struct Cat { 
    name: string;
}

struct Dog { 
    name: string;
}

extension of Cat {
    speak(): string { 
        return "meow"; 
    }
}
extension of Dog {
    speak(): string {
        return "woof";
    }
}

function greet(pet: Cat | Dog): string {
    return pet.speak();  // Resolution::Dynamic { candidates: [Cat.speak, Dog.speak] }
}
```

```ds
// after reify
function greet(pet: Cat | Dog): string {
    if (pet is Cat) {
        return pet.speak();  // Resolution::Static { target: Cat.speak }
    } else {
        return pet.speak();  // Resolution::Static { target: Dog.speak }
    }
}
```

When dynamic call candidates have different parameter types, each static branch reifies its own call argument casts.

#### Member access

Field access on union types where fields have different offsets or types.

**Static resolution:**

```ds
// source
struct User { 
    name: string;
    age: int32;
}

function getName(u: User): string {
    return u.name;  // Resolution::Static { target: User.name }
}
```

```ds
// after reify (unchanged, already static)
function getName(u: User): string {
    return u.name;  // Resolution::Static { target: User.name }
}
```

**Dynamic resolution:**

```ds
// source
struct User { 
    name: string;
    role: string;
}

struct Admin { 
    name: string; 
    level: int32;
}

function getName(person: User | Admin): string {
    return person.name;  // Resolution::Dynamic { candidates: [User.name, Admin.name] }
}
```

```ds
// after reify
function getName(person: User | Admin): string {
    if (person is User) {
        return person.name;  // Resolution::Static { target: User.name }
    } else {
        return person.name;  // Resolution::Static { target: Admin.name }
    }
}
```

#### Index access

Index operations on union types with different `Index` implementations.

**Static resolution:**

```ds
// source
function first(arr: int32[]): int32 {
    return arr[0];  // Resolution::Static { target: Array.index }
}
```

```ds
// after reify (unchanged, already static)
function first(arr: int32[]): int32 {
    return arr[0];  // Resolution::Static { target: Array.index }
}
```

**Dynamic resolution:**

```ds
// source
struct Vec2 { x: int32, y: int32 }

extension of Vec2 implements Index<int32, int32> {
    index(i: int32): int32 { 
        if (i == 0) { 
            return this.x; 
        } else {
            return this.y;
        } 
}

function getFirst(v: int32[] | Vec2): int32 {
    return v[0];  // Resolution::Dynamic { candidates: [Array.index, Vec2.index] }
}
```

```ds
// after reify
function getFirst(v: int32[] | Vec2): int32 {
    if (v is int32[]) {
        return (v /* as int32[] */).[0]  // Resolution::Static { target: Array.index }
    } else {
        return (v /* as Vec2 */).index(0)  // Resolution::Static { target: Vec2.index }
    }
}
```

### Reify nominal constructor calls into tagged expressions

Calls that Analyze resolves as nominal constructors become tagged expressions in DIR.
Note that struct expressions (`Type { ... }`) are already bound, so reify only needs to handle call-form constructors (primarily newtypes).

```ds
newtype UserId = int64;
newtype Point = (float32, float32);

function build(): (UserId, Point) {
    let id = UserId(42);          
    let point = Point(1.0, 2.0);
    return (id, point);
}
```

### Reify must (`!`) into explicit downcasts

Must assertions are reified to explicit cast nodes.
For nullable unions this becomes a nullable or union downcast, which Lower can emit with runtime checks.

```ds
// source
function requireName(name: string | null): string {
    return name!;
}
```

```ds
// after reify
function requireName(name: string | null): string {
    return (name as string);
}
```

### Planned: desugar Try (and related Maybe and Coalesce)

This section describes the target shape, not current completed behavior.
Elaborate does not yet fully rewrite `Maybe` and `Coalesce` nodes into canonical control flow.
Lower does not currently accept leftover `Maybe` nodes, so this path remains an active Elaborate gap.

When this is implemented, overload resolution will be reified in all profiles using the same resolution machinery as operator overloading.
If no overload exists, explicit control flow will be inserted.
Coalesce will use Try semantics when the left side implements Try and nullish semantics otherwise.

```ds
// source
function loadCount(): Result<int32, Error> {
    let value = readCount()?;
    let fallback = readCount() ?? 0;
    return Result.ok(value + fallback);
}
```

```ds
// target after reify
function loadCount(): Result<int32, Error> {
    let __try0 = readCount();
    let value;
    if (__try0.isOk()) {
        value = __try0.value
    } else {
        return Result.fromError(__try0.error)
    };

    let __try1 = readCount();
    let fallback;
    if (__try1.isOk()) {
        fallback = __try1.value
    } else {
        0
    };

    return Result.ok(value + fallback);
}
```

Profiles with native nullish operators may keep them instead of rewriting.

```ds
// source
function nameOrDefault(name: string | undefined, fallback: string): string {
    return name ?? fallback;
}
```

```ds
// target JS/TS profile reify
function nameOrDefault(name: string | undefined, fallback: string): string {
    return name ?? fallback;
}
```

```ds
// target native profile reify
function nameOrDefault(name: string | undefined, fallback: string): string {
    if (name == null || name == undefined) {
        return fallback;
    } else {
        return name;
    }
}
```

### Realize tree literals (via `TreeTag` and `TreeTagBuilder`)

Tree literals are realized via "tag routing".
Value tags (e.g. `<Button />`) lower through `fromTree` on the resolved `TreeTag`.

```ds
// source
function view(label: string): unknown {
    return <Button>{label}</Button>;
}
```

```ds
// after reify
function view(label: string): unknown {
    return Button.fromTree({}, [label]);
}
```

Intrinsic tags (e.g. `<div />` and `<svg:path />`) lower through the active `TreeTagBuilder`.

```ds
// source
function panel(label: string): unknown {
    return <div className="card">{label}</div>;
}
```

```ds
// after reify
function panel(label: string): unknown {
    return TreeTagBuilder.Tag<"div">.fromTree({ className: "card" }, [label]);
}
```

Same goes for XML-style-namespaced intrinsic tags like `svg:path`:

```ds
// source
function namespaced(): unknown {
    return <svg:path />;
}
```

```ds
// after reify
function namespaced(): unknown {
    return TreeTagBuilder.Tag<"svg:path">.fromTree({}, []);
}
```

Fragment syntax lowers through the builder `Fragment` tag.

```ds
// TODO #Incomplete: revisit TreeTag reification (?)
```
