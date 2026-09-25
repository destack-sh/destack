# Operator Wrapping

Operator wrapping fixtures cover ternaries, binary chains, mixed precedence, and optional chain combinations.

## Ternary Branches

### ternary with function calls

Ternary branches containing function calls.

```tspp line-width=50
const result = isValid ? processSuccess(data) : handleFailure(error)
```

```tspp expected
const result = isValid
    ? processSuccess(data)
    : handleFailure(error);
```

### ternary with objects

Object literals in ternary branches.

```tspp line-width=40
const config = isDev ? { debug: true, log: "verbose" } : { debug: false }
```

```tspp expected
const config = isDev
    ? { debug: true, log: "verbose" }
    : { debug: false };
```

### ternary in function argument

Ternary expressions as function arguments.

```tspp
render(loading ? <Spinner /> : <Content data={data} />)
```

```tspp expected
render(loading ? <Spinner /> : <Content data={data} />);
```

## Binary Expression Chains

### mixed operators same precedence

Operators at the same precedence level flatten together when they don't fit.

```tspp line-width=20
const x = a + b + c + d + e
```

```tspp expected
const x = a
    + b
    + c
    + d
    + e;
```

### logical chain with calls

Logical operators with function call operands.

```tspp line-width=40
const valid = isActive() && hasPermission() && !isBlocked()
```

```tspp expected
const valid = isActive()
    && hasPermission()
    && !isBlocked();
```

### comparison chain

Comparison expressions format cleanly.

```tspp
const inRange = value >= min && value <= max
```

```tspp expected
const inRange = value >= min && value <= max;
```

## Mixed Operators

### arithmetic with different precedence

Mixed arithmetic operators respect precedence.

```tspp
const x = a + b * c - d / e
```

```tspp expected
const x = a + b * c - d / e;
```

### logical with comparison

Logical operators with comparisons.

```tspp
const valid = x > 0 && x < 100 || y === 0
```

```tspp expected
const valid = (x > 0 && x < 100) || y === 0;
```

### long mixed expression breaks

Long expressions with mixed operators break appropriately.

```tspp line-width=30
const x = veryLongA + veryLongB * veryLongC
```

```tspp expected
const x = veryLongA
    + veryLongB * veryLongC;
```

## Optional Chains

### optional chain with nullish coalescing

Optional chaining combined with nullish coalescing.

```tspp
const name = user?.profile?.name ?? "Anonymous"
```

```tspp expected
const name = user?.profile?.name ?? "Anonymous";
```

### optional chain with method call

Optional chaining with method invocation.

```tspp
const result = obj?.method?.(arg1, arg2)
```

```tspp expected
const result = obj?.method?.(arg1, arg2);
```

### optional access cascade

Multiple optional accesses and calls.

```tspp
data?.items?.[0]?.value?.toString()
```

```tspp expected
data?.items?.[0]?.value?.toString();
```

## Long Binary Chains

### many additions

Long chain of additions.

```tspp line-width=30
const sum = a + b + c + d + e + f + g
```

```tspp expected
const sum = a
    + b
    + c
    + d
    + e
    + f
    + g;
```

### mixed logical operators

Chain of mixed && and || operators.

```tspp line-width=40
const ok = a && b || c && d || e && f
```

```tspp expected
const ok = (a && b)
    || (c && d)
    || (e && f);
```

### nullish chain

Multiple nullish coalescing operators.

```tspp line-width=50
const value = first ?? second ?? third ?? fourth ?? fallback
```

```tspp expected
const value = first
    ?? second
    ?? third
    ?? fourth
    ?? fallback;
```

### comparison chain with logical

Comparison operators combined with logical.

```tspp line-width=35
const inBounds = x >= 0 && x < width && y >= 0 && y < height
```

```tspp expected
const inBounds = x >= 0
    && x < width
    && y >= 0
    && y < height;
```
