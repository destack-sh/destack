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

## Type-Only Re-Exports

### Rename through type-only re-exports

Renaming a type should update type-only re-exports and imports.

```ds:types.ds
export type Options = {
//          ^^^^^^^ target
    enabled: boolean,
};
```

```ds:barrel.ds
export type { Options } from "./types.ds";
```

```ds:main.ds
import type { Options } from "./barrel.ds";

const config: Options = { enabled: true };
```

```query rename target "Settings"
```

```expected:types
export type Settings = {
    enabled: boolean,
};
```

```expected:barrel
export type { Settings } from "./types.ds";
```

```expected:main
import type { Settings } from "./barrel.ds";

const config: Settings = { enabled: true };
```

### Rename through aliased re-exports

Renaming a symbol should update the exported name while preserving re-export aliases.

```ds:alias_lib.ds
export function greet(name: string): string {
//              ^^^^^ target:alias
    return "Hello, " + name;
}
```

```ds:alias_barrel.ds
export { greet as hello } from "./alias_lib.ds";
```

```ds:alias_main.ds
import { hello } from "./alias_barrel.ds";

const message = hello("Destack");
```

```query rename target:alias "welcome"
```

```expected:alias_lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:alias_barrel
export { welcome as hello } from "./alias_lib.ds";
```

```expected:alias_main
import { hello } from "./alias_barrel.ds";

const message = hello("Destack");
```
