# Newtype Construction

Newtypes use call syntax for explicit construction.

## calls

### scalar newtypes use one argument

```ds
newtype UserId = int64;

const id = UserId(42);
id satisfies UserId;
```

### tuple newtypes use positional arguments

```ds
newtype Point = (float32, float32);

const point = Point(1.0, 2.0);
point satisfies Point;
```

### object newtypes use one object argument

```ds
newtype Config = { debug: boolean };

const config = Config({ debug: true });
config satisfies Config;
```

### object literals do not construct newtypes

```ds
newtype Config = { debug: boolean };

const config: Config = { debug: true };
```

- contains: is not assignable
