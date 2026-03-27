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
//            ^^^^^^^ decl:Options_import

function configure(options: Options): void {
//                          ^^^^^ use:Options
    console.log(options.name);
}
```

```query goto_type_definition use:Options
def:Options
```

### Goto definition with type-only import

Goto definition should resolve type-only imports to the source type declaration.

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

```query goto_definition use:Options
def:Options
```

### Goto declaration with type-only import

Goto declaration should stop at the local type-only import specifier.

```ds:types.ds
export type Options = {
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";
//            ^^^^^^^ decl:Options_import

function configure(options: Options): void {
//                          ^^^^^ use:Options
    console.log(options.name);
}
```

```query goto_declaration use:Options
decl:Options_import
```
