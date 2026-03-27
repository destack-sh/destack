# Rename Across Modules

## Exported Functions

### Rename exported function across modules

Rename should update export definitions, imports, and call sites.

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^ target
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet } from "./lib.ds";

const message = greet("Destack");
//              ^^^^^ use:greet
```

```query rename target "welcome"
```

```expected:lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import { welcome } from "./lib.ds";

const message = welcome("Destack");
```

### Rename exported function used through an aliased import

Rename should update the exported name while preserving local import aliases.

```ds:alias_lib.ds
export function greet(name: string): string {
//              ^^^^^ target:alias
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greet as importedGreet } from "./alias_lib.ds";

const greet = 1;
const message = importedGreet("Destack");
```

```query rename target:alias "welcome"
```

```expected:alias_lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:alias_main
import { welcome as importedGreet } from "./alias_lib.ds";

const greet = 1;
const message = importedGreet("Destack");
```
