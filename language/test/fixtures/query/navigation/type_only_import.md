# Type-Only Imports

## Type Definitions

### Goto type definition with type-only import

Goto type definition should resolve type-only imports.

```ds:types.ds
export type Options = {
//          ^^^^^^^ def:Options
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(options: Options): void {
//                          ^^^^^ use:Options
    console.log(options.name);
}
```

```query goto_type_definition use:Options
def:Options
```
