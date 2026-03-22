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

### tuple literals reject missing elements for fixed tuple targets

> Tuple literal arity must satisfy the target tuple arity.

```ds
let point: (int32, int32) = (1);
```

- type ("left", 2) is not assignable to type (int32, string)

### tuple literals reject extra elements for fixed tuple targets

> Tuple literal arity must not exceed the target tuple arity.

```ds
let point: (int32, int32) = (1, 2, 3);
```

- type ("left", 2) is not assignable to type (int32, string)

### tuple literals enforce positional element types

> Tuple literals check each element against its positional target type.

```ds
let pair: (int32, string) = ("left", 2);
```

- type ("left", 2) is not assignable to type (int32, string)