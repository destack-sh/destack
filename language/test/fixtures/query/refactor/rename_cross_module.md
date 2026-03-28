# Rename Across Modules

## Exported Functions

### Rename exported function across modules

Rename should update export definitions, imports, and call sites.

```ds:lib.ds
export function greet(name: string): string {
//                ^^^^^ target
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet } from "./lib.ds";

const message = greet("Destack");
//                ^^^^^ use:greet
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
//                ^^^^^ target:alias
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

## Default Exports

### Rename default exports across modules

Rename should update default export definitions and their default imports.

```ds:default_lib.ds
export default function greet(name: string): string {
//                        ^^^^^ target:default
    return "Hello, " + name;
}
```

```ds:default_main.ds
import greet from "./default_lib.ds";

const message = greet("Destack");
```

```query rename target:default "welcome"
```

```expected:default_lib
export default function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:default_main
import welcome from "./default_lib.ds";

const message = welcome("Destack");
```

## Namespace Imports

### Rename exported symbols used through namespace imports

Rename should update exported names while preserving namespace imports and member access.

```ds:namespace_lib.ds
export function greet(name: string): string {
//                ^^^^^ target:namespace
    return "Hello, " + name;
}
```

```ds:namespace_main.ds
import * as api from "./namespace_lib.ds";

const message = api.greet("Destack");
```

```query rename target:namespace "welcome"
```

```expected:namespace_lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:namespace_main
import * as api from "./namespace_lib.ds";

const message = api.welcome("Destack");
```

## Re-Export Barrels

### Rename exported functions through re-export barrels

Rename should update the source export, barrel export, downstream import, and call site.

```ds:barrel_lib.ds
export function greet(name: string): string {
//                ^^^^^ target:barrel
    return "Hello, " + name;
}
```

```ds:barrel.ds
export { greet } from "./barrel_lib.ds";
```

```ds:barrel_main.ds
import { greet } from "./barrel.ds";

const message = greet("Destack");
```

```query rename target:barrel "welcome"
```

```expected:barrel_lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:barrel
export { welcome } from "./barrel_lib.ds";
```

```expected:barrel_main
import { welcome } from "./barrel.ds";

const message = welcome("Destack");
```

### Rename exported functions through aliased re-export barrels

Rename should update the source export while preserving the downstream alias shape.

```ds:alias_barrel_lib.ds
export function greet(name: string): string {
//                ^^^^^ target:barrel_alias
    return "Hello, " + name;
}
```

```ds:alias_barrel.ds
export { greet as hello } from "./alias_barrel_lib.ds";
```

```ds:alias_barrel_main.ds
import { hello } from "./alias_barrel.ds";

const message = hello("Destack");
```

```query rename target:barrel_alias "welcome"
```

```expected:alias_barrel_lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:alias_barrel
export { welcome as hello } from "./alias_barrel_lib.ds";
```

```expected:alias_barrel_main
import { hello } from "./alias_barrel.ds";

const message = hello("Destack");
```

## Type Space

### Rename exported types through type-only imports

Rename should update exported type names and downstream type-only imports.

```ds:type_lib.ds
export type Settings = {
//            ^^^^^^^^ target:type
    enabled: boolean,
};
```

```ds:type_main.ds
import type { Settings } from "./type_lib.ds";

const config: Settings = { enabled: true };
```

```query rename target:type "Config"
```

```expected:type_lib
export type Config = {
    enabled: boolean,
};
```

```expected:type_main
import type { Config } from "./type_lib.ds";

const config: Config = { enabled: true };
```

### Rename exported types through type-only re-export barrels

Rename should propagate through type-only re-export chains across modules.

```ds:type_barrel_lib.ds
export type Settings = {
//            ^^^^^^^^ target:type_barrel
    enabled: boolean,
};
```

```ds:type_barrel.ds
export type { Settings as AppSettings } from "./type_barrel_lib.ds";
```

```ds:type_barrel_main.ds
import type { AppSettings } from "./type_barrel.ds";

const config: AppSettings = { enabled: true };
```

```query rename target:type_barrel "Config"
```

```expected:type_barrel_lib
export type Config = {
    enabled: boolean,
};
```

```expected:type_barrel
export type { Config as AppSettings } from "./type_barrel_lib.ds";
```

```expected:type_barrel_main
import type { AppSettings } from "./type_barrel.ds";

const config: AppSettings = { enabled: true };
```
