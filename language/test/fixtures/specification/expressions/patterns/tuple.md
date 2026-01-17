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
