# Code Actions

## No Action

### Return no action for valid code

Valid code has no applicable action.

```ds main.ds
const value = 42;
      ^^^^^ range
```

```query code_actions main.ds#range
@code_actions.none
```

## Auto Imports

### Import an exported function

An unresolved exported name offers an import action.

```ds library.ds
export function greet(): void {}
```

```ds main.ds

^ insertion
function main(): void {
    greet();
    ^^^^^ range
}
```

```query code_actions main.ds#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix preferred=true diagnostic=unresolved-reference
@code_actions.patch action=0 range=main.ds#insertion text="import { greet } from \"./library\";\n"
```

```query code_actions main.ds#range only=quick_fix
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix preferred=true diagnostic=unresolved-reference
@code_actions.patch action=0 range=main.ds#insertion text="import { greet } from \"./library\";\n"
```

### Import an exported type

A missing type reference offers a plain import that preserves its type symbol space.

```ds library.ds
export type Options = {
    enabled: boolean,
};
```

```ds main.ds

^ insertion
declare const options: Options;
                       ^^^^^^^ range
```

```query code_actions main.ds#range only=quick_fix
@code_actions.action index=0 title="Import Options from \"./library\"" kind=quick_fix preferred=true diagnostic=unresolved-reference
@code_actions.patch action=0 range=main.ds#insertion text="import { Options } from \"./library\";\n"
```

### Return every exact import candidate

Equal exported names produce stable actions, and only the first candidate is preferred.

```ds alpha.ds
export function greet(): void {}
```

```ds beta.ds
export function greet(): void {}
```

```ds main.ds

^ insertion
greet();
^^^^^ range
```

```query code_actions main.ds#range only=quick_fix
@code_actions.action index=0 title="Import greet from \"./alpha\"" kind=quick_fix preferred=true diagnostic=unresolved-reference
@code_actions.patch action=0 range=main.ds#insertion text="import { greet } from \"./alpha\";\n"
@code_actions.action index=1 title="Import greet from \"./beta\"" kind=quick_fix diagnostic=unresolved-reference
@code_actions.patch action=1 range=main.ds#insertion text="import { greet } from \"./beta\";\n"
```

### Extend an existing import

An import from the selected module receives the missing named specifier.

```ds library.ds
export function alpha(): void {}
export function beta(): void {}
```

```ds main.ds
import { alpha } from "./library";
               ^ insertion

alpha();
beta();
^^^^ range
```

```query code_actions main.ds#range only=quick_fix
@code_actions.action index=0 title="Import beta from \"./library\"" kind=quick_fix preferred=true diagnostic=unresolved-reference
@code_actions.patch action=0 range=main.ds#insertion text=", beta"
```

### Import a default declaration

A missing default export receives a default import.

```ds library.ds
export default function greet(): void {}
```

```ds main.ds

^ insertion
greet();
^^^^^ range
```

```query code_actions main.ds#range only=quick_fix
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix preferred=true diagnostic=unresolved-reference
@code_actions.patch action=0 range=main.ds#insertion text="import greet from \"./library\";\n"
```

## Extraction

### Extract a selected expression

A non-empty expression range offers the exact extraction edit.

```ds main.ds
function total(): int32 {
    return 1 + 2;
           ^^^^^ selection
}
```

```query code_actions main.ds#selection only=refactor_extract
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.ds:2:1 text="    const extracted = 1 + 2;\n"
@code_actions.patch action=0 range=main.ds#selection text=extracted
```

## Inline

### Inline a selected binding

A binding name offers the same complete edit as the inline query.

```ds main.ds
const offset = 10;
      ^^^^^^ target
const total = offset + 1;
              ^^^^^^ reference
```

```query code_actions main.ds#target only=refactor_inline
@code_actions.action index=0 title="Inline symbol" kind=refactor_inline
@code_actions.patch action=0 range=main.ds:1:1-2:1 text=""
@code_actions.patch action=0 range=main.ds#reference text=10
```

## Filtering

### Include every refactor subkind

The broad refactor kind retains every applicable specialized refactor in stable kind order.

```ds main.ds
const offset = 10;
const total = offset + 1;
              ^^^^^^ target
```

```query code_actions main.ds#target only=refactor
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.ds:2:1 text="const extracted = offset;\n"
@code_actions.patch action=0 range=main.ds#target text=extracted
@code_actions.action index=1 title="Inline symbol" kind=refactor_inline
@code_actions.patch action=1 range=main.ds:1:1-2:1 text=""
@code_actions.patch action=1 range=main.ds#target text=10
```
