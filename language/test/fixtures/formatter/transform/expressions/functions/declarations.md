# Function Declarations

## Function Forms

### function declaration

Extra whitespace in the function signature should be removed.

```ds
function   foo  (  )   {   }
```

Empty function bodies stay on one line with a space inside the braces.

```ds expected
function foo() {}
```

### function with parameters

Parameter spacing should be normalized with no space after `(` or before `)`.

```ds
function   foo  (  x  :  number  ,  y  :  string  )   {   }
```

```ds expected
function foo(x: number, y: string) {}
```

### function with return type

Functions with statements in the body get their body broken to multiple lines.

```ds
function   foo  (  )  :  number   {  return 1  }
```

```ds expected
function foo(): number {
    return 1;
}
```

### function with body

Return statements cause the function body to expand to multiple lines.

```ds
function add(a: number, b: number): number { return a + b }
```

```ds expected
function add(a: number, b: number): number {
    return a + b;
}
```

### function with multiple statements

Each statement goes on its own line with stable indentation.

```ds
function process(x: number) { const y = x * 2; const z = y + 1; return z; }
```

```ds expected
function process(x: number) {
    const y = x * 2;
    const z = y + 1;
    return z;
}
```

### short value function with nested tail expression

Short value-returning functions expand nested control-flow tails.

```ds
function score(value: number): number { const base = value * 2; if (base > 10) { base } else { base + 1 } }
```

```ds expected
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

```ds
function score(value: number): number { const base = value * 2; if (base > 10) { const capped = base - 1; capped } else { const boosted = base + 1; boosted } }
```

```ds expected
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

```ds
function score(value: number): void { const base = value * 2; if (base > 10) { report(base) } else { report(base + 1) } }
```

```ds expected
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

```ds
function label(status: Status): string { const normalized = status.normalize(); match (normalized) { Ready => "ready"; Waiting => "waiting"; Failed(error) => error.message } }
```

```ds expected
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

```ds
export function foo() { }
```

```ds expected
export function foo() {}
```

### export default function

Default exports use `export default` before the function.

```ds
export default function handler() { }
```

```ds expected
export default function handler() {}
```

## Overloads

### function overloads

Overload signatures are listed before the implementation signature.

```ds
function parse(x: string): number
function parse(x: number): number
function parse(x: string | number): number { return typeof x === "string" ? parseInt(x) : x }
```

```ds expected
function parse(x: string): number;
function parse(x: number): number;
function parse(x: string | number): number {
    return typeof x === "string" ? parseInt(x) : x;
}
```
