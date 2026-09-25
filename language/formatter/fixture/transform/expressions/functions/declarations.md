# Function Declarations

## Function Forms

### function declaration

Extra whitespace in the function signature should be removed.

```tspp
function   foo  (  )   {   }
```

Empty function bodies stay on one line with a space inside the braces.

```tspp expected
function foo() {}
```

### function with parameters

Parameter spacing should be normalized with no space after `(` or before `)`.

```tspp
function   foo  (  x  :  number  ,  y  :  string  )   {   }
```

```tspp expected
function foo(x: number, y: string) {}
```

### function with return type

Functions with statements in the body get their body broken to multiple lines.

```tspp
function   foo  (  )  :  number   {  return 1  }
```

```tspp expected
function foo(): number {
    return 1;
}
```

### function with body

Return statements cause the function body to expand to multiple lines.

```tspp
function add(a: number, b: number): number { return a + b }
```

```tspp expected
function add(a: number, b: number): number {
    return a + b;
}
```

### function with multiple statements

Each statement goes on its own line with stable indentation.

```tspp
function process(x: number) { const y = x * 2; const z = y + 1; return z; }
```

```tspp expected
function process(x: number) {
    const y = x * 2;
    const z = y + 1;
    return z;
}
```

### short value function with nested tail expression

Short value-returning functions expand nested control-flow tails.

```tspp
function score(value: number): number { const base = value * 2; if (base > 10) { base } else { base + 1 } }
```

```tspp expected
function score(value: number): number {
    const base = value * 2;
    if (base > 10) {
        base
    } else {
        base + 1
    }
}
```

### expanded value function with nested tail expression

Value-returning functions preserve expression tails through nested blocks.

```tspp
function score(value: number): number { const base = value * 2; if (base > 10) { const capped = base - 1; capped } else { const boosted = base + 1; boosted } }
```

```tspp expected
function score(value: number): number {
    const base = value * 2;
    if (base > 10) {
        const capped = base - 1;
        capped
    } else {
        const boosted = base + 1;
        boosted
    }
}
```

### void function with nested if tail

Void functions keep nested if branch tails semicolonless unless the semicolon was explicit.

```tspp
function score(value: number): void { const base = value * 2; if (base > 10) { report(base) } else { report(base + 1) } }
```

```tspp expected
function score(value: number): void {
    const base = value * 2;
    if (base > 10) {
        report(base)
    } else {
        report(base + 1)
    }
}
```

### value function with match tail expression

Match expressions in function tail position keep arm values.

```tspp
function label(status: Status): string { const normalized = status.normalize(); match (normalized) { Ready => "ready"; Waiting => "waiting"; Failed(error) => error.message } }
```

```tspp expected
function label(status: Status): string {
    const normalized = status.normalize();
    match (normalized) {
        Ready => "ready"
        Waiting => "waiting"
        Failed(error) => error.message
    }
}
```

## Export and Visibility

### exported function

The `export` keyword precedes the function declaration.

```tspp
export function foo() { }
```

```tspp expected
export function foo() {}
```

### export default function

Default exports use `export default` before the function.

```tspp
export default function handler() { }
```

```tspp expected
export default function handler() {}
```

## Overloads

### function overloads

Overload signatures are listed before the implementation signature.

```tspp
function parse(x: string): number
function parse(x: number): number
function parse(x: string | number): number { return x is string ? parseInt(x) : x }
```

```tspp expected
function parse(x: string): number;
function parse(x: number): number;
function parse(x: string | number): number {
    return x is string ? parseInt(x) : x;
}
```
