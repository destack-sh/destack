# Rename

## Preparation

### Prepare rename at a local value

Prepare rename should return the exact editable span for a local binding.

```ds:main.ds
const [|/*prepare_rename*/value|] = 1;
const next = value + 1;
```

## Cross-Module Updates

### Rename an exported symbol across modules

Rename should update the exported symbol, its imports, and its uses across files.

```ds:main.ds
import { [|greet|] } from "./lib.ds";
const output = [|/*rename*/greet|]("Ada");
```

```ds:lib.ds
export function [|greet|](name: string): string {
    return name;
}
```

