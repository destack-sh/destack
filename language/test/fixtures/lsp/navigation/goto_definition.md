# Goto Definition

## Imported Values

### Jump from an imported use site

Goto definition should jump from an imported use site to the exported declaration.

```ds:main.ds
import { ping } from "./lib.ds";
const result = /*use*/ping();
```

```ds:lib.ds
export function /*def*/ping(): void {}
```

## Repeated Lookups

### Each marker across modules

Goto definition should resolve each marked use site to the shared declaration across modules.

```ds:lib.ds
export const /*def*/value = 1;
```

```ds:main.ds
import { value } from "./lib.ds";
const first = /*use_first*/value;
const second = /*use_second*/value;
```

```lsp definition use_first [0]
def
```

```lsp definition use_second [0]
def
```

### Each marker in one file

Goto definition should resolve each marked use site to the local declaration in the same file.

```ds:main.ds
const [|/*def*/value|] = 1;
const first = /*use_first*/value;
const second = /*use_second*/value;
```

```lsp definition use_first [0]
def
```

```lsp definition use_second [0]
def
```

## Definition drift

### Retarget an import after rewriting the module path
Definition should jump to the new export after the import path changes in the overlay.

```ds:foo.ds
[|export const /*foo_def*/value = 1;|]
```

```ds:bar.ds
[|export const /*bar_def*/value = 2;|]
```

```ds:main.ds
import { value } from "./foo.ds";
const result = /*use*/value;
```

```ds:foo.ds[1]
[|export const /*foo_def*/value = 1;|]
```

```ds:bar.ds[1]
[|export const /*bar_def*/value = 2;|]
```

```ds:main.ds[1]
import { value } from "./bar.ds";
const result = /*use*/value;
```

```lsp definition use [0]
foo_def
```

```lsp definition use [1]
bar_def
```

### Prefer a new local shadow over the imported declaration
Definition should switch from the imported symbol to the new local declaration after the import is removed.

```ds:lib.ds
[|export function /*imported*/ping(): void {}|]
```

```ds:main.ds
import { ping } from "./lib.ds";

/*use*/ping();
```

```ds:lib.ds[1]
[|export function /*imported*/ping(): void {}|]
```

```ds:main.ds[1]
[|function /*local*/ping(): void {}|]

/*use*/ping();
```

```lsp definition use [0]
imported
```

```lsp definition use [1]
local
```

## Definition Retargeting

### Retarget from a barrel export to a direct beta import
Definition should stay exact when one consumer moves off a barrel export onto a direct import.

```ds:alpha.ds
export [|const /*alpha_def*/value = 1;|]
```

```ds:beta.ds
export [|const /*beta_def*/value = 2;|]
```

```ds:barrel.ds
export { value } from "./alpha.ds";
```

```ds:main.ds
import { value } from "./barrel.ds";
const current = /*use*/value;
```

```ds:main.ds[1]
import { value } from "./beta.ds";
const current = /*use*/value;
```

```lsp definition use [0]
alpha_def
```

```lsp definition use [1]
beta_def
```

### Follow a direct import from alpha to gamma through two edits
Definition should keep tracking the active import target across repeated rewrites.

```ds:alpha.ds
export [|const /*alpha_def*/value = 1;|]
```

```ds:beta.ds
export [|const /*beta_def*/value = 2;|]
```

```ds:gamma.ds
export [|const /*gamma_def*/value = 3;|]
```

```ds:main.ds
import { value } from "./alpha.ds";
const current = /*use*/value;
```

```ds:main.ds[1]
import { value } from "./beta.ds";
const current = /*use*/value;
```

```ds:main.ds[2]
import { value } from "./gamma.ds";
const current = /*use*/value;
```

```lsp definition use [0]
alpha_def
```

```lsp definition use [1]
beta_def
```

```lsp definition use [2]
gamma_def
```

### Preserve definition while an unrelated dependency churns
Definition should stay stable when another imported file changes but the active symbol does not.

```ds:alpha.ds
export [|const /*def*/value = 1;|]
```

```ds:helper.ds
export const helper = 1;
```

```ds:main.ds
import { value } from "./alpha.ds";
import { helper } from "./helper.ds";

const total = /*use*/value + helper;
```

```ds:helper.ds[1]
export const helper = 2;

export const second_helper = 3;
```

```ds:main.ds[1]
import { value } from "./alpha.ds";
import { helper } from "./helper.ds";

const total = /*use*/value + helper + second_helper;
```

```lsp definition use [0]
def
```

```lsp definition use [1]
def
```

## Multi-Consumer Retargeting

### Retarget two consumers across three export rewrites
Definitions should keep following the active export while two consumers move together across three files.

```ds:alpha.ds
export [|const /*alpha_def*/value = 1;|]
```

```ds:beta.ds
export [|const /*beta_def*/value = 2;|]
```

```ds:gamma.ds
export [|const /*gamma_def*/value = 3;|]
```

```ds:a.ds
import { value } from "./alpha.ds";
const current = /*use_a*/value;
```

```ds:b.ds
import { value } from "./alpha.ds";
const current = /*use_b*/value;
```

```ds:a.ds[1]
import { value } from "./beta.ds";
const current = /*use_a*/value;
```

```ds:b.ds[1]
import { value } from "./beta.ds";
const current = /*use_b*/value;
```

```ds:a.ds[2]
import { value } from "./gamma.ds";
const current = /*use_a*/value;
```

```ds:b.ds[2]
import { value } from "./gamma.ds";
const current = /*use_b*/value;
```

```lsp definition use_a [0]
alpha_def
```

```lsp definition use_b [0]
alpha_def
```

```lsp definition use_a [1]
beta_def
```

```lsp definition use_b [1]
beta_def
```

```lsp definition use_a [2]
gamma_def
```

```lsp definition use_b [2]
gamma_def
```

### Preserve definition while sibling consumers churn independently
Definition should stay pinned to the same export while neighboring consumers rewrite around it.

```ds:lib.ds
export [|const /*def*/value = 1;|]

export const other = 2;
```

```ds:a.ds
import { value } from "./lib.ds";
const current = /*use*/value;
```

```ds:b.ds
import { other } from "./lib.ds";
const current = other;
```

```ds:c.ds
import { other } from "./lib.ds";
const current = other;
```

```ds:b.ds[1]
import { value } from "./lib.ds";
const current = value;
```

```ds:c.ds[1]
import { other } from "./lib.ds";
const current = other;
```

```ds:c.ds[2]
import { other } from "./lib.ds";
import { value } from "./lib.ds";

const current = other + value;
```

```lsp definition use [0]
def
```

```lsp definition use [1]
def
```

```lsp definition use [2]
def
```
