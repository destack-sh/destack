# Tuple Patterns

## tuple patterns

### tuple patterns destructure values

Tuple patterns bind tuple elements.

```ds
let (left, right) = (1, 2);
left satisfies number;
right satisfies number;
```

### wildcard tuple patterns ignore values

Wildcards discard tuple elements.

```ds
let (_, value) = (1, 2);
value satisfies number;
```

### tuple patterns reject arity mismatch

Tuple patterns require enough source elements for each binding.

```ds
let (left, right, extra) = (1, 2);
```

- contains: not assignable

### tuple patterns nest

Tuple patterns destructure nested tuple elements positionally.

```ds
let (left, (middle, right)) = (1, (2, 3));
left satisfies number;
middle satisfies number;
right satisfies number;
```

## defaults

### tuple defaults fill undefined positions

Defaults are used when a matched position is undefined.

```ds
let (left, right = 2) = (1, undefined);
left satisfies number;
right satisfies number;
```

## rest

### tuple rest binds tails

Rest patterns collect remaining tuple positions.

```ds
let (head, ...tail) = (1, "two", true);
head satisfies number;
tail satisfies (string, boolean);
```
