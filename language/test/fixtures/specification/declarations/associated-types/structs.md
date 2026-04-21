# Associated Types: Structs

Struct associated type tests live here.

## structs

### struct associated type alias is allowed

> Structs can declare associated type aliases.

```ds
struct Box<T> {
    type Item = T;
    value: Item;
}

const box: Box<string> = Box { value: "ok" };
box.value satisfies string;
```

### struct associated type can include a constraint

> Associated type aliases can include constraints.

```ds
struct SizedBox {
    type Item: number = int32;
    value: Item;
}

const boxed = SizedBox { value: 1 };
boxed.value satisfies int32;
```

### struct associated type can be referenced from the type

> Associated types are accessed via the containing type.

```ds
struct Box<T> {
    type Item = T;
    value: Item;
}

const value: Box<string>.Item = "ok";
value satisfies string;
```

### struct associated type can declare static parameters

> Associated type aliases can declare static parameters.

```ds
struct Pair<T, U> {
    type Swap<V> = Pair<V, T>;
}

declare const value: Pair<int32, string>.Swap<boolean>;
value satisfies Pair<boolean, int32>;
```

### struct generic associated type projection requires static arguments

> Struct associated type projections must provide required generic arguments.
> This should fail with the expected projection shape diagnostic.

```ds
struct Pair<T, U> {
    type Swap<V> = Pair<V, T>;
}

function project<P: Pair<int32, string>>(value: P.Swap): P.Swap {
    return value;
}
```

- contains: argument

### struct associated type parameters can reference outer parameters

> Associated type aliases can reference outer parameters and their own parameters.

```ds
struct Wrapper<T> {
    type With<U> = [T, U];
}

declare const value: Wrapper<string>.With<int32>;
value satisfies [string, int32];
```

### struct associated type can use static value parameters

> Associated type aliases can use comptime static value parameters.
> Static value arguments must remain intact across owner substitution and associated projection

```ds
struct Matrix<comptime rows: uint, comptime cols: uint> {
    type Row = float64[cols];
    type View<comptime r: uint> = float64[r];
}

declare const row: Matrix<4, 4>.Row;
row satisfies float64[4];

declare const view: Matrix<4, 4>.View<2>;
view satisfies float64[2];
```

### struct associated type constraint rejects incompatible defaults

> Associated type defaults must satisfy the declared constraint.
> This should fail with the expected contract compatibility diagnostic.

```ds
struct SizedBox {
    type Item: number = string;
}
```

- contains: not assignable

### struct associated type projection composes across aliases

> Struct associated type projections compose through nested associated aliases.
> Type operators should run after specialization, not on unspecialized placeholders.

```ds
struct Registry<T> {
    type Entry = { value: T };
    type Wrapped<U> = [Entry, U];
}

// projected member should reflect substituted operator results
declare const value: Registry<int32>.Wrapped<boolean>;
value satisfies [{ value: int32 }, boolean];
```

### struct associated type projection enforces static value arguments

> Struct associated type projections require comptime static value arguments.
> Missing value arguments must be rejected before the projection can be instantiated

```ds
struct Matrix<comptime cols: uint> {
    type Row<comptime width: uint> = float64[width];
}

function take<M: Matrix<4>>(value: M.Row): M.Row {
    return value;
}
```

- contains: argument

### struct associated type supports mixed type and value parameters

> Struct associated type projections can combine type and static value parameters.
> Static value arguments must remain intact across owner substitution and associated projection

```ds
struct Buffer<T> {
    type Slice<U, comptime n: uint> = [T, U, n];
}

// projection should preserve owner-scoped substitutions
declare const value: Buffer<int32>.Slice<boolean, 3>;
value satisfies [int32, boolean, 3];
```

### struct associated type projection works on constrained parameters

> Projections are allowed on constrained type parameters.
> Constraint solving should happen before associated projection.

```ds
struct Wrapper<T> {
    type Item = T;
}

function project<W: Wrapper<int32>>(value: W.Item): W.Item {
    return value;
}

declare const value: Wrapper<int32>.Item;
project<Wrapper<int32>>(value) satisfies int32;
```

### struct associated defaults can reference sibling associated types

> Struct associated type defaults can reference sibling associated types.

```ds
struct PairBox<T> {
    type Item = T;
    type Pair<U> = [Item, U];
}

declare const value: PairBox<boolean>.Pair<int32>;
value satisfies [boolean, int32];
```

### struct generic associated defaults can reference sibling generic aliases

> Struct generic associated aliases can compose through sibling generic aliases.

```ds
struct PairBox<T> {
    type Entry<V> = { left: T, right: V };
    type Pair<U> = [Entry<U>, Entry<U>];
}

declare const value: PairBox<boolean>.Pair<int32>;
value satisfies [{ left: boolean, right: int32 }, { left: boolean, right: int32 }];
```

### struct associated projections reject extra static arguments

> Struct associated type projections reject extra static arguments.
> Extra static arguments should be rejected.

```ds
struct PairBox<T> {
    type Item = T;
}

function project<P: PairBox<boolean>>(value: P.Item<int32>): P.Item<int32> {
    return value;
}
```

- contains: argument

### struct associated projections work through type aliases

> Associated type projections are preserved through struct alias indirection.
> Alias indirection must resolve to the owner symbol before associated member substitution

```ds
struct Wrapper<T> {
    type Item = T;
}

type Alias<T> = Wrapper<T>;

declare const value: Alias<string>.Item;
value satisfies string;
```

### struct associated projections resolve across module boundaries

> Struct associated type projections are available through imported modules.

```ds:box.ds
export struct Box<T> {
    type Item = T;
    value: T;
}
```

```ds:main.ds
import { Box } from "./box";

// projection should resolve after module graph resolution
declare const value: Box<int32>.Item;
value satisfies int32;
```

### struct generic associated projections resolve across module boundaries

> Imported struct projections preserve outer and member substitutions.

```ds:pair.ds
export struct Pair<T> {
    type Wrap<U> = [T, U];
    value: T;
}
```

```ds:main.ds
import { Pair } from "./pair";

// imported owner and member substitutions should both materialize
declare const value: Pair<string>.Wrap<int32>;
value satisfies [string, int32];
```

### struct mixed generic associated projections resolve across module boundaries

> Imported struct projections preserve mixed type and value substitutions.

```ds:grid.ds
export struct Grid<T> {
    type Cell<U, comptime n: uint> = [T, U, n];
}
```

```ds:main.ds
import { Grid } from "./grid";

// mixed type and value substitutions should remain stable after import
declare const value: Grid<string>.Cell<int32, 2>;
value satisfies [string, int32, 2];
```

### struct associated projections work on constrained generic parameters

> Generic constraints can project struct associated types with mixed static arguments.
> Constraint solving should happen before associated projection.

```ds
struct Grid<T> {
    type Cell<U, comptime n: uint> = [T, U, n];
}

function project<G: Grid<string>>(value: G.Cell<int32, 2>): G.Cell<int32, 2> {
    value
}

// constrained projections should preserve mixed substitutions in signatures
declare const value: [string, int32, 2];
project(value) satisfies [string, int32, 2];
```

### struct associated defaults support multiple static value parameters

> Struct associated defaults can combine multiple static value parameters.
> Static value arguments must remain intact across owner substitution and associated projection

```ds
struct Matrix<T> {
    type Cell<comptime row: uint, comptime column: uint> = [T, row, column];
}

declare const cell: Matrix<float64>.Cell<1, 2>;
cell satisfies [float64, 1, 2];
```

### struct associated projections use inherited interface generic defaults

> Struct implementors inherit generic associated defaults from interfaces.
> When there is no override, the inherited default should be used.

```ds
interface Projected<T> {
    type View<U> = [T, U];
}

struct Buffer<T> {
    value: T;
}

extension<T> of Buffer<T> implements Projected<T> {}

// inherited contracts should apply before projection
declare const value: Buffer<int32>.View<boolean>;
value satisfies [int32, boolean];
```

### struct associated types can use conditional type operators

> Struct associated type aliases can evaluate conditional type operators with outer substitutions.
> Type operators should run after specialization, not on unspecialized placeholders.

```ds
struct Box<T> {
    type Item = T extends string ? int32 : int16;
    value: T;
}

// projected member should reflect substituted operator results
declare const text: Box<string>.Item;
text satisfies int32;

// projected member should reflect substituted operator results
declare const flag: Box<boolean>.Item;
flag satisfies int16;
```

### struct associated types can use mapped type operators

> Struct associated type aliases can evaluate mapped type operators over outer substitutions.
> Type operators should run after specialization, not on unspecialized placeholders.

```ds
struct Project<T> {
    type Shape = { [K in keyof T]: T[K] };
    value: T;
}

// projected member should reflect substituted operator results
declare const shape: Project<{ left: int32, right: string }>.Shape;
shape satisfies { left: int32, right: string };
```

### struct associated types are not static expressions for comptime value arguments

> Projected struct associated types are not yet valid static value expressions.
> Type projections cannot be consumed as static value expressions in comptime argument positions

```ds
type Bytes<comptime n: number> = uint8[n];

struct BufferShape<T> {
    type Length = T extends string ? 8 : 12;
    type Buffer = Bytes<Length>;
    value: T;
}

declare const short: BufferShape<string>.Buffer;
short satisfies uint8[8];

declare const wide: BufferShape<boolean>.Buffer;
wide satisfies uint8[12];
```

- static argument must be a static expression

### struct associated types are not runtime members

> Associated type aliases are type only and cannot be accessed as runtime values.
> Type-space and runtime member boundaries should stay strict.

```ds
struct Box<T> {
    type Item = T;
    value: T;
}

const value = Box.Item;
```

- property 'Item' does not exist on type { new <T>(T): Box<T> } & type Box