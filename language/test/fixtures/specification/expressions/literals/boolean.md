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

## Inference

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

## Type Mismatches

### boolean assigned to string

> Boolean literals cannot be assigned to string type.

```ds
const x: string = true;
```

- type true is not assignable to type string

### boolean assigned to number

> Boolean literals cannot be assigned to number type.

```ds
const x: number = false;
```

- type false is not assignable to type number

### boolean does not satisfy string

> Boolean literal cannot satisfy string type.

```ds
const x = true;
x satisfies string;
```

- contains: expected string, found true

## Boolean Members

### boolean toString resolves

> Boolean literals expose Boolean standard members.


```ds libs=es5
const value = true;
const text = value.toString();
text satisfies string;
```
