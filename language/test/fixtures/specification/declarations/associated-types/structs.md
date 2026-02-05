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

const box = Box { value: "ok" };
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
    type View<comptime r: uint> = float64[r][cols];
}

declare const row: Matrix<4, 4>.Row;
row satisfies float64[4];

declare const view: Matrix<4, 4>.View<2>;
view satisfies float64[2][4];
```

### struct associated type constraint rejects incompatible defaults

> Associated type defaults must satisfy the declared constraint.

```ds
struct SizedBox {
    type Item: number = string;
}
```

- contains: not assignable
