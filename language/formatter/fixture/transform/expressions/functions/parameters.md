# Function Parameters

## Parameters

### optional parameter

Optional parameters use `?` after the parameter name.

```tspp
function greet(name?: string) { return `Hello, ${name ?? "world"}` }
```

```tspp expected
function greet(name?: string) {
    return `Hello, ${name ?? "world"}`;
}
```

### default parameter

Default values use `= value` after the type annotation.

```tspp
function greet(name: string = "world") { return `Hello, ${name}` }
```

```tspp expected
function greet(name: string = "world") {
    return `Hello, ${name}`;
}
```

### rest parameter

Rest parameters use `...` prefix and must be the last parameter.

```tspp
function sum(...numbers: number[]): number { return numbers.reduce((a, b) => a + b, 0) }
```

```tspp expected
function sum(...numbers: number[]): number {
    return numbers.reduce((a, b) => a + b, 0);
}
```

### function with this parameter

Explicit `this` parameters stay first in the list.

```tspp:main.tspp
function bind(this: Handler, event: Event) { this.handle(event) }
```

```tspp expected
function bind(this: Handler, event: Event) {
    this.handle(event)
}
```

### function with receiver shorthand

Receiver shorthand stays first in the list.

```tspp
function visit(&readonly this, node: Node): void { this.handle(node) }
```

```tspp expected
function visit(&readonly this, node: Node): void {
    this.handle(node);
}
```

### method with exclusive receiver

Exclusive receiver shorthand formats like a normal receiver.

```tspp
extension of Buffer {
push(&exclusive this, value: uint8): void { undefined! }
}
```

```tspp expected
extension of Buffer {
    push(&exclusive this, value: uint8): void {
        undefined!;
    }
}
```

### destructured parameter

Object destructuring in parameters preserves the pattern structure.

```tspp
function point({ x, y }: Point): string { return `(${x}, ${y})` }
```

```tspp expected
function point({ x, y }: Point): string {
    return `(${x}, ${y})`;
}
```

### array destructured parameter

Array destructuring extracts elements by position.

```tspp
function first([head]: number[]): number { return head }
```

```tspp expected
function first([head]: number[]): number {
    return head;
}
```

### parameter annotations

Annotated parameters keep the `@` prefix before the name.

```tspp
function process(@nonempty input: string) { return input }
```

```tspp expected
function process(@nonempty input: string) {
    return input;
}
```
