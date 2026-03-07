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
export const [|/*def*/value|] = 1;
```

```ds:main.ds
import { [|/*import_use*/value|] } from "./lib.ds";
const first = [|/*use_first*/value|];
const second = [|/*use_second*/value|];
```

```lsp references def [0]
def
import_use
use_first
use_second
```

## Reference churn

### Drop a deleted use site from the reference set
References should shrink after one use site disappears from the overlay, while import references remain.

```ds:lib.ds
[|export function /*def*/ping(): void {}|]
```

```ds:main.ds
import { /*import_use*/ping } from "./lib.ds";

/*use1*/ping();
/*use2*/ping();
```

```ds:lib.ds[1]
[|export function /*def*/ping(): void {}|]
```

```ds:main.ds[1]
import { /*import_use*/ping } from "./lib.ds";

/*use1*/ping();
```

```lsp references def [0]
def
import_use
use1
use2
```

```lsp references def [1]
def
import_use
use1
```

### Grow references across three files after a new import use appears
References should include the new third-file import and use after the overlay adds them.

```ds:lib.ds
[|export function /*def*/ping(): void {}|]
```

```ds:a.ds
import { /*import_a*/ping } from "./lib.ds";

/*use_a*/ping();
```

```ds:b.ds
import { /*import_b*/ping } from "./lib.ds";
```

```ds:lib.ds[1]
[|export function /*def*/ping(): void {}|]
```

```ds:a.ds[1]
import { /*import_a*/ping } from "./lib.ds";

/*use_a*/ping();
```

```ds:b.ds[1]
import { /*import_b*/ping } from "./lib.ds";

/*use_b*/ping();
```

```lsp references def [0]
def
import_a
use_a
import_b
```

```lsp references def [1]
def
import_a
use_a
import_b
use_b
```

### Switch references from the imported symbol to the local shadow
References should follow the new local declaration after the import is removed.

```ds:lib.ds
[|export function /*imported*/ping(): void {}|]
```

```ds:main.ds
import { /*import_use*/ping } from "./lib.ds";

/*use*/ping();
```

```ds:lib.ds[1]
[|export function /*imported*/ping(): void {}|]
```

```ds:main.ds[1]
[|function /*local*/ping(): void {}|]

/*use*/ping();
```

```lsp references imported [0]
imported
import_use
use
```

```lsp references local [1]
local
use
```

## Reference Churn

### Grow references across three consumers one file at a time
References should grow as more files start importing and using the same export.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:a.ds
import { /*a_symbol*/ping } from "./lib.ds";

/*a_import*/ping();
```

```ds:b.ds
import { /*b_symbol*/ping } from "./lib.ds";
```

```ds:c.ds
import { /*c_symbol*/ping } from "./lib.ds";
```

```ds:b.ds[1]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_import*/ping();
```

```ds:c.ds[1]
import { /*c_symbol*/ping } from "./lib.ds";
```

```ds:b.ds[2]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_import*/ping();
```

```ds:c.ds[2]
import { /*c_symbol*/ping } from "./lib.ds";

/*c_import*/ping();
```

```lsp references def [0]
def
a_symbol
a_import
b_symbol
c_symbol
```

```lsp references def [1]
def
a_symbol
a_import
b_symbol
b_import
c_symbol
```

```lsp references def [2]
def
a_symbol
a_import
b_symbol
b_import
c_symbol
c_import
```

### Grow then prune references across four overlay states
References should follow each intermediate consumer state instead of collapsing to the final answer.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:a.ds
import { /*a_symbol*/ping } from "./lib.ds";

/*a_use*/ping();
```

```ds:b.ds
import { /*b_symbol*/ping } from "./lib.ds";
```

```ds:c.ds
import { /*c_symbol*/ping } from "./lib.ds";
```

```ds:b.ds[1]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_use*/ping();
```

```ds:b.ds[2]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_use*/ping();
```

```ds:c.ds[2]
import { /*c_symbol*/ping } from "./lib.ds";

/*c_use*/ping();
```

```ds:a.ds[3]
import { /*a_symbol*/ping } from "./lib.ds";
```

```ds:b.ds[3]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_use*/ping();
```

```ds:c.ds[3]
import { /*c_symbol*/ping } from "./lib.ds";

/*c_use*/ping();
```

```ds:a.ds[4]
const unrelated = 1;
```

```ds:b.ds[4]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_use*/ping();
```

```ds:c.ds[4]
import { /*c_symbol*/ping } from "./lib.ds";

/*c_use*/ping();
```

```lsp references def [0]
def
a_symbol
a_use
b_symbol
c_symbol
```

```lsp references def [1]
def
a_symbol
a_use
b_symbol
b_use
c_symbol
```

```lsp references def [2]
def
a_symbol
a_use
b_symbol
b_use
c_symbol
c_use
```

```lsp references def [3]
def
a_symbol
b_symbol
b_use
c_symbol
c_use
```

```lsp references def [4]
def
b_symbol
b_use
c_symbol
c_use
```

### Shrink references as consumers switch to other exports
References should shrink when different files stop using the tracked export.

```ds:lib.ds
export [|function /*def*/ping(): void {}|] export function pong(): void {}
```

```ds:a.ds
import { /*a_symbol*/ping } from "./lib.ds";

/*a_import*/ping();
```

```ds:b.ds
import { /*b_symbol*/ping } from "./lib.ds";

/*b_import*/ping();
```

```ds:c.ds
import { /*c_symbol*/ping } from "./lib.ds";

/*c_import*/ping();
```

```ds:b.ds[1]
import { pong } from "./lib.ds";
pong();
```

```ds:c.ds[1]
import { /*c_symbol*/ping } from "./lib.ds";

/*c_import*/ping();
```

```ds:c.ds[2]
import { pong } from "./lib.ds";
pong();
```

```lsp references def [0]
def
a_symbol
a_import
b_symbol
b_import
c_symbol
c_import
```

```lsp references def [1]
def
a_symbol
a_import
c_symbol
c_import
```

```lsp references def [2]
def
a_symbol
a_import
```

### Move references from one export to another through a shared consumer
References should move cleanly between exports when one consumer rewrites its import.

```ds:lib.ds
export [|function /*first_def*/ping(): void {}|] export [|function /*second_def*/pong(): void {}|]
```

```ds:main.ds
import { /*first_symbol*/ping } from "./lib.ds";

/*first_use*/ping();
```

```ds:main.ds[1]
import { /*second_symbol*/pong } from "./lib.ds";

/*second_use*/pong();
```

```lsp references first_def [0]
first_def
first_symbol
first_use
```

```lsp references second_def [1]
second_def
second_symbol
second_use
```

## Reference Churn

### Grow references through staggered consumer activation
References should grow as two dormant consumers start using the export one after another.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:a.ds
import { /*a_symbol*/ping } from "./lib.ds";

/*a_use*/ping();
```

```ds:b.ds
import { /*b_symbol*/ping } from "./lib.ds";
```

```ds:c.ds
import { /*c_symbol*/ping } from "./lib.ds";
```

```ds:b.ds[1]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_use*/ping();
```

```ds:c.ds[1]
import { /*c_symbol*/ping } from "./lib.ds";
```

```ds:b.ds[2]
import { /*b_symbol*/ping } from "./lib.ds";

/*b_use*/ping();
```

```ds:c.ds[2]
import { /*c_symbol*/ping } from "./lib.ds";

/*c_use*/ping();
```

```lsp references def [0]
def
a_symbol
a_use
b_symbol
c_symbol
```

```lsp references def [1]
def
a_symbol
a_use
b_symbol
b_use
c_symbol
```

```lsp references def [2]
def
a_symbol
a_use
b_symbol
b_use
c_symbol
c_use
```
