# If Let Patterns

If let uses the same pattern families as bindings and match arms.

## tuple patterns

### if (let ...) tuple patterns bind elements

Tuple patterns bind tuple elements.

```ds
declare const pair: (int32, int32);

if (let (left, right) = pair) {
    left satisfies int32;
    right satisfies int32;
}
```

## fixed arrays

### if (let ...) fixed array patterns bind elements

Fixed array patterns bind element types.

```ds
declare const pair: [int32; 2];

if (let [left, right] = pair) {
    left satisfies int32;
    right satisfies int32;
}
```

## rest patterns

### if (let ...) tuple rest binds tails

Tuple rest patterns bind the tail as a tuple.

```ds
declare const values: (int32, int32, int32);

if (let (first, ...rest) = values) {
    first satisfies int32;
    rest satisfies (int32, int32);
}
```

### if (let ...) fixed array rest binds tails

Fixed array rest patterns bind the tail as a fixed array.

```ds
declare const values: [int32; 3];

if (let [first, ...rest] = values) {
    first satisfies int32;
    rest satisfies [int32; 2];
}
```

### if (let ...) object rest binds tails

Object rest patterns bind the remaining fields.

```ds
type Config = { enabled: boolean; retries: int32 };

declare const config: Config;

if (let { enabled, ...rest } = config) {
    enabled satisfies boolean;
    rest satisfies { retries: int32 };
}
```

## must patterns

### if (let ...) must patterns bind non-nullish values

Must patterns bind the non-nullish value in the then branch.

```ds
declare const value: int32 | null;

if (let x! = value) {
    x satisfies int32;
} else {
    value satisfies null;
}
```

## ownership patterns

### if (let ...) owned patterns bind owned views

Owned patterns bind an owned view.

```ds
declare const value: ^int32;

if (let ^x = value) {
    x satisfies ^int32;
}
```

### if (let ...) borrow patterns bind borrowed views

Borrow patterns bind a borrowed view.

```ds
declare const value: &int32;

if (let &x = value) {
    x satisfies &int32;
}
```

## newtype patterns

### if (let ...) scalar newtype patterns bind values

Scalar newtype patterns bind the wrapped value.

```ds
newtype UserId = int64;

declare const id: UserId;

if (let UserId(value) = id) {
    value satisfies int64;
}
```

### if (let ...) tuple newtype patterns bind fields

Tuple newtype patterns bind positional fields.

```ds
newtype Point = (float32, float32);

declare const point: Point;

if (let Point(x, y) = point) {
    x satisfies float32;
    y satisfies float32;
}
```

### if (let ...) scalar newtype patterns bind defaults

Defaults can be attached to scalar newtype bindings.

```ds
newtype UserId = int64;

declare const id: UserId;

if (let UserId(value = 1) = id) {
    value satisfies int64;
}
```

### if (let ...) newtype defaults are expressions

Defaults in newtype patterns are ordinary expressions.

```ds
newtype UserId = int64;

declare const id: UserId;

if (let UserId(value = missing_default) = id) {
    value;
}
```

- contains: missing symbol


### if (let ...) object newtype patterns unwrap object payloads

Object newtypes use the newtype wrapper pattern around an object pattern.

```ds
newtype Config = { debug: boolean };

declare const config: Config;

if (let Config({ debug }) = config) {
    debug satisfies boolean;
}
```

### if (let ...) object newtype patterns reject bare objects

Untagged object patterns do not match object newtypes.

```ds
newtype Config = { debug: boolean };

declare const config: Config;

if (let { debug } = config) {
    debug
}
```

- contains: not assignable

## struct patterns

### if (let ...) struct patterns need tags

Struct patterns use the type tag.

```ds
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

if (let Point { x, y } = point) {
    x satisfies int32;
    y satisfies int32;
}
```

### if (let ...) struct patterns reject bare objects

Bare object patterns do not match nominal structs.

```ds
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

if (let { x, y } = point) {
    x
}
```

- contains: not assignable

## enum patterns

### if (let ...) enum patterns narrow variants

Enum patterns in if (let ...) narrow to the matched variant.

```ds
enum State {
    Ready,
    Failed,
}

declare const state: State;

if (let State.Ready = state) {
    state satisfies State.Ready;
} else {
    state satisfies State.Failed;
}
```

## union patterns

### if (let ...) union patterns narrow covered literals

Union patterns narrow to the covered literals in the then branch.

```ds
declare const value: 1 | 2 | 3;

if (let 1 | 2 = value) {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}
```
