# Boolean Literals

Boolean literal type inference and checking.

## booleans

### true literal

> True can be assigned to boolean type.

```ds
const x: boolean = true;
```

### false literal

> False can be assigned to boolean type.

```ds
const x: boolean = false;
```

## inference

### inferred true type

> True literals without annotation infer to boolean.

```ds
const x = true;
x satisfies boolean;
```

### inferred false type

> False literals without annotation infer to boolean.

```ds
const x = false;
x satisfies boolean;
```

## type mismatches

### boolean assigned to string

> Boolean literals cannot be assigned to string type.

```ds
const x: string = true;
```

- contains: not assignable

### boolean assigned to number

> Boolean literals cannot be assigned to number type.

```ds
const x: number = false;
```

- contains: not assignable

### boolean does not satisfy string

> Boolean literal cannot satisfy string type.

```ds
const x = true;
x satisfies string;
```

- contains: not assignable

## boolean members

### boolean toString resolves

> Boolean literals expose Boolean standard members.


```ds
const value = true;
const text = value.toString();
text satisfies string;
```
