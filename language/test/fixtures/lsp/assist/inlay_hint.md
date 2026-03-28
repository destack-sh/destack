# Inlay Hint

## Function Calls

### Show type and parameter hints

Inlay hints should show the inferred binding type and parameter names for literal arguments.

```ds:main.ds
function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}

const msg = greet("World", "Hello");
```

```lsp inlay_hint
position=4:9
label=: string
kind=type
padding_left=false
padding_right=false

position=4:18
label=name:
kind=parameter
padding_left=false
padding_right=true

position=4:27
label=greeting:
kind=parameter
padding_left=false
padding_right=true
```

## Inlay hint churn

### Gain a binding type hint after removing the annotation
Inlay hints should add the inferred binding type when the explicit annotation is removed.

```ds:main.ds
function greet(name: string, greeting: string): string {
   return greeting + ", " + name;
}
const msg: string = greet("World", "Hello");
```

```ds:main.ds[1]
function greet(name: string, greeting: string): string {
   return greeting + ", " + name;
}
const msg = greet("World", "Hello");
```

```lsp inlay_hint main.ds [0]
position=3:26
label=name:
kind=parameter
padding_left=false
padding_right=true

position=3:35
label=greeting:
kind=parameter
padding_left=false
padding_right=true
```

```lsp inlay_hint main.ds [1]
position=3:9
label=: string
kind=type
padding_left=false
padding_right=false

position=3:18
label=name:
kind=parameter
padding_left=false
padding_right=true

position=3:27
label=greeting:
kind=parameter
padding_left=false
padding_right=true
```

### Show parameter hints for imported calls
Inlay hints should resolve imported parameter names and still suppress obvious non-literal arguments.

```ds:main.ds
import { paint } from "./lib.ds";

const label = "red";
paint("blue", 2);
paint(label, 3);
```

```ds:lib.ds
export function paint(color: string, coats: int32): void {}
```

```lsp inlay_hint
position=2:11
label=: string
kind=type
padding_left=false
padding_right=false

position=3:6
label=color:
kind=parameter
padding_left=false
padding_right=true

position=3:14
label=coats:
kind=parameter
padding_left=false
padding_right=true

position=4:13
label=coats:
kind=parameter
padding_left=false
padding_right=true
```

### Preserve imported hints while a sibling file breaks and recovers
Inlay hints should stay exact for the active file while a sibling file breaks and later recovers.

```ds:lib.ds
export function paint(color: string, coats: int32): void {}
```

```ds:main.ds
import { paint } from "./lib.ds";

paint("blue", 2);
```

```ds:broken.ds
export const stable = 1;
```

```ds:main.ds[1]
import { paint } from "./lib.ds";

paint("blue", 2);
```

```ds:broken.ds[1]
export const stable = ;
```

```ds:main.ds[2]
import { paint } from "./lib.ds";

paint("blue", 2);
```

```ds:broken.ds[2]
export const stable = 1;
```

```lsp inlay_hint main.ds [0]
position=2:6
label=color:
kind=parameter
padding_left=false
padding_right=true

position=2:14
label=coats:
kind=parameter
padding_left=false
padding_right=true
```

```lsp inlay_hint main.ds [1]
position=2:6
label=color:
kind=parameter
padding_left=false
padding_right=true

position=2:14
label=coats:
kind=parameter
padding_left=false
padding_right=true
```

```lsp inlay_hint main.ds [2]
position=2:6
label=color:
kind=parameter
padding_left=false
padding_right=true

position=2:14
label=coats:
kind=parameter
padding_left=false
padding_right=true
```
