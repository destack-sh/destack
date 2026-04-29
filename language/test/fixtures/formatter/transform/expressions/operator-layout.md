# Operator Layout

Operator layout fixtures cover ternaries, binary chains, mixed precedence, and optional chain compositions.

## Complex Ternaries

### ternary with function calls

Ternary branches containing function calls.

```ds line-width=50
const result = isValid ? processSuccess(data) : handleFailure(error)
```

```ds expected
const result = isValid
    ? processSuccess(data)
    : handleFailure(error);
```

### ternary with objects

Object literals in ternary branches.

```ds line-width=40
const config = isDev ? { debug: true, log: "verbose" } : { debug: false }
```

```ds expected
const config = isDev
    ? { debug: true, log: "verbose" }
    : { debug: false };
```

### ternary in function argument

Ternary expressions as function arguments.

```ds
render(loading ? <Spinner /> : <Content data={data} />)
```

```ds expected
render(loading ? <Spinner /> : <Content data={data} />);
```

## Binary Expression Chains

### mixed operators same precedence

Operators at the same precedence level flatten together when they don't fit.

```ds line-width=20
const x = a + b + c + d + e
```

```ds expected
const x =
    a +
    b +
    c +
    d +
    e;
```

### logical chain with calls

Logical operators with function call operands.

```ds line-width=40
const valid = isActive() && hasPermission() && !isBlocked()
```

```ds expected
const valid =
    isActive() &&
    hasPermission() &&
    !isBlocked();
```

### comparison chain

Comparison expressions format cleanly.

```ds
const inRange = value >= min && value <= max
```

```ds expected
const inRange = value >= min && value <= max;
```

## Mixed Operators

### arithmetic with different precedence

Mixed arithmetic operators respect precedence.

```ds
const x = a + b * c - d / e
```

```ds expected
const x = a + b * c - d / e;
```

### logical with comparison

Logical operators with comparisons.

```ds
const valid = x > 0 && x < 100 || y === 0
```

```ds expected
const valid = (x > 0 && x < 100) || y === 0;
```

### long mixed expression breaks

Long expressions with mixed operators break appropriately.

```ds line-width=30
const x = veryLongA + veryLongB * veryLongC
```

```ds expected
const x =
    veryLongA +
    veryLongB * veryLongC;
```

## Optional Chain Composition

### optional chain with nullish coalescing

Optional chaining combined with nullish coalescing.

```ds
const name = user?.profile?.name ?? "Anonymous"
```

```ds expected
const name = user?.profile?.name ?? "Anonymous";
```

### optional chain with method call

Optional chaining with method invocation.

```ds
const result = obj?.method?.(arg1, arg2)
```

```ds expected
const result = obj?.method?.(arg1, arg2);
```

### complex optional access

Multiple optional accesses and calls.

```ds
data?.items?.[0]?.value?.toString()
```

```ds expected
data?.items?.[0]?.value?.toString();
```

## Long Binary Chains

### many additions

Long chain of additions.

```ds line-width=30
const sum = a + b + c + d + e + f + g
```

```ds expected
const sum =
    a + b + c + d + e + f + g;
```

### mixed logical operators

Chain of mixed && and || operators.

```ds line-width=40
const ok = a && b || c && d || e && f
```

```ds expected
const ok =
    (a && b) || (c && d) || (e && f);
```

### nullish chain

Multiple nullish coalescing operators.

```ds line-width=50
const value = first ?? second ?? third ?? fourth ?? fallback
```

```ds expected
const value =
    first ??
    second ??
    third ??
    fourth ??
    fallback;
```

### comparison chain with logical

Comparison operators combined with logical.

```ds line-width=35
const inBounds = x >= 0 && x < width && y >= 0 && y < height
```

```ds expected
const inBounds =
    x >= 0 &&
    x < width &&
    y >= 0 &&
    y < height;
```
