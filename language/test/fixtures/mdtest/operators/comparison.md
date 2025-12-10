# Comparison Operators

Tests for comparison binary operators.

## Equality

### number equals number

> Comparing two numbers with == produces a boolean.

```ds
const x = 1 == 1;
x satisfies true;
```

### string equals string

> Comparing two strings with == produces a boolean.

```ds
const x = "a" == "a";
x satisfies boolean;
```

### strict equality

> Strict equality === compares without type coercion.

```ds
const x = 1 === 1;
x satisfies true;
```

## Inequality

### not equal

> Not equal != checks if values are different.

```ds
const x = 1 != 2;
x satisfies true;
```

### strict not equal

> Strict not equal !== compares without type coercion.

```ds
const x = 1 !== 2;
x satisfies true;
```

## Relational

### less than

> Less than < compares numeric values.

```ds
const x = 1 < 2;
x satisfies true;
```

### greater than

> Greater than > compares numeric values.

```ds
const x = 2 > 1;
x satisfies true;
```

### less than or equal

> Less than or equal <= includes equality.

```ds
const x = 1 <= 2;
x satisfies true;
```

### greater than or equal

> Greater than or equal >= includes equality.

```ds
const x = 2 >= 1;
x satisfies true;
```
