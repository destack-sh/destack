# Newtype Patterns

## scalar newtypes

### scalar newtype patterns bind values

Scalar newtype patterns bind the wrapped value.

```ds
newtype UserId = int64;

let UserId(value) = UserId(42);
value satisfies int64;
```

## tuple newtypes

### tuple newtype patterns bind fields

Tuple newtype patterns bind positional fields.

```ds
newtype Point = (float32, float32);

let Point(x, y) = Point(1.0, 2.0);
x satisfies float32;
y satisfies float32;
```

## object newtypes

### object newtype patterns unwrap object payloads

Object newtypes use the newtype wrapper pattern around an object pattern.

```ds
newtype Config = { debug: boolean };

let Config({ debug }) = Config({ debug: true });
debug satisfies boolean;
```
