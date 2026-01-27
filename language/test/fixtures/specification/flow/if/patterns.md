# If Let Patterns

## Tuple Patterns

### if let tuple patterns bind positional values

> Tuple patterns bind positional values in the then branch.

```ds
declare const pair: (int32, int32);

if let (left, right) = pair {
    left satisfies int32;
    right satisfies int32;
}
```

### if let tuple patterns allow ignored bindings

> Tuple patterns allow ignored bindings with `_`.

```ds
declare const pair: (int32, int32);

if let (left, _) = pair {
    left satisfies int32;
}
```

### if let single field tuple patterns bind the first element type

> Single field tuple patterns should bind the first tuple element type.

```ds
declare const pair: (int32, string);

if let (only) = pair {
    only satisfies int32;
}
```

## Object Patterns

### if let object patterns bind named fields

> Object patterns bind named fields in the then branch.

```ds
type Point = { x: int32, y: int32 };
declare const point: Point;

if let { x, y } = point {
    x satisfies int32;
    y satisfies int32;
}
```

### if let object patterns bind aliases to field types

> Object pattern aliases use the field type of the matched value.

```ds
type Point = { x: int32, y: int32 };
declare const point: Point;

if let { x: left, y } = point {
    left satisfies int32;
    y satisfies int32;
}
```

## Tagged Object Patterns

### if let tagged object patterns bind tagged fields

> Tagged object patterns bind field values in the then branch.

```ds
struct Point { x: int32, y: int32 }
declare const point: Point;

if let Point { x, y } = point {
    x satisfies int32;
    y satisfies int32;
}
```

### if let object patterns reject structs

> Untagged object patterns do not match nominal structs.

```ds
struct Point { x: int32, y: int32 }
declare const point: Point;

if let { x, y } = point {
    x satisfies int32;
    y satisfies int32;
}
```

- contains: not assignable

### if let tagged object patterns accept type aliases

> Tagged object patterns allow type aliases as tags.

```ds
type Point = { x: int32, y: int32 };
declare const point: Point;

if let Point { x, y } = point {
    x satisfies int32;
    y satisfies int32;
}
```

## Newtype Patterns

### if let newtype patterns bind inner values

> Newtype patterns bind the underlying value in the then branch.

```ds
newtype UserId = int64;
declare const id: UserId;

if let UserId(value) = id {
    value satisfies int64;
}
```

### if let scalar newtype patterns reject multi field sequence patterns

> Scalar newtypes should not accept multi field sequence patterns.

```ds
newtype UserId = int64;
declare const id: UserId;

if let UserId(first, second) = id {
}
```

- contains: not assignable

### if let newtype object patterns require tags

> Object newtypes require tagged patterns.

```ds
newtype Config = { debug: boolean };
declare const config: Config;

if let Config { debug } = config {
    debug satisfies boolean;
}
```

### if let newtype object patterns reject untagged objects

> Untagged object patterns do not match object newtypes.

```ds
newtype Config = { debug: boolean };
declare const config: Config;

if let { debug } = config {
    debug satisfies boolean;
}
```

- contains: expected boolean

## Binding Patterns

### if let binding patterns introduce names

> Binding patterns introduce names in the then branch.

```ds
declare const value: int32;

if let bound = value {
    bound satisfies int32;
}
```

## Wildcard Patterns

### if let wildcard patterns accept any value

> Wildcard patterns match any value without introducing a binding.

```ds
declare const value: int32;

if let _ = value {
    value satisfies int32;
}
```

## Union Patterns

### if let union patterns narrow in both branches

> Union patterns narrow the matched value in then and else branches.

```ds
declare const value: 1 | 2 | 3;

if let 1 | 2 = value {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}
```
