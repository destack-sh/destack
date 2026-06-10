# Newtype Construction

Newtypes use call syntax for explicit construction.

## calls

### scalar newtypes use one argument

The constructor takes the backing value.

```ds
newtype UserId = int64;

const id = UserId(42);
id satisfies UserId;
```

### tuple newtypes use positional arguments

Tuple backings spread into positions.

```ds
newtype Point = (float32, float32);

const point = Point(1.0, 2.0);
point satisfies Point;
```

### object newtypes use one object argument

Object backings take the object.

```ds
newtype Config = { debug: boolean };

const config = Config({ debug: true });
config satisfies Config;
```

### object literals do not construct newtypes

The constructor is required; structure is not enough.

```ds
newtype Config = { debug: boolean };

const config: Config = { debug: true };
```

- contains: is not assignable
