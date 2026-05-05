# Scalars

`number`, `string`, and `boolean` keep their strict scalar behavior.

## number

### number accepts numeric literals

```ds
const value: number = 42;
value satisfies number;
```

### const numeric literals keep exact values

```ds
const value = 42;
value satisfies 42;
value satisfies number;
value satisfies int;
```

### let numeric literals widen to number

```ds
let value = 42;
value satisfies number;
```

### let numeric literals do not keep exact values

```ds
let value = 42;
value satisfies 42;
```

- contains: not assignable

### numeric literals fit integer contexts

```ds
const value: int = 42;
value satisfies int;
```

### numeric literals fit float contexts

```ds
const value: float32 = 42;
value satisfies float32;
```

## string

### string accepts string literals

```ds
const value: string = "hello";
value satisfies string;
```

### string literals infer literal strings

```ds
const value = "hello";
value satisfies "hello";
value satisfies string;
```

## boolean

### boolean accepts boolean literals

```ds
const value: boolean = true;
value satisfies boolean;
```

### boolean literals infer literal booleans

```ds
const value = true;
value satisfies true;
value satisfies boolean;
```
