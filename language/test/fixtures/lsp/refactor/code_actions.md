# Code Actions

## Eager Actions

### Organize imports immediately

Code actions should return organize-imports edits eagerly when the action already carries its edit.

```ds:main.ds
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + zed;
```

```ds:alpha.ds
export const alpha = 1;
```

```ds:zeta.ds
export const zed = 2;
```

```lsp code_action_result
import { alpha } from "./alpha.ds";
import { zed } from "./zeta.ds";

const value = alpha + zed;
```

```lsp code_action
Organize Imports|source_organize_imports|false|true
```

## Lazy Resolve

### Resolve organize imports on demand

Code-action resolve should materialize the organize-imports edit when the action is resolved lazily.

```ds:main.ds
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + zed;
```

```ds:alpha.ds
export const alpha = 1;
```

```ds:zeta.ds
export const zed = 2;
```

```lsp code_action_resolve_support
true
```

```lsp code_action_result
import { alpha } from "./alpha.ds";
import { zed } from "./zeta.ds";

const value = alpha + zed;
```

```lsp code_action
Organize Imports|source_organize_imports|false|false
```

## Cross-File Churn

### Keep organize imports available after adding a third import
Organize imports should remain available after a new unsorted import appears in the overlay.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:zeta.ds
export const zed = 3;
```

```ds:alpha.ds[1]
export const alpha = 1;
```

```ds:beta.ds[1]
export const beta = 2;
```

```ds:zeta.ds[1]
export const zed = 3;
```

```ds:main.ds
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + zed;
```

```ds:main.ds[1]
/*action*/import { zed } from "./zeta.ds";
import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + beta + zed;
```

```lsp code_action action [0]
Organize Imports|source_organize_imports|false|true
```

```lsp code_action action [1]
Organize Imports|source_organize_imports|false|true
```

### Keep organize imports available after removing one import use
Organize imports should remain available after an imported binding disappears from the overlay.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:alpha.ds[1]
export const alpha = 1;
```

```ds:beta.ds[1]
export const beta = 2;
```

```ds:main.ds
/*action*/import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + beta;
```

```ds:main.ds[1]
/*action*/import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const total = alpha;
```

```lsp code_action action [0]
Organize Imports|source_organize_imports|false|true
```

```lsp code_action action [1]
Organize Imports|source_organize_imports|false|true
```

### Keep organize imports available after a reindex command
Organize imports should remain available after a command churns the workspace state.

```ds:alpha.ds
export const alpha = 1;
```

```ds:zeta.ds
export const zed = 2;
```

```ds:main.ds
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + zed;
```

```lsp execute_command destack.reindex [0]
```

```lsp code_action action [0]
Organize Imports|source_organize_imports|false|true
```

### Keep organize imports available after a save and close cycle
Organize imports should remain available after saving and reopening the same overlay state.

```ds:alpha.ds
export const alpha = 1;
```

```ds:zeta.ds
export const zed = 2;
```

```ds:alpha.ds[1]
export const alpha = 1;
```

```ds:zeta.ds[1]
export const zed = 2;
```

```ds:main.ds
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + zed;
```

```ds:main.ds[1]
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + zed;
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
/*action*/import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const total = alpha + zed;
```

```lsp code_action action [1]
Organize Imports|source_organize_imports|false|true
```

## Code Action Churn

### Organize imports after two library additions
Code actions should stay available after two imported libraries are added across steps.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:zeta.ds
export const zed = 3;
```

```ds:main.ds
/*action*/import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + beta;
```

```ds:main.ds[1]
/*action*/import { zed } from "./zeta.ds";
import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + beta + zed;
```

```lsp code_action action [0]
Organize Imports|source_organize_imports|false|true
```

```lsp code_action action [1]
Organize Imports|source_organize_imports|false|true
```

### Organize imports survives a reindex after library churn
Code actions should remain exact after a library rewrite and an explicit reindex.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:main.ds
/*action*/import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + beta;
```

```ds:beta.ds[1]
export const beta = 3;

export const bonus = 4;
```

```ds:main.ds[1]
/*action*/import { beta } from "./beta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + beta + bonus;
```

```lsp execute_command destack.reindex [1]
```

```lsp code_action action [1]
Organize Imports|source_organize_imports|false|true
```
