# Tuple Expressions

## tuple expressions

### tuple literal assigns to tuple type

> Tuple literals can be assigned to tuple types.

```ds
let point: (int32, int32) = (1, 2);
```

### tuple destructuring binds elements

> Tuple patterns bind element values.

```ds
let (left, right) = (1, 2);
left satisfies int32;
right satisfies int32;
```
