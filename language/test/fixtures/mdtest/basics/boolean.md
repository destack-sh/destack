# Boolean Literals

Tests for boolean literal type inference and checking.

## Basic Booleans

### true literal

> True can be assigned to boolean type.

```ds
const x: boolean = true
```

### false literal

> False can be assigned to boolean type.

```ds
const x: boolean = false
```

## Inference

### inferred true type

> True literals without annotation infer to boolean.

```ds
const x = true
```

### inferred false type

> False literals without annotation infer to boolean.

```ds
const x = false
```

## Type Mismatches

### boolean assigned to string

> Boolean literals cannot be assigned to string type.

```ds
const x: string = true
```

- type true is not assignable to type string

### boolean assigned to number

> Boolean literals cannot be assigned to number type.

```ds
const x: number = false
```

- type false is not assignable to type number
