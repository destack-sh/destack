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

### union literal equals union

> Literal equality allows comparisons when the literal is assignable to the union.

```ds
function isReady(value: true | { value: int32 }): boolean {
    return value == true;
}

const result = isReady(true);
result satisfies boolean;
```

### strict equality

> Strict equality === compares without type coercion.

```ds
const x = 1 === 1;
x satisfies true;
```

### strict equality rejects structs

> Strict equality requires reference identity types.

```ds
struct Point {
    x: int32
}

const left = Point { x: 1 };
const right = Point { x: 1 };
const value = left === right;
```

- strict equality not supported

### strict equality allows classes

> Strict equality is allowed for identity types.

```ds
class User {
    name: string = ""
}

const left = new User();
const right = new User();
const value = left === right;
value satisfies boolean;
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
