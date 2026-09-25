
## No Action

### Return no action for valid code

Valid code has no applicable action.

```tspp main.tspp
const value = 42;
      ^^^^^ range
```

```query code_actions main.tspp#range
@code_actions.none
```

## Auto Imports

### Import an exported function

An unresolved exported name offers an import action.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp

^ insertion
function main(): void {
    greet();
    ^^^^^ range
}
```

```query code_actions main.tspp#range
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#insertion text="import { greet } from \"./library\";\n"
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#insertion text="import { greet } from \"./library\";\n"
```

```query code_actions main.tspp#range only=quick_fix
diagnostics none
@code_actions.none
```

### Return user and builtin type imports

A missing type reference offers every addressable declaration with the nearer source first.

```tspp library.tspp
export type Options = {
    enabled: boolean,
};
```

```tspp main.tspp

^ insertion
declare const options: Options;
                       ^^^^^^^ range
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="Import Options from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#insertion text="import { Options } from \"./library\";\n"
@code_actions.action index=1 title="Import Options from \"tspp:test\"" kind=quick_fix applicability=automatic
@code_actions.diagnostic action=1 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=1 range=main.tspp#insertion text="import { Options } from \"tspp:test\";\n"
```

### Return every import candidate

Equal exported names produce stable actions, and only the first candidate is preferred.

```tspp alpha.tspp
export function greet(): void {}
```

```tspp beta.tspp
export function greet(): void {}
```

```tspp main.tspp

^ insertion
greet();
^^^^^ range
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="Import greet from \"./beta\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#insertion text="import { greet } from \"./beta\";\n"
@code_actions.action index=1 title="Import greet from \"./alpha\"" kind=quick_fix applicability=automatic
@code_actions.diagnostic action=1 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=1 range=main.tspp#insertion text="import { greet } from \"./alpha\";\n"
```

### Extend an existing import

An existing import from the target module receives the missing named specifier.

```tspp library.tspp
export function alpha(): void {}
export function beta(): void {}
```

```tspp main.tspp
import { alpha } from "./library";
              ^ insertion

alpha();
beta();
^^^^ range
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="Import beta from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#insertion text=", beta"
```

### Import a default declaration

A missing default export receives a default import.

```tspp library.tspp
export default function greet(): void {}
```

```tspp main.tspp

^ insertion
greet();
^^^^^ range
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="Import greet from \"./library\"" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#insertion text="import greet from \"./library\";\n"
```

## Diagnostic Suggestions

### Apply an automatic name correction

An unambiguous case correction is safe to apply directly.

```tspp main.tspp
const fixtureValue = 1;
const copy = FixtureValue;
             ^^^^^^^^^^^^ range
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="rename to 'fixtureValue'" kind=quick_fix applicability=automatic preferred=true
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#range text=fixtureValue
```

### Offer a correction that requires review

A transposed name remains available without becoming the preferred action.

```tspp main.tspp
const value = 1;
const copy = valeu;
             ^^^^^ range
```

```query code_actions main.tspp#range only=quick_fix
diagnostic unresolved-reference main.tspp#range
@code_actions.action index=0 title="rename to 'value'" kind=quick_fix applicability=dangerous
@code_actions.diagnostic action=0 id=unresolved-reference location=main.tspp#range
@code_actions.patch action=0 range=main.tspp#range text=value
```

## Extraction

### Extract an expression

A non-empty expression range offers its extraction edit.

```tspp main.tspp
function total(): int32 {
    return 1 + 2;
           ^^^^^ selection
}
```

```query code_actions main.tspp#selection only=refactor_extract
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.tspp:2:1 text="    const extracted = 1 + 2;\n"
@code_actions.patch action=0 range=main.tspp#selection text=extracted
```

### Offer extraction for the current expression

Code actions reflect the selected expression after each edit.

```tspp main.tspp
const value = 42;
      ^^^^^ range
```

```query code_actions main.tspp#range only=refactor_extract
@code_actions.none
```

```tspp main.tspp change
function total(): int32 {
    return 1 + 2;
           ^^^^^ range
}
```

```query code_actions main.tspp#range only=refactor_extract
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.tspp:2:1 text="    const extracted = 1 + 2;\n"
@code_actions.patch action=0 range=main.tspp#range text=extracted
```

## Inline

### Inline a binding

A binding name offers the same complete edit as the inline refactoring.

```tspp main.tspp
const offset = 10;
      ^^^^^^ target
const total = offset + 1;
              ^^^^^^ reference
```

```query code_actions main.tspp#target only=refactor_inline
@code_actions.action index=0 title="Inline symbol" kind=refactor_inline
@code_actions.patch action=0 range=main.tspp:1:1-2:1 text=""
@code_actions.patch action=0 range=main.tspp#reference text=10
```

## Filtering

### Select several refactor kinds

Several requested kinds return every matching action in stable kind order.

```tspp main.tspp
const offset = 10;
const total = offset + 1;
              ^^^^^^ target
```

```query code_actions main.tspp#target only=refactor_extract,refactor_inline
@code_actions.action index=0 title="Extract constant" kind=refactor_extract
@code_actions.patch action=0 range=main.tspp:2:1 text="const extracted = offset;\n"
@code_actions.patch action=0 range=main.tspp#target text=extracted
@code_actions.action index=1 title="Inline symbol" kind=refactor_inline
@code_actions.patch action=1 range=main.tspp:1:1-2:1 text=""
@code_actions.patch action=1 range=main.tspp#target text=10
```
