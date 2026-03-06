# Find References

## Cross-Module Uses

### References across modules

Find references should include the exact cross-module uses of the selected symbol.

```ds:main.ds
import { [|ping|] } from "./lib.ds";
[|/*refs*/ping|]();
```

```ds:lib.ds
export function [|/*def*/ping|](): void {}
```

## Repeated Range Walks

### Run references at each marked range

Find references should stay stable when the harness walks each marked range in turn.

```ds:lib.ds
export const [|value|] = 1;
```

```ds:main.ds
import { [|value|] } from "./lib.ds";
const first = [|value|];
const second = [|value|];
```

```lsp scenario references-each-range-cross-module
```

