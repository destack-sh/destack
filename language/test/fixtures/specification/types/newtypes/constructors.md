# Newtype Constructors

Newtype constructors wrap values in the declared nominal type.

## calls

### scalar newtype constructors wrap values

> Constructor calls yield the declared newtype.

```ds
newtype UserId = int64;

const id = UserId(42);
id satisfies UserId;
```

### tuple newtype constructors wrap positional values

> Tuple newtypes use positional constructor arguments.

```ds
newtype Point = (float32, float32);

const point = Point(1.0, 2.0);
point satisfies Point;
```

## objects

### object newtype constructors wrap object values

> Object newtype constructors wrap object literals.

```ds
newtype Config = { debug: boolean };

const config = Config({ debug: true });
config satisfies Config;
```

### object literals do not implicitly construct newtypes

> Untagged object literals do not satisfy nominal newtype types.

```ds
newtype Config = { debug: boolean };

const config: Config = { debug: true };
```

- contains: is not assignable
