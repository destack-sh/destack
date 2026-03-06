# Commands

## Workspace Commands

### Reindex keeps navigation responsive

Executing the reindex command should not break later definition requests.

```ds:lib.ds
export function /*def*/ping(): void {}
```

```ds:main.ds
import { ping } from "./lib.ds";

/*use*/ping();
```

```lsp execute_command destack.reindex
```

### Rescan keeps navigation responsive

Executing the rescan command should not break later definition requests.

```ds:lib.ds
export function /*def*/pong(): void {}
```

```ds:main.ds
import { pong } from "./lib.ds";

/*use*/pong();
```

```lsp execute_command destack.rescan
```

### Clear cache keeps navigation responsive

Executing the clear-cache command should not break later definition requests.

```ds:lib.ds
export function /*def*/buzz(): void {}
```

```ds:main.ds
import { buzz } from "./lib.ds";

/*use*/buzz();
```

```lsp execute_command destack.clearCache
```
