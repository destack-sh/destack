# Rename Re-Exports

## Exported Symbols

### Rename through re-exports

Renaming a symbol should update its definition, re-exports, imports, and uses.

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^ target
    return "Hello, " + name;
}
```

```ds:barrel.ds
export { greet } from "./lib.ds";
```

```ds:main.ds
import { greet } from "./barrel.ds";

const message = greet("Destack");
```

```query rename target "welcome"
```

```expected:lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:barrel
export { welcome } from "./lib.ds";
```

```expected:main
import { welcome } from "./barrel.ds";

const message = welcome("Destack");
```
