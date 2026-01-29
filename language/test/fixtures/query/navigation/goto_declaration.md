# Goto Declaration

## Locals

### Local variable declaration

Goto declaration should return the local declaration.

```ds
const value = 1;
//    ^^^^^ decl:value

const other = value;
//            ^^^^ use:value
```

```query goto_declaration use:value
decl:value
```

## Imports

### Imported symbol declaration

Goto declaration should stop at the import declaration.

```ds:lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet } from "./lib.ds";
//       ^^^^^ decl:import_greet

const message = greet("World");
//              ^^^^^ use:greet
```

```query goto_declaration use:greet
decl:import_greet
```

### Re-exported alias declaration

Goto declaration should stop at the local import alias even when the symbol is re-exported.

```ds:alias_base.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_barrel.ds
export { greet as greetAlias } from "./alias_base.ds";
```

```ds:alias_main.ds
import { greetAlias } from "./alias_barrel.ds";
//       ^^^^^^^^^^ decl:import_greet_alias

const message = greetAlias("World");
//              ^^^^^^^^^^ use:greet_alias
```

```query goto_declaration use:greet_alias
decl:import_greet_alias
```

### Type-only import declaration

Goto declaration should stop at the type-only import specifier.

```ds:types.ds
export struct Thing {
    value: int32,
}
```

```ds:main.ds
import type { Thing } from "./types.ds";
//            ^^^^^ decl:import_thing

const item: Thing = Thing { value: 1 };
//          ^^^^^ use:thing
```

```query goto_declaration use:thing
decl:import_thing
```

### Namespace import declaration

Goto declaration should stop at the namespace import alias.

```ds:utils.ds
export function ping(): void {}
```

```ds:main.ds
import * as utils from "./utils.ds";
//     ^^^^^^^^^^ decl:import_utils

utils.ping();
// ^^^^^ use:utils
```

```query goto_declaration use:utils
decl:import_utils
```

### Re-exported import declaration

Goto declaration should stop at the importing site even when the symbol is re-exported.

```ds:lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:bar.ds
export { greet } from "./lib.ds";
```

```ds:main.ds
import { greet } from "./bar.ds";
//       ^^^^^ decl:import_greet

const message = greet("World");
//              ^^^^^ use:greet
```

```query goto_declaration use:greet
decl:import_greet
```
