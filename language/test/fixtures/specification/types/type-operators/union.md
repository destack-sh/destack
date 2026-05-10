# Union Types

Union type assignability.

## unions

### unions accept any member

```ds
let x: string | number = "hello";
x = 42;
```

### members assign to unions

```ds
const x: string | number = "hello";
```

## union assignability

### declared members assign to unions

```ds
const x: string = "hello";
const y: string | number = x;
```

## union normalization

### union flattening through aliases

```ds
type A = { a: number } | { b: string };
type B = A | { c: boolean };
const value: B = { c: true };
value satisfies { a: number } | { b: string } | { c: boolean };
```

### union removes never

```ds
type A = never | string;
const value: A = "hello";
value satisfies string;
```
