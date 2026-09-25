# Arrow Function Types

## Typed Arrow Functions

### arrow function with parameter types

Type annotations follow parameter names with colon.

```tspp
const f = (x: number) => x * 2
```

```tspp expected
const f = (x: number) => x * 2;
```

### arrow function with return type

Return types appear after the parameter list.

```tspp
const f = (x: number): number => x * 2
```

```tspp expected
const f = (x: number): number => x * 2;
```

### arrow function with complex types

Union types and multiple typed parameters stay in the arrow head.

```tspp:main.tspp
const f = (a: string, b: number): string | number => a || b
```

```tspp expected
const f = (a: string, b: number): string | number => a || b;
```
