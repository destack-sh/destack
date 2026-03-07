# Document Link

## Import Links

### Link import specifiers

Document links should expose import specifiers as exact file targets.

```ds:main.ds
import { foo } from "./foo.ds";
import { bar } from "./bar.ds";
```

```ds:foo.ds
export const foo = 1;
```

```ds:bar.ds
export const bar = 2;
```

```lsp document_link
range=0:20-0:30
target=foo.ds
tooltip=Go to ./foo.ds

range=1:20-1:30
target=bar.ds
tooltip=Go to ./bar.ds
```

## Split Imports

### Retarget document links through two module path rewrites
Document links should follow the current import target across repeated edits.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:gamma.ds
export const gamma = 3;
```

```ds:main.ds
import { alpha } from "./alpha.ds";
const current = alpha;
```

```ds:main.ds[1]
import { beta } from "./beta.ds";
const current = beta;
```

```ds:main.ds[2]
import { gamma } from "./gamma.ds";
const current = gamma;
```

```lsp document_link main.ds [0]
range=0:22-0:34
target=alpha.ds
tooltip=Go to ./alpha.ds
```

```lsp document_link main.ds [1]
range=0:21-0:32
target=beta.ds
tooltip=Go to ./beta.ds
```

```lsp document_link main.ds [2]
range=0:22-0:34
target=gamma.ds
tooltip=Go to ./gamma.ds
```

### Retarget a barrel import link after the barrel rewrites
Document links should update when the barrel path changes but the consumer file does not.

```ds:alpha.ds
export const value = 1;
```

```ds:beta.ds
export const value = 2;
```

```ds:barrel.ds
export { value } from "./alpha.ds";
```

```ds:main.ds
import { value } from "./barrel.ds";
const current = value;
```

```ds:barrel.ds[1]
export { value } from "./beta.ds";
```

```lsp document_link barrel.ds [0]
range=0:22-0:34
target=alpha.ds
tooltip=Go to ./alpha.ds
```

```lsp document_link barrel.ds [1]
range=0:22-0:33
target=beta.ds
tooltip=Go to ./beta.ds
```

## Link Retargeting

### Retarget two direct import links after a shared library split
Document links should update both import targets when two consumers move from one shared file to two split files.

```ds:shared.ds
export const left = 1;

export const right = 2;
```

```ds:left.ds
export const left = 1;
```

```ds:right.ds
export const right = 2;
```

```ds:a.ds
import { left } from "./shared.ds";
const current = left;
```

```ds:b.ds
import { right } from "./shared.ds";
const current = right;
```

```ds:a.ds[1]
import { left } from "./left.ds";
const current = left;
```

```ds:b.ds[1]
import { right } from "./right.ds";
const current = right;
```

```lsp document_link a.ds [0]
range=0:21-0:34
target=shared.ds
tooltip=Go to ./shared.ds
```

```lsp document_link b.ds [0]
range=0:22-0:35
target=shared.ds
tooltip=Go to ./shared.ds
```

```lsp document_link a.ds [1]
range=0:21-0:32
target=left.ds
tooltip=Go to ./left.ds
```

```lsp document_link b.ds [1]
range=0:22-0:34
target=right.ds
tooltip=Go to ./right.ds
```

## Command Churn

### Preserve document links while reindexing through sibling breakage and recovery

Document links should stay exact while import targets rewrite, a sibling file breaks, and later command churn clears the workspace again.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:gamma.ds
export const gamma = 3;
```

```ds:main.ds
import { alpha } from "./alpha.ds";
import { beta } from "./beta.ds";

const left = alpha;
const right = beta;
```

```ds:broken.ds
export const stable = 1;
```

```lsp document_link main.ds [0]
range=0:22-0:34
target=alpha.ds
tooltip=Go to ./alpha.ds

range=1:21-1:32
target=beta.ds
tooltip=Go to ./beta.ds
```

```ds:main.ds[1]
import { gamma } from "./gamma.ds";
import { beta } from "./beta.ds";

const left = gamma;
const right = beta;
```

```ds:broken.ds[1]
export const stable = ;
```

```lsp execute_command destack.reindex [1]
```

```lsp document_link main.ds [1]
range=0:22-0:34
target=gamma.ds
tooltip=Go to ./gamma.ds

range=1:21-1:32
target=beta.ds
tooltip=Go to ./beta.ds
```

```lsp workspace_diagnostic [1]
file=broken.ds
range=0:22-0:23
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```ds:main.ds[2]
import { gamma } from "./gamma.ds";
import { alpha } from "./alpha.ds";

const left = gamma;
const right = alpha;
```

```ds:broken.ds[2]
export const stable = 1;
```

```lsp execute_command destack.rescan [2]
```

```lsp document_link main.ds [2]
range=0:22-0:34
target=gamma.ds
tooltip=Go to ./gamma.ds

range=1:22-1:34
target=alpha.ds
tooltip=Go to ./alpha.ds
```

```lsp workspace_diagnostic [2]
```
