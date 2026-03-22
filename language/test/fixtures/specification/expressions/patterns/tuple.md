# Tuple Patterns

## tuple patterns

### tuple patterns destructure values

> Tuple patterns bind tuple elements.

```ds
let (left, right) = (1, 2);
left satisfies int32;
right satisfies int32;
```

### wildcard tuple patterns ignore values

> Wildcards discard tuple elements.

```ds
let (_, value) = (1, 2);
value satisfies int32;
```

### tuple patterns cannot use named fields in TypeScript

> Named fields are not allowed in tuple patterns in TypeScript.

```ts:main.ts
let (x: y) = (1, 2);
```

- named fields are not allowed in array or tuple patterns

### tuple patterns reject arity mismatch

> Tuple patterns require enough source elements for each binding.

```ds
let (left, right, extra) = (1, 2);
```

- not assignable

### tuple patterns support nested destructuring

> Tuple patterns destructure nested tuple elements positionally.

```ds
let (left, (middle, right)) = (1, (2, 3));
left satisfies int32;
middle satisfies int32;
right satisfies int32;
```
