# Signature Help

## Basic

### Active parameter in the second argument

Signature help should point at the second parameter when the caret sits on the second argument.

```ds:main.ds
function greet(name: string, times: number): string {
    return name;
}

const message = greet("World", /*signature*/1);
```

```lsp signature_label
greet(name: string, times: number): string
```

```lsp active_parameter
1
```

## Trigger Handling

### Trigger character opens help

Signature help should open when the configured trigger character is typed.

```ds:main.ds
function render(input: number, scale: number): number {
    return input * scale;
}

const output = render(1, /*signature_trigger*/2);
```

```lsp signature_help_trigger signature_trigger [0]
,
```

```lsp signature_label
function render(input: number, scale: number): number
```

```lsp active_parameter
1
```

### Value names do not open help

Signature help should stay absent when the caret is on a non-call value name.

```ds:main.ds
const /*no_signature*/value = 1;
```

```lsp no_signature_help no_signature [0]
```

### Trigger-reason checks stay empty

Trigger-reason requests should stay empty when the caret is not at a callable position.

```ds:main.ds
const /*no_signature_trigger*/value = 1;
```

```lsp no_signature_help_for_trigger_reason no_signature_trigger [0]
```

## Imports

### Namespace imported calls show signature help

Signature help should resolve parameter names through namespace imports.

```ds:lib.ds
export function paint(color: string, coats: int32): void {}
```

```ds:main.ds
import * as api from "./lib.ds";

api.paint("blue", /*signature*/2);
```

```lsp signature_label
paint(color: string, coats: int32): void
```

```lsp active_parameter
1
```

### Default re-exported calls show signature help

Signature help should resolve through default re-export alias chains.

```ds:lib.ds
export default function repeat(text: string, times: int32): string {
    return text;
}
```

```ds:barrel.ds
export { default as repeat } from "./lib.ds";
```

```ds:main.ds
import { repeat } from "./barrel.ds";

const value = repeat("hi", /*signature*/2);
```

```lsp signature_label
repeat(text: string, times: int32): string
```

```lsp active_parameter
1
```

### Trailing commas keep the last parameter active

Signature help should keep the final parameter active after a trailing comma.

```ds:main.ds
function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, 2,/*signature*/);
```

```lsp signature_label
add(x: int32, y: int32): int32
```

```lsp active_parameter
1
```
