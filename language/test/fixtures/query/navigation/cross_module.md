# Cross Module Navigation

## Imported Symbols

### Goto definition across modules

Goto definition should resolve imported symbols across files.

The library module provides the exported symbol.

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^ def:greet
    return "Hello, " + name;
}
```

The main module imports and uses the symbol.

```ds:main.ds
import { greet } from "./lib.ds";

const message = greet("Destack");
//    ^^^^^^^ def:message
//              ^^^^^ use:greet

const local = message;
//            ^^^^^^^ use:message
```

Goto definition from the imported call should land on the exported definition.

```query goto_definition use:greet
def:greet
```

Find references should include the definition, import specifier, and call site.

```query find_references use:greet
lib.ds:1:17-1:22
main.ds:1:10-1:15
main.ds:3:17-3:22
```

Goto definition should still handle local symbols in the same file.

```query goto_definition use:message
def:message
```

## Import Shapes

### Goto definition through aliased imports

Goto definition should still reach the exported definition through a local import alias.

```ds:alias_lib.ds
export function greet(name: string): string {
//              ^^^^^ def:greet
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greet as importedGreet } from "./alias_lib.ds";

const greet = 1;
const message = importedGreet("Destack");
//              ^^^^^^^^^^^^^ use:importedGreet
```

```query goto_definition use:importedGreet
def:greet
```

### Goto definition through namespace imports

Goto definition should resolve through namespace member access across modules.

```ds:namespace_lib.ds
export function greet(name: string): string {
//              ^^^^^ def:namespace_greet
    return "Hello, " + name;
}
```

```ds:namespace_main.ds
import * as api from "./namespace_lib.ds";

const message = api.greet("Destack");
//                  ^^^^^ use:namespace_greet
```

```query goto_definition use:namespace_greet
def:namespace_greet
```
