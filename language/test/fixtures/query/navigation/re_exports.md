# Re-Export Navigation

## Re-Exported Symbols

### Goto definition through re-exports

Goto definition should resolve symbols through re-export chains.

```ds:base.ds
export function greet(name: string): string {
//              ^^^^^ def:greet
    return "Hello, " + name;
}
```

```ds:reexport.ds
export { greet } from "./base.ds";
```

```ds:main.ds
import { greet } from "./reexport.ds";

const message = greet("Destack");
//              ^^^^^ use:greet
```

```query goto_definition use:greet
def:greet
```

### Goto definition through re-exported aliases

Re-exported aliases should resolve to the original definition.

```ds:alias_base.ds
export function build(name: string): string {
//              ^^^^^ def:build
    return "Hello, " + name;
}
```

```ds:alias_reexport.ds
export { build as buildAlias } from "./alias_base.ds";
```

```ds:alias_main.ds
import { buildAlias } from "./alias_reexport.ds";

const message = buildAlias("Destack");
//              ^^^^^^^^^^ use:build_alias
```

```query goto_definition use:build_alias
def:build
```

### Goto definition through export star chains

Export star chains should resolve to the original definition.

```ds:base_star.ds
export function wave(name: string): string {
//              ^^^^ def:wave
    return "Hi, " + name;
}
```

```ds:barrel_one.ds
export * from "./base_star.ds";
```

```ds:barrel_two.ds
export * from "./barrel_one.ds";
```

```ds:main_star.ds
import { wave } from "./barrel_two.ds";

const message = wave("Destack");
//              ^^^^ use:wave
```

```query goto_definition use:wave
def:wave
```

### Goto definition for default exports

Default imports should resolve to the exported definition.

```ds:base_default.ds
export default function greetDefault(name: string): string {
//                      ^^^^^^^^^^^^ def:greet_default
    return "Hello, " + name;
}
```

```ds:main_default.ds
import greetDefault from "./base_default.ds";

const message = greetDefault("Destack");
//              ^^^^^^^^^^^^ use:greet_default
```

```query goto_definition use:greet_default
def:greet_default
```

### Goto definition through type-only re-export chains

Type-only re-export chains should still resolve to the original type declaration.

```ds:types.ds
export type Config = string;
//          ^^^^^^ def:Config
```

```ds:barrel_a.ds
export type { Config as AppConfig } from "./types.ds";
```

```ds:barrel_b.ds
export type { AppConfig } from "./barrel_a.ds";
```

```ds:main_type.ds
import type { AppConfig } from "./barrel_b.ds";

const config: AppConfig = "ok";
//            ^^^^^^^^^ use:AppConfig
```

```query goto_definition use:AppConfig
def:Config
```
