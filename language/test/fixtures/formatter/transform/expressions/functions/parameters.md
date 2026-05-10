# Function Parameters

## Parameters

### optional parameter

Optional parameters use `?` after the parameter name.

```ds
function greet(name?: string) { return `Hello, ${name ?? "world"}` }
```

```ds expected
function greet(name?: string) {
    return `Hello, ${name ?? "world"}`;
}
```

### default parameter

Default values use `= value` after the type annotation.

```ds
function greet(name: string = "world") { return `Hello, ${name}` }
```

```ds expected
function greet(name: string = "world") {
    return `Hello, ${name}`;
}
```

### rest parameter

Rest parameters use `...` prefix and must be the last parameter.

```ds
function sum(...numbers: number[]): number { return numbers.reduce((a, b) => a + b, 0) }
```

```ds expected
function sum(...numbers: number[]): number {
    return numbers.reduce((a, b) => a + b, 0);
}
```

### function with this parameter

Explicit `this` parameters stay first in the list.

```ts:main.ts
function bind(this: Handler, event: Event) { this.handle(event) }
```

```ts expected
function bind(this: Handler, event: Event) {
    this.handle(event);
}
```

### destructured parameter

Object destructuring in parameters preserves the pattern structure.

```ds
function point({ x, y }: Point): string { return `(${x}, ${y})` }
```

```ds expected
function point({ x, y }: Point): string {
    return `(${x}, ${y})`;
}
```

### array destructured parameter

Array destructuring extracts elements by position.

```ds
function first([head]: number[]): number { return head }
```

```ds expected
function first([head]: number[]): number {
    return head;
}
```

### parameter annotations

Annotated parameters keep the `@` prefix before the name.

```ds
function process(@nonempty input: string) { return input }
```

```ds expected
function process(@nonempty input: string) {
    return input;
}
```
