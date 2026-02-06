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

```ds
struct SizedBox {
    type Item: number = string;
}
```

- contains: not assignable

### struct associated type projection composes across aliases

> Struct associated type projections compose through nested associated aliases.

```ds
struct Registry<T> {
    type Entry = { value: T };
    type Wrapped<U> = [Entry, U];
}

declare const value: Registry<int32>.Wrapped<boolean>;
value satisfies [{ value: int32 }, boolean];
```

### struct associated type projection enforces static value arguments

> Struct associated type projections require comptime static value arguments.

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

```ds
struct Buffer<T> {
    type Slice<U, comptime n: uint> = [T, U, n];
}

declare const value: Buffer<int32>.Slice<boolean, 3>;
value satisfies [int32, boolean, 3];
```

### struct associated type projection works on constrained parameters

> Projections are allowed on constrained type parameters.

```ds
struct Wrapper<T> {
    type Item = T;
}

function project<W: Wrapper<int32>>(value: W.Item): W.Item {
    return value;
}
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

declare const value: Pair<string>.Wrap<int32>;
value satisfies [string, int32];
```

### struct associated defaults support multiple static value parameters

> Struct associated defaults can combine multiple static value parameters.

```ds
struct Matrix<T> {
    type Cell<comptime row: uint, comptime column: uint> = [T, row, column];
}

declare const cell: Matrix<float64>.Cell<1, 2>;
cell satisfies [float64, 1, 2];
```

### struct associated projections use inherited interface generic defaults

> Struct implementors inherit generic associated defaults from interfaces.

```ds
interface Projected<T> {
    type View<U> = [T, U];
}

struct Buffer<T> {
    value: T;
}

extension<T> for Buffer<T> implements Projected<T> {}

declare const value: Buffer<int32>.View<boolean>;
value satisfies [int32, boolean];
```
