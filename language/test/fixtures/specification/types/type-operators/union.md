# Union Types

Union type assignability.

## unions

### unions accept any member

A union place holds any arm.

```ds
let x: string | number = "hello";
x = 42;
```

### members assign to unions

Arms flow into the union.

```ds
const x: string | number = "hello";
```

## union assignability

### declared members assign to unions

Declared arm types flow the same way.

```ds
const x: string = "hello";
const y: string | number = x;
```

## union normalization

### union flattening through aliases

Nested unions flatten.

```ds
type A = { a: number } | { b: string };
type B = A | { c: boolean };
const value: B = { c: true };
value satisfies { a: number } | { b: string } | { c: boolean };
```

### union removes never

`never` contributes nothing.

```ds
type A = never | string;
const value: A = "hello";
value satisfies string;
```
