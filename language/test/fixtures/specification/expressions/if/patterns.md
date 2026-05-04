# If Let Patterns

If let patterns supports tuple, fixed array, ownership, and union forms.

## Tuple patterns

### if let tuple patterns bind tuple elements

> Tuple patterns in if let bind tuple element types.

```ds
declare const pair: (int32, int32);

if let (left, right) = pair {
    left satisfies int32;
    right satisfies int32;
}
```

## Fixed arrays

### if let array patterns bind fixed array elements

> Fixed array patterns in if let bind element types.

```ds
declare const pair: [int32; 2];

if let [left, right] = pair {
    left satisfies int32;
    right satisfies int32;
}
```

## Rest patterns

### if let rest tuple patterns bind remaining elements

> Rest tuple patterns bind the remaining elements as a tuple.

```ds
declare const values: (int32, int32, int32);

if let (first, ...rest) = values {
    first satisfies int32;
    rest satisfies (int32, int32);
}
```

### if let rest array patterns bind remaining elements

> Rest array patterns bind the remaining elements as an array.

```ds
declare const values: [int32; 3];

if let [first, ...rest] = values {
    first satisfies int32;
    rest satisfies int32[];
}
```

### if let rest object patterns bind remaining properties

> Rest object patterns bind the remaining properties.

```ds
type Config = { enabled: boolean, retries: int32 };

declare const config: Config;

if let { enabled, ...rest } = config {
    enabled satisfies boolean;
    rest satisfies { retries: int32 };
}
```

## Must patterns

### if let must patterns unwrap non nullish values

> Must patterns unwrap non nullish values in the then branch.

```ds
declare const value: int32 | null;

if let x! = value {
    x satisfies int32;
} else {
    value satisfies null | undefined;
}
```

## Ownership patterns

### if let value patterns bind owned values

> Value patterns in if let bind owned values without unwrapping.

```ds
declare const value: ^int32;

if let ^x = value {
    x satisfies ^int32;
}
```

### if let reference patterns bind references

> Reference patterns in if let bind references without unwrapping.

```ds
declare const value: &int32;

if let &x = value {
    x satisfies &int32;
}
```

## Newtype patterns

### if let scalar newtype patterns bind inner values

> Scalar newtype patterns bind the underlying value.

```ds
newtype UserId = int64;

declare const id: UserId;

if let UserId(value) = id {
    value satisfies int64;
}
```

### if let tuple newtype patterns bind positional values

> Tuple newtype patterns bind the underlying tuple values.

```ds
newtype Point = (float32, float32);

declare const point: Point;

if let Point(x, y) = point {
    x satisfies float32;
    y satisfies float32;
}
```

### if let scalar newtype patterns accept positional defaults

> Scalar newtype tuple patterns can assign defaults to positional bindings.

```ds
newtype UserId = int64;

declare const id: UserId;

if let UserId(value = 1) = id {
    value satisfies int64;
}
```

### if let scalar newtype positional defaults are analyzed

> Positional defaults in scalar newtype tuple patterns are analyzed as expressions.

```ds
newtype UserId = int64;

declare const id: UserId;

if let UserId(value = missing_default) = id {
    value;
}
```

- contains: missing symbol


### if let object newtype patterns require tags

> Object newtypes require tagged object patterns.

```ds
newtype Config = { debug: boolean };

declare const config: Config;

if let Config { debug } = config {
    debug satisfies boolean;
}
```

### if let object newtype patterns reject untagged objects

> Untagged object patterns do not match object newtypes.

```ds
newtype Config = { debug: boolean };

declare const config: Config;

if let { debug } = config {
    debug
}
```

- contains: not assignable

## Struct patterns

### if let struct patterns require tags

> Struct patterns must use the type tag.

```ds
struct Point {
    x: int32
    y: int32
}

declare const point: Point;

if let Point { x, y } = point {
    x satisfies int32;
    y satisfies int32;
}
```

### if let struct patterns reject bare object patterns

> Bare object patterns do not match nominal structs.

```ds
struct Point {
    x: int32
    y: int32
}

declare const point: Point;

if let { x, y } = point {
    x
}
```

- contains: not assignable

## Enum patterns

### if let enum patterns narrow to variants

> Enum patterns in if let narrow to the matched variant.

```ds
enum State {
    Ready
    Failed
}

declare const state: State;

if let State.Ready = state {
    state satisfies State.Ready;
} else {
    state satisfies State.Failed;
}
```

## Union patterns

### if let union patterns narrow to covered literals

> Union patterns narrow to the covered literals in the then branch.

```ds
declare const value: 1 | 2 | 3;

if let 1 | 2 = value {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}
```
