# Code Lens

## Function Lenses

### Show reference and test lenses

Code lenses should report reference counts and test actions.

```ds:main.ds
function /*lens*/greet(): void {}

greet();

@test
function test_example(): void {}
```

```lsp code_lens
range=0:9-0:14
title=1 reference
command=destack.showReferences

range=5:9-5:21
title=▶ Run test_example
command=destack.runTest
arg=test_example
```

## Code lens counts

### Shrink reference counts after deleting one call
Code lenses should lower the reference count after one call site is removed.

```ds:main.ds
function /*lens*/greet(): void {}

greet();
greet();
```

```ds:main.ds[1]
function /*lens*/greet(): void {}
greet();
```

```lsp code_lens main.ds [0]
range=0:9-0:14
title=2 references
command=destack.showReferences
```

```lsp code_lens main.ds [1]
range=0:9-0:14
title=1 reference
command=destack.showReferences
```

### Grow reference counts after adding a third call
Code lenses should raise the reference count after another call is inserted.

```ds:main.ds
function /*lens*/greet(): void {}

greet();
greet();
```

```ds:main.ds[1]
function /*lens*/greet(): void {}

greet();
greet();
greet();
```

```lsp code_lens main.ds [0]
range=0:9-0:14
title=2 references
command=destack.showReferences
```

```lsp code_lens main.ds [1]
range=0:9-0:14
title=3 references
command=destack.showReferences
```

## Workspace Churn

### Preserve code-lens counts while a sibling file breaks and recovers

Code lenses should keep exact reference counts while another file breaks, commands churn the workspace, and one consumer drops back out.

```ds:lib.ds
export function /*lens*/ping(): void {}
```

```ds:a.ds
import { ping } from "./lib.ds";

ping();
```

```ds:b.ds
export const clean = 1;
```

```ds:c.ds
export const clean = 1;
```

```lsp code_lens lib.ds [0]
range=0:16-0:20
title=1 reference
command=destack.showReferences
```

```ds:b.ds[1]
import { ping } from "./lib.ds";

ping();
```

```ds:c.ds[1]
export const broken = ;
```

```lsp execute_command destack.reindex [1]
```

```lsp code_lens lib.ds [1]
range=0:16-0:20
title=2 references
command=destack.showReferences
```

```lsp workspace_diagnostic [1]
file=c.ds
range=0:22-0:23
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

```ds:a.ds[2]
export const idle = 1;
```

```ds:c.ds[2]
export const fixed = 1;
```

```lsp execute_command destack.clearCache [2]
```

```lsp code_lens lib.ds [2]
range=0:16-0:20
title=1 reference
command=destack.showReferences
```

```lsp workspace_diagnostic [2]
```
