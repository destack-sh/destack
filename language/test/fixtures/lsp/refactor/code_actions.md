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
Organize Imports|source_organize_imports|false
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

