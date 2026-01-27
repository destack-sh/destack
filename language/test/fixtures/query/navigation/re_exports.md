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
