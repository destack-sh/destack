# Extract Variable

## Local Expression

### Extracts a return expression into a local constant

Extract should insert a const binding before the owning statement and replace the expression.

```ds:main.ds
function total(a: int32, b: int32): int32 {
    return a + b;
    //     ^^^^^ selection
}
```

```query extract_variable selection sum
```

```expected:main.ds
function total(a: int32, b: int32): int32 {
    const sum = a + b;
    return sum;
}
```

## Statement Scope

### Extracts call arguments within the same statement scope

Extract should keep the declaration close to the selected expression usage.

```ds:main.ds
function main(bar: int32, baz: int32): void {
    log(bar + baz);
    //  ^^^^^^^^^ selection
}
```

```query extract_variable selection total
```

```expected:main.ds
function main(bar: int32, baz: int32): void {
    const total = bar + baz;
    log(total);
}
```

## Top Level

### Extracts top-level expressions

Extract should work at the module scope too.

```ds:main.ds
const value = 1 + 2;
//            ^^^^^ selection
```

```query extract_variable selection computed
```

```expected:main.ds
const computed = 1 + 2;
const value = computed;
```

### Extracts local initializer expressions

Extract should insert a const binding in the same local scope before the rewritten statement.

```ds:main.ds
function main(): void {
    const value = 1 + 2;
    //            ^^^^^ selection
}
```

```query extract_variable selection computed
```

```expected:main.ds
function main(): void {
    const computed = 1 + 2;
    const value = computed;
}
```

## Invalid Name

### Rejects invalid extracted names

Extract should fail when the requested variable name is not an identifier.

```ds:main.ds
const value = 1 + 2;
//            ^^^^^ selection
```

```query extract_variable selection bad-name
<none>
```

## No-Op Guard

### Rejects extraction when the expression already matches the new name

Extract should fail when the selected expression is already the requested identifier.

```ds:main.ds
const extracted = 1;
const value = extracted;
//            ^^^^^^^^^ selection
```

```query extract_variable selection extracted
<none>
```
