# Newtype Patterns

## scalar newtypes

### scalar newtype patterns bind inner values

> Scalar newtypes can be destructured by their constructor name.

```ds
newtype UserId = int64;

let UserId(value) = UserId(42);
value satisfies int64;
```

## tuple newtypes

### tuple newtype patterns bind positional values

> Tuple newtypes can be destructured positionally.

```ds
newtype Point = (float32, float32);

let Point(x, y) = Point(1.0, 2.0);
x satisfies float32;
y satisfies float32;
```

## object newtypes

### object newtype patterns bind fields

> Object newtypes use tagged object patterns.

```ds
newtype Config = { debug: boolean };

let Config { debug } = Config { debug: true };
debug satisfies boolean;
```
