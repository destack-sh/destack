# Newtype Declarations

## constructors

### scalar newtype constructor call

> Constructor calls yield the declared newtype.

```ds
newtype UserId = int64;

const id = UserId(42);
id satisfies UserId;
```

### tuple newtype constructor call

> Tuple newtypes use positional constructor arguments.

```ds
newtype Point = (float32, float32);

const point = Point(1.0, 2.0);
point satisfies Point;
```

### tagged object literal constructs struct newtypes

> Tagged object literals yield the declared newtype.

```ds
newtype Config = { debug: boolean };

const config = Config { debug: true };
config satisfies Config;
```

### untagged object literal is not a struct newtype

> Untagged object literals do not satisfy nominal newtype types.

```ds
newtype Config = { debug: boolean };

const config: Config = { debug: true };
```

- contains: is not assignable

## nominal typing

### newtypes remain distinct

> Newtypes do not coerce to each other, even when they share the same backing type.

```ds
newtype UserId = int64;
newtype OrderId = int64;

const id = UserId(42);
const bad: OrderId = id;
```

- contains: is not assignable
