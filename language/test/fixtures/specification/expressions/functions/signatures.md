# Function Signatures

Functions declare parameters, return types, and declaration-only signatures.

## declarations

### functions can have no parameters

Functions can be declared with no parameters.

```ds
function f(): void {}
```

### functions can have parameters

Functions can have typed parameters.

```ds
function add(a: number, b: number): void {
    a satisfies number;
    b satisfies number;
}
```

### functions can have return types

Functions specify their return type after the parameter list.

```ds
function greet(name: string): string {
    name satisfies string;
    let result = "hello " + name;
    result satisfies string;
    return result;
}
```

### declared async functions are allowed

Declared function signatures can use async markers.

```ds
declare async function load(): Promise<void>
```

### declared generator functions are allowed

Declared function signatures can use generator markers.

```ds
declare function* ids(): Generator<int32, void, unknown>
```

### declared functions cannot have bodies

Declared functions cannot include bodies.

```ds
declare function greet(): string {
    return "hi";
}
```

- contains: invalid function

## overloads

### declaration file overloads merge

Overloads in declaration files merge into a single callable.

```ds:types.ds
export function apply(value: string): number;
export function apply(value: number): string;
```

```ds:main.ds
import { apply } from "./types.ds";

apply("ok") satisfies number;
apply(42) satisfies string;
```
