# Scalars

`number`, `string`, and `boolean` keep their strict scalar behavior.

## number

### number accepts numeric literals

`number` is the default numeric type.

```ds
const value: number = 42;
value satisfies number;
```

### const numeric literals keep exact values

Const bindings keep the literal type.

```ds
const value = 42;
value satisfies 42;
value satisfies number;
value satisfies int;
```

### let numeric literals widen to number

Mutable bindings widen.

```ds
let value = 42;
value satisfies number;
```

### let numeric literals do not keep exact values

The literal type is gone after widening.

```ds
let value = 42;
value satisfies 42;
```

- contains: not assignable

### numeric literals fit integer contexts

Context selects the numeric type.

```ds
const value: int = 42;
value satisfies int;
```

### numeric literals fit float contexts

Context selects float types too.

```ds
const value: float32 = 42;
value satisfies float32;
```

## string

### string accepts string literals

`string` accepts every string literal.

```ds
const value: string = "hello";
value satisfies string;
```

### string literals infer literal strings

Const bindings keep the literal string.

```ds
const value = "hello";
value satisfies "hello";
value satisfies string;
```

## boolean

### boolean accepts boolean literals

`boolean` accepts both literals.

```ds
const value: boolean = true;
value satisfies boolean;
```

### boolean literals infer literal booleans

Const bindings keep the literal boolean.

```ds
const value = true;
value satisfies true;
value satisfies boolean;
```
