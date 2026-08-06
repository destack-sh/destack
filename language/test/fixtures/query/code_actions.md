
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
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#insertion text="import { greet } from \"./library\";\n"
```

```query code_actions main.ds#range only=quick_fix
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#insertion text="import { greet } from \"./library\";\n"
```

```query code_actions main.ds#range only=quick_fix
diagnostics none
@code_actions.none
```

### Import an exported type

A missing type reference offers a named import that preserves its type symbol space.

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
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="Import Options from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#insertion text="import { Options } from \"./library\";\n"
```

### Return every import candidate

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
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="Import greet from \"./beta\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#insertion text="import { greet } from \"./beta\";\n"
@code_actions.action index=1 title="Import greet from \"./alpha\"" kind=quick_fix applicability=automatic
@code_actions.diagnostic action=1 id=unresolved-reference location=main.ds#range
@code_actions.patch action=1 range=main.ds#insertion text="import { greet } from \"./alpha\";\n"
```

### Extend an existing import

An existing import from the target module receives the missing named specifier.

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
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="Import beta from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
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
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#insertion text="import greet from \"./library\";\n"
```

## Diagnostic Suggestions

### Apply an automatic name correction

An unambiguous case correction is safe to apply directly.

```ds main.ds
const value = 1;
const copy = Value;
             ^^^^^ range
```

```query code_actions main.ds#range only=quick_fix
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="rename to 'value'" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#range text=value
```

### Offer a correction that requires review

A transposed name remains available without becoming the preferred action.

```ds main.ds
const value = 1;
const copy = valeu;
             ^^^^^ range
```

```query code_actions main.ds#range only=quick_fix
diagnostic unresolved-reference main.ds#range
@code_actions.action index=0 title="rename to 'value'" kind=quick_fix applicability=dangerous
@code_actions.diagnostic action=0 id=unresolved-reference location=main.ds#range
@code_actions.patch action=0 range=main.ds#range text=value
```

## Extraction

### Extract an expression

A non-empty expression range offers its extraction edit.

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

### Inline a binding

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

### Select several refactor kinds

Several requested kinds return every matching action in stable kind order.

```ds main.ds
const offset = 10;
const total = offset + 1;
              ^^^^^^ target
```

```query code_actions main.ds#target only=refactor_extract,refactor_inline
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.ds:2:1 text="const extracted = offset;\n"
@code_actions.patch action=0 range=main.ds#target text=extracted
@code_actions.action index=1 title="Inline symbol" kind=refactor_inline
@code_actions.patch action=1 range=main.ds:1:1-2:1 text=""
@code_actions.patch action=1 range=main.ds#target text=10
```

## Extract Function

### [ignored] Extract an expression into a function

Extraction can introduce a module function and replace the selected expression with its call.

```ds main.ds

^ insertion
function calculate(): int32 {
    return 1 + 2;
           ^^^^^ selection
}
```

```query code_actions main.ds#selection only=refactor_extract
@code_actions.action index=0 title="Extract function" kind=refactor_extract
@code_actions.patch action=0 range=main.ds#insertion text="function extracted(): int32 {\n    return 1 + 2;\n}\n\n"
@code_actions.patch action=0 range=main.ds#selection text="extracted()"
```

## Implementations

### [ignored] Implement required members

A concrete type can insert the interface members it has not implemented.

```ds main.ds
interface Drawable {
    draw(): void;
}

class Point implements Drawable {
                       ^^^^^^^ diagnostic
    x: int32 = 0;
}
^ insertion
```

```query code_actions main.ds#diagnostic
diagnostic interface-not-implemented main.ds#diagnostic
@code_actions.action index=0 title="Implement missing members" kind=quick_fix applicability=dangerous
@code_actions.diagnostic action=0 id=interface-not-implemented location=main.ds#diagnostic
@code_actions.patch action=0 range=main.ds#insertion@start text="    draw(): void {}\n"
```

## Source changes

### Update available actions after an expression changes

Code actions reflect the selected expression in the current revision.

```ds main.ds
const value = 42;
      ^^^^^ range
```

```query code_actions main.ds#range only=refactor_extract
@code_actions.none
```

```ds main.ds change
function total(): int32 {
    return 1 + 2;
           ^^^^^ range
}
```

```query code_actions main.ds#range only=refactor_extract
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.ds:2:1 text="    const extracted = 1 + 2;\n"
@code_actions.patch action=0 range=main.ds#range text=extracted
```
